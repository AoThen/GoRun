
//! GoRun 可执行入口。
//!
//! 所有实现都在 lib（`GoRun` crate）内，这里只做模块转导出 + `main`。
//! 这样集成测试（`tests/`）测的就是真实运行的那一份代码，
//! 避免 bin/lib 各自 `include!` Slint 生成代码导致 MainWindow 变成两个不同类型。
//! rustc 1.100 起用 crate 属性指定 Windows GUI 子系统（旧 `-C windows-subsystem` 已移除，
//! Cargo.toml 的 `windows_subsystem` 键因此失效并报 unused）。无控制台窗口。

#![windows_subsystem = "windows"]

pub use GoRun::{
    app_state, clipboard, config, drop_handler, fileops, geometry, handlers, hotkey, icon_cache,
    item_manager, localization, logger, model, platform, runner, storage, tray, tray_ui, ui_sync,
    util, CategoryModel, EditDialog, ItemModel, MainWindow,
};

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use GoRun::app_state::AppState;
use GoRun::item_manager::ItemManager;
use GoRun::model::{AppConfig, Category, ViewType};
use GoRun::storage::Storage;

fn main() {
    let debug_to_console = std::env::args_os().any(|arg| arg == "--debug");
    if let Err(e) = logger::init(debug_to_console) {
        eprintln!("Warning: Failed to initialize logger: {}", e);
    }
    logger::set_panic_hook();
    log::info!("GoRun starting...");

    // 便携统一（R1）：旧 AppData 数据迁移到当前基准目录（只复制不删源）
    config::migrate_legacy();

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{GetLastError, HANDLE, ERROR_ALREADY_EXISTS};
        use windows::Win32::System::Threading::CreateMutexW;
        let handle = unsafe {
            CreateMutexW(None, false, windows::core::w!("Local\\GoRun_SingleInstance"))
        }
        .unwrap_or(HANDLE::default());
        if handle.is_invalid() {
            log::error!("Failed to create single instance mutex");
            return;
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            log::info!("Another GoRun instance is already running, exiting");
            return;
        }
        // HANDLE 是 Copy 且无 Drop，无需 forget；保持变量存活即可，进程退出时内核释放。
        let _ = handle;
    }

    let config_path = config::config_path();
    let first_run = !std::path::Path::new(&config_path).exists();
    let storage = Storage::new(
        config_path
            .to_str()
            .unwrap_or("config.json")
            .to_string(),
    );

    // P2-05：区分"文件不存在"与"文件损坏"。损坏且无可用备份时隔离坏文件留证，
    // 避免后续保存覆盖坏数据/污染备份链；有可用备份时 load_from_backup 已自愈。
    let loaded = storage.load();
    let restored = if loaded.is_none() {
        storage.load_from_backup()
    } else {
        None
    };
    let corrupt_no_backup =
        !first_run && loaded.is_none() && restored.is_none();
    let mut app_config = loaded.or(restored).unwrap_or_default();
    // P2-3：记录是否自动创建了默认分类，供后续用当前语言重命名（外部隔离分支会置 true）
    let mut created_default_cat = app_config.data.categories.is_empty();
    if created_default_cat {
        log::warn!("No categories found, created default category");
        let default_cat = Category {
            id: model::generate_id("cat"),
            name: "Default".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        };
        app_config.data.categories.push(default_cat);
    }

    // P2-05：损坏且无可用备份 → 隔离坏文件（留证），使用默认配置继续运行
    let corrupt_quarantined: Option<String> = if corrupt_no_backup {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let corrupt_path = format!("{}.corrupt.{}", config_path.display(), ts);
        if std::fs::rename(&config_path, &corrupt_path).is_ok() {
            log::warn!("Corrupt config quarantined to: {}", corrupt_path);
            Some(corrupt_path)
        } else {
            log::error!(
                "Failed to quarantine corrupt config {}; will run with default in memory",
                config_path.display()
            );
            None
        }
    } else {
        None
    };

    log::info!("Config loaded from: {:?}", config::config_path());
    // 信任边界（P0-2）：非首启、含条目且未通过完整性校验 → 视为外部/未信任配置。
    // 校验 = generated_by 标记值精确匹配 + 内容完整性值（config_integrity）匹配，
    // 仅伪造标记无法通过。默认拒绝执行，将原文件备份为 config.json.external，
    // 重建默认配置加载，避免任意外部 target 被双击/run_all 批量执行。
    let mut external_blocked = false;
    let mut external_backup_path: Option<String> = None;
    let external_config =
        !first_run && !app_config.data.items.is_empty() && !storage::verify_integrity(&app_config);
    if external_config {
        log::warn!(
            "External/untrusted config detected, refusing to execute: {:?}",
            config_path
        );
        let external_backup = format!("{}.external", config_path.display());
        if std::fs::rename(&config_path, &external_backup).is_ok() {
            log::info!("External config quarantined to: {}", external_backup);
            external_backup_path = Some(external_backup);
        } else {
            log::error!("Failed to backup external config, will fallback to default in memory only");
        }
        app_config = AppConfig::default();
        app_config.data.categories.push(Category {
            id: model::generate_id("cat"),
            name: "Default".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        });
        // 重建的默认分类同样需要本地化名称（P2-3）
        created_default_cat = true;
        external_blocked = true;
    }

    let hotkey_str = app_config.config.global_hotkey.clone();
    // 语言/跟随鼠标初始状态（决策⑤：默认 zh-CN；UI 文本切换延后，仅持久化）
    let lang_code = app_config
        .extra_config
        .get("language")
        .cloned()
        .unwrap_or_else(|| "zh-CN".to_string());
    let follow_mouse = app_config
        .extra_config
        .get("follow_mouse")
        .map(|v| v == "1")
        .unwrap_or(false);
    // P2-3：提前创建 loc，以便用当前语言重命名首启默认分类
    let loc = Rc::new(RefCell::new(localization::Localization::new(&lang_code)));
    if created_default_cat {
        let default_name = loc.borrow().tr("Default_Category").to_string();
        if let Some(first) = app_config.data.categories.first_mut() {
            first.name = default_name;
        }
    }
    // P2-3：首启或外部配置隔离后重建的默认配置，需以本地化名称落盘；
    // 统一补 generated_by 标记，避免隔离后的默认配置被下次启动再次误判为外部配置；
    // 外部隔离仅在原文件成功隔离（external_backup_path 非空）后才落盘，避免覆盖原配置。
    if first_run || external_blocked {
        app_config
            .extra_config
            .insert("generated_by".to_string(), "gorun-rs".to_string());
        if first_run {
            if storage.save(&app_config) {
                log::info!("First run: config file created");
            }
        } else if external_backup_path.is_some() {
            if storage.save(&app_config) {
                log::info!("Default config recreated after external config quarantine");
            }
        }
    }
    let manager = ItemManager::new(app_config);
    let icon_cache = icon_cache::IconCache::new();
    let (tray_icon, rx) = tray::TrayIcon::create(0, "GoRun");
    tray::set_ui_state(follow_mouse, &lang_code);
    // 托盘菜单文本由 apply_localized_texts（窗口初始化后、事件循环前）统一设置，
    // 避免重复设置；见 main.rs 的 tray_ui::apply_localized_texts 调用。
    let tray_ui: std::sync::Arc<std::sync::Mutex<Vec<tray_ui::TrayUiAction>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let _tray = tray_icon;

    let current_category_id = manager
        .categories()
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let state = Rc::new(RefCell::new(AppState {
        manager,
        storage,
        icon_cache,
        current_category_id: current_category_id.clone(),
        search_query: String::new(),
        edit_dialog: None,
        loc: loc.clone(),
        save_fail_notified: false,
        pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        drop_handler: None,
        geometry_dirty: false,
        last_geometry_change: None,
    }));

    let ui = match MainWindow::new() {
        Ok(ui) => ui,
        Err(e) => {
            log::error!("Failed to create main window: {}", e);
            return;
        }
    };
    log::info!("MainWindow created successfully");

    // 窗口几何恢复（C++ 存物理像素，直接以物理单位设置）；
    // 仅当配置合法（最小尺寸 + 在屏内）时应用（P2 #12）
    {
        let cfg = state.borrow().manager.config().config.clone();
        if cfg.window_width >= 400
            && cfg.window_height >= 300
            && util::window_on_screen(
                cfg.window_x,
                cfg.window_y,
                cfg.window_width,
                cfg.window_height,
            )
        {
            use slint::{PhysicalPosition, PhysicalSize};
            ui.window().set_position(PhysicalPosition::new(cfg.window_x, cfg.window_y));
            ui.window().set_size(PhysicalSize::new(
                cfg.window_width as u32,
                cfg.window_height as u32,
            ));
        }
    }

    {
        let state_close = state.clone();
        let weak_close = ui.as_weak();
        ui.window().on_close_requested(move || {
            // 关闭=隐藏，隐藏前强制落盘几何（去抖不适用于退出路径）
            if let Some(ui) = weak_close.upgrade() {
                geometry::flush_window_geometry(&ui, &state_close);
            }
            slint::CloseRequestResponse::HideWindow
        });
    }

    tray_ui::start_tray_handler(rx, ui.as_weak(), tray_ui.clone());

    // 周期保存窗口几何 + 消费托盘 UI 动作（500ms，对齐 C++ App::Flush）：
    // 保证托盘退出等非 close 路径也能在窗口销毁前落盘；Timer 回调在事件循环
    // 线程执行（Rc 可捕获，无需 Send）。
    let _geo_timer = {
        let timer = slint::Timer::default();
        let weak_geo = ui.as_weak();
        let state_geo = state.clone();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(500),
            move || {
                if let Some(ui) = weak_geo.upgrade() {
                    geometry::save_window_geometry(&ui, &state_geo);
                    // P0：把后台提取到的图标缓存路径回写模型并落盘，
                    // 否则重启后自动提取的图标全部丢失。
                    let pending_icons = state_geo.borrow().pending_icon_paths.clone();
                    ui_sync::persist_cached_icons(&pending_icons, &state_geo);
                    // P1-5：几何去抖窗口到期才真正写盘
                    state_geo.borrow_mut().flush_geometry_if_due(false);
                    // P2-05：保存失败一次性状态栏提示（避免静默丢数据）；显示后复位，
                    // 下次新的保存失败会再次置位并提示，不长期覆盖其他状态信息。
                    let notify_fail = state_geo.borrow().save_fail_notified;
                    if notify_fail {
                        // 先克隆 loc 再取文案，避免 Ref 临时值在尾表达式处被借用检查器
                        // 认为与外层 borrow 重叠（E0597）
                        let loc_ref = state_geo.borrow().loc.clone();
                        let msg = loc_ref.borrow().tr("Status_SaveFailed").to_string();
                        ui.set_status_text(msg.into());
                        state_geo.borrow_mut().save_fail_notified = false;
                    }
                    let actions: Vec<tray_ui::TrayUiAction> = tray_ui
                        .lock()
                        .map(|mut q| q.drain(..).collect())
                        .unwrap_or_default();
                    for act in actions {
                        tray_ui::handle_tray_ui_action(act, &ui, &state_geo);
                    }
                }
            },
        );
        timer
    };

    let hotkey_mgr = Rc::new(RefCell::new(hotkey::HotkeyManager::new()));
    let (hk_tx, hk_rx) = std::sync::mpsc::channel();
    let hk_tx_shared: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>> =
        Rc::new(RefCell::new(Some(hk_tx.clone())));
    {
        if hotkey_mgr.borrow_mut().register_with_notifier(&hotkey_str, hk_tx) {
            log::info!("Global hotkey registered: {}", hotkey_str);
        } else {
            log::warn!("Failed to register global hotkey: {}", hotkey_str);
        }
        let ui_hk = ui.as_weak();
        std::thread::spawn(move || {
            while hk_rx.recv().is_ok() {
                let weak = ui_hk.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        if ui.window().is_visible() {
                            let _ = ui.hide();
                        } else {
                            let _ = ui.show();
                        }
                    }
                });
            }
        });
    }
    let _hotkey_mgr = hotkey_mgr.clone();

    let categories: Vec<CategoryModel> = state
        .borrow()
        .manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();
    let category_model = Rc::new(slint::VecModel::from(categories));
    ui.set_categories(category_model.into());

    // 主题持久化：复用 C++ 版 config 的 "theme" 键（"0"=Light, "1"=Dark），缺失默认 Light
    let is_dark = state
        .borrow()
        .manager
        .config()
        .extra_config
        .get("theme")
        .map(|v| v.as_str() == "1")
        .unwrap_or(false);
    ui.set_is_dark_theme(is_dark);

    // 菜单栏/空状态/确认按钮/侧栏文本（本地化）；主题键按当前主题选择，避免文案错配
    tray_ui::apply_localized_texts(&ui, &loc, is_dark);

    // 初始视图模式按当前分类的 view_type（默认 Icon）
    let initial_is_icon = state
        .borrow()
        .manager
        .categories()
        .first()
        .map(|c| matches!(c.view_type, ViewType::Icon))
        .unwrap_or(true);
    ui.set_is_icon_view(initial_is_icon);

    ui_sync::refresh_items_ui(&ui, &state.borrow(), &current_category_id, "");

    if external_blocked {
        let msg = format!(
            "{}: {}",
            loc.borrow().tr("Tip_ExternalBackup"),
            config_path.display()
        );
        ui.set_status_text(msg.into());
    }
    if let Some(corrupt_path) = &corrupt_quarantined {
        let msg = loc
            .borrow()
            .tr("Status_CorruptQuarantined")
            .replace("{0}", corrupt_path);
        ui.set_status_text(msg.into());
    }

    handlers::wire_handler(
        &ui,
        state.clone(),
        &loc,
        hotkey_mgr.clone(),
        hk_tx_shared,
        external_backup_path,
    );

    // P0：拖放守卫必须存入 AppState 存活到进程结束；
    // 只把消息接收端交给消费线程。
    if let Some((guard, rx)) = drop_handler::setup_drop_handler(&ui) {
        drop_handler::spawn_drop_consumer(ui.as_weak(), rx);
        state.borrow_mut().drop_handler = Some(guard);
    }

    log::info!("Entering main event loop");
    if let Err(e) = ui.run() {
        log::error!("Main event loop error: {}", e);
    }
    log::info!("Main event loop ended");

    let mut state = state.borrow_mut();
    // 退出前强制落盘几何（去抖窗口可能还没到）
    state.flush_geometry_if_due(true);
    state.save_if_modified();
    // 清空拖放子类化（窗口仍在时按序移除，避免回调读到已释放 refdata）
    state.drop_handler = None;
}

