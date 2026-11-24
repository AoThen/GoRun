//! GoRun 可执行入口。
//!
//! 所有实现都在 lib（`GoRun` crate）内，这里只做模块转导出 + `main`。
//! 这样集成测试（`tests/`）测的就是真实运行的那一份代码，
//! 避免 bin/lib 各自 `include!` Slint 生成代码导致 MainWindow 变成两个不同类型。
//! rustc 1.100 起用 crate 属性指定 Windows GUI 子系统（旧 `-C windows-subsystem` 已移除，
//! Cargo.toml 的 `windows_subsystem` 键因此失效并报 unused）。无控制台窗口。

#![windows_subsystem = "windows"]

pub use GoRun::{
    app_state, background, clipboard, config, drop_handler, fileops, geometry, handlers, hotkey,
    icon_cache, item_manager, localization, logger, model, platform, runner, storage, tray,
    tray_ui, ui_sync, util, CategoryModel, EditDialog, ItemModel, MainWindow,
};

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use GoRun::app_state::AppState;
use GoRun::item_manager::ItemManager;
use GoRun::model::ViewType;
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
        use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
        use windows::Win32::System::Threading::CreateMutexW;
        let handle = unsafe {
            CreateMutexW(
                None,
                false,
                windows::core::w!("Local\\GoRun_SingleInstance"),
            )
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
    let storage = Storage::new(config_path.to_str().unwrap_or("config.json").to_string());

    // P2-05：区分"文件不存在"与"文件损坏"。损坏且无可用备份时隔离坏文件留证，
    // 避免后续保存覆盖坏数据/污染备份链；有可用备份时 load_from_backup 已自愈。
    let loaded = storage.load();
    let restored = if loaded.is_none() {
        storage.load_from_backup()
    } else {
        None
    };
    let corrupt_no_backup = !first_run && loaded.is_none() && restored.is_none();
    let mut app_config = loaded.or(restored).unwrap_or_default();
    // P2-3：记录是否自动创建了默认分类，供后续用当前语言重命名
    let created_default_cat = app_config.data.categories.is_empty();
    if created_default_cat {
        log::warn!("No categories found, created default category");
        model::ensure_default_category(&mut app_config.data, "Default");
    }
    // P2-8：孤儿条目（指向不存在分类，如手改配置/旧版数据）归入首个分类；
    // 此前孤儿只在跨分类搜索中可见，分类视图不可见不可管理
    model::attach_orphan_items(&mut app_config.data);

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

    // L-5：加载时迁移了旧默认热键 → 顺手落盘一次，避免每次启动重复迁移
    if storage.migrated_hotkey() && storage.save(&app_config) {
        log::info!("Migrated legacy default hotkey persisted");
    }

    // M-3/L-2：启动时便携目录卫生检查——清理原子写残留 tmp 与孤儿图标缓存
    {
        Storage::cleanup_stale_tmp(&config_path);
        let live_icons: std::collections::HashSet<String> = app_config
            .data
            .items
            .iter()
            .filter_map(|it| {
                if it.cached_icon.is_empty() {
                    None
                } else {
                    std::path::Path::new(&it.cached_icon)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                }
            })
            .collect();
        icon_cache::cleanup_and_gc(&config::icons_dir(), &live_icons);
    }

    // 日志档位（设置项）：未设置默认标准（Info，隐私收敛；Debug 记录搜索词/路径需显式开启），可选调试 Debug/最少 Warn，运行时即时生效
    let log_level = app_config
        .extra_config
        .get("log_level")
        .cloned()
        .unwrap_or_default();
    logger::apply_level(&log_level);

    let hotkey_str = app_config.config.global_hotkey.clone();
    // 仅中文（v2 决策）：固定简体中文，不再支持多语言切换
    let lang_code = "zh-CN";
    let follow_mouse = app_config.extra_get_bool("follow_mouse");
    // P3/P5 后台低驻留：窗口隐藏时暂停 500ms 周期 Timer 的非必要工作（仅保留托盘动作消费）
    let low_resident = app_config.extra_get_bool("low_resident");
    // P3-11 F7 自动隐藏初值（须先于 ItemManager::new 取走 app_config 前读好）
    let auto_hide = app_config.extra_get_bool("auto_hide");
    // P2-3：提前创建 loc，以便用当前语言重命名首启默认分类
    let loc = Rc::new(RefCell::new(localization::Localization::new(lang_code)));
    if created_default_cat {
        let default_name = loc.borrow().tr("Default_Category").to_string();
        if let Some(first) = app_config.data.categories.first_mut() {
            first.name = default_name;
        }
    }
    // P2-3：首启创建的默认配置需以本地化名称落盘（默认分类名按当前语言重命名后写入）
    if first_run && storage.save(&app_config) {
        log::info!("First run: config file created");
    }
    let manager = ItemManager::new(app_config);
    let icon_cache = icon_cache::IconCache::new();
    let (tray_icon, rx) = tray::TrayIcon::create(0, "GoRun");
    tray::set_ui_state(follow_mouse, low_resident);
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

    // F8/4.1：条目级全局热键管理器 + 触发通道（消费线程在事件循环前启动）
    let item_hotkey_mgr = Rc::new(RefCell::new(hotkey::ItemHotkeyManager::new()));
    let (ihk_tx, ihk_rx) = std::sync::mpsc::channel::<String>();
    // P3/P5 低驻留：可运行时经托盘切换（Cell 供 Timer 闭包与托盘动作共享）
    let low_resident_cell = Rc::new(std::cell::Cell::new(low_resident));

    let state = Rc::new(RefCell::new(AppState {
        manager,
        storage,
        icon_cache,
        current_category_id: current_category_id.clone(),
        search_query: String::new(),
        edit_dialog: None,
        loc: loc.clone(),
        save_fail_notified: false,
        pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
        icon_extract_attempted: std::sync::Mutex::new(std::collections::HashSet::new()),
        drop_handler: None,
        geometry_dirty: false,
        last_geometry_change: None,
        soft_dirty: false,
        last_soft_change: None,
        pending_run_results: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        item_hotkey_mgr: item_hotkey_mgr.clone(),
        item_hotkey_tx: ihk_tx,
        low_resident: low_resident_cell.clone(),
        // P3-11：F7 自动隐藏运行时标志（初值取配置；设置页开关可同时更新）
        auto_hide_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
            auto_hide,
        )),
    }));

    let ui = match MainWindow::new() {
        Ok(ui) => ui,
        Err(e) => {
            log::error!("Failed to create main window: {}", e);
            return;
        }
    };
    log::info!("MainWindow created successfully");

    // 窗口几何恢复（slint position/size 返回物理像素，直接以物理单位设置）；
    // 仅当配置合法（最小尺寸 + 在屏内）时应用（P2 #12）。
    // 创建前 backend 按 scale=1 把尺寸存为逻辑值，创建时再乘真实 DPI：若直接设
    // Physical 会按 DPI 倍数漂移（旧缺陷），且首帧先按默认逻辑尺寸绘制、恢复后再
    // 缩到保存尺寸，产生"先大后小"闪动。因此先查目标显示器缩放，把物理尺寸换算
    // 成逻辑尺寸在创建前预设，首帧即命中目标大小；when_window_ready 的延迟恢复
    // 保留为兜底校准（预设准确时为同尺寸无操作）。
    let mut pending_window_size: Option<(u32, u32)> = None;
    {
        let cfg = state.borrow().manager.config().config.clone();
        if cfg.window_width >= 360
            && cfg.window_height >= 240
            && util::window_on_screen(
                cfg.window_x,
                cfg.window_y,
                cfg.window_width,
                cfg.window_height,
            )
        {
            use slint::PhysicalPosition;
            ui.window()
                .set_position(PhysicalPosition::new(cfg.window_x, cfg.window_y));
            // 历史版本 DPI 漂移可能留下超过屏幕的超大尺寸：等比缩回虚拟屏 80% 以内
            let (w, h) = util::clamp_size_to_virtual_screen(cfg.window_width, cfg.window_height);
            // 创建前以"物理尺寸 ÷ 所在屏缩放"的逻辑尺寸预设，首帧即为目标尺寸；
            // scale 未知时不预设，退回 when_window_ready 的延迟恢复（原行为）。
            // 注：winit 按创建位置的 DPI 物化 Logical 尺寸，保存位置跨异 DPI 副屏时
            // 首帧可能仍有偏差，由 when_window_ready 的兜底恢复校准到目标尺寸。
            match util::monitor_scale_at(cfg.window_x, cfg.window_y) {
                Some(scale) => {
                    log::info!(
                        "Pre-setting window to {}x{} physical (scale {:.2})",
                        w,
                        h,
                        scale
                    );
                    ui.window()
                        .set_size(slint::LogicalSize::new(w as f32 / scale, h as f32 / scale));
                }
                None => {
                    log::info!("Monitor scale unavailable; size restore deferred to window-ready")
                }
            }
            pending_window_size = Some((w as u32, h as u32));
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

    // 周期保存窗口几何 + 消费托盘 UI 动作（500ms 周期）：
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
                    // P3/P5 低驻留：窗口隐藏时跳过几何/图标/保存（省 CPU），
                    // 仅保留托盘动作消费（托盘 FollowMouse 依赖本 Timer）。
                    // 运行时读 state.low_resident（托盘开关可实时切换）。
                    let resident_active =
                        state_geo.borrow().low_resident.get() && !ui.window().is_visible();
                    if !resident_active {
                        geometry::save_window_geometry(&ui, &state_geo);
                        // P0：把后台提取到的图标缓存路径回写模型并落盘，
                        // 否则重启后自动提取的图标全部丢失。
                        // 有回写时顺带整表重建条目模型（见 persist_cached_icons 文档）。
                        let pending_icons = state_geo.borrow().pending_icon_paths.clone();
                        ui_sync::persist_cached_icons(&ui, &pending_icons, &state_geo);
                        // P1-5：几何去抖窗口到期才真正写盘
                        state_geo.borrow_mut().flush_geometry_if_due(false);
                        // M-5/M-6：消费异步启动结果（成功回写运行计数并标脏软保存；
                        // 失败弹本地化错误提示），软保存去抖窗口到期才真正写盘。
                        // 窗口隐藏时只回写计数不弹提示（重新显示后不再补弹旧错误）。
                        let ui_visible = ui.window().is_visible();
                        for r in ui_sync::drain_run_results(&state_geo) {
                            if !r.success && ui_visible {
                                let loc = state_geo.borrow().loc.clone();
                                let err_text = ui_sync::run_error_text(&loc, &r.error);
                                let msg = util::tr_fill(
                                    &loc,
                                    "Status_RunFail",
                                    &format!("{}: {}", r.item_name, err_text),
                                );
                                ui_sync::show_toast(&ui, msg, true);
                            }
                        }
                        state_geo.borrow_mut().flush_soft_if_due(false);
                        // P2-05：保存失败一次性状态栏提示（避免静默丢数据）；显示后复位，
                        // 下次新的保存失败会再次置位并提示，不长期覆盖其他状态信息。
                        let notify_fail = state_geo.borrow().save_fail_notified;
                        if notify_fail {
                            // 先克隆 loc 再取文案，避免 Ref 临时值在尾表达式处被借用检查器
                            // 认为与外层 borrow 重叠（E0597）
                            let loc_ref = state_geo.borrow().loc.clone();
                            let msg = loc_ref.borrow().tr("Status_SaveFailed").to_string();
                            ui_sync::show_toast(&ui, msg, true);
                            state_geo.borrow_mut().save_fail_notified = false;
                        }
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
        if hotkey_mgr
            .borrow_mut()
            .register_with_notifier(&hotkey_str, hk_tx)
        {
            log::info!("Global hotkey registered: {}", hotkey_str);
        } else {
            log::warn!("Failed to register global hotkey: {}", hotkey_str);
            // 热键常被输入法/截图等常驻软件占用：明确提示更换，而非静默失效
            let msg = loc.borrow().tr("Tip_HotkeyBusy").to_string();
            ui_sync::show_toast(&ui, msg, true);
        }
        let ui_hk = ui.as_weak();
        std::thread::spawn(move || {
            while hk_rx.recv().is_ok() {
                let weak = ui_hk.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        // 可见且未最小化 → 隐藏；否则（隐藏或最小化）唤醒并置前。
                        // 最小化时 is_visible() 仍为 true，需排除，否则热键会把
                        // 用户想唤起的窗口藏起来。
                        if ui.window().is_visible() && !ui.window().is_minimized() {
                            let _ = ui.hide();
                        } else {
                            let _ = ui.show();
                            geometry::bring_to_front(&ui);
                        }
                    }
                });
            }
        });
    }
    let _hotkey_mgr = hotkey_mgr.clone();

    // F8/4.1：条目级全局热键——事件循环内轮询 item_id 并运行；首次注册所有非空条目热键
    // （不用独立线程+invoke_from_event_loop：run_item_by_id 需 Rc<AppState>，非 Send；
    //  事件循环 Timer 回调在 UI 线程执行，Rc 可捕获，与 geo_timer 同模式。）
    let _ihk_timer = {
        let timer = slint::Timer::default();
        let state_ihk = state.clone();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                while let Ok(item_id) = ihk_rx.try_recv() {
                    crate::ui_sync::run_item_by_id(&state_ihk, &item_id);
                }
            },
        );
        timer
    };
    ui_sync::sync_item_hotkeys(&state.borrow());

    let st_cats = state.borrow();
    let categories: Vec<CategoryModel> = st_cats
        .manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
            count: st_cats.manager.items(&c.id).len() as i32,
        })
        .collect();
    drop(st_cats);
    let category_model = Rc::new(slint::VecModel::from(categories));
    ui.set_categories(category_model.into());

    // 主题持久化："theme" 键 "0"=Light, "1"=Dark，缺失默认 Light
    let is_dark = state
        .borrow()
        .manager
        .config()
        .extra_config
        .get("theme")
        .map(|v| v.as_str() == "1")
        .unwrap_or(false);
    ui.set_is_dark_theme(is_dark);

    // 背景图恢复（设置页 V1）：路径失效/解码失败时降级纯色、清除失效路径并告警，不阻断启动
    {
        let (bg_path, dim_enabled, dim_percent) = {
            let st = state.borrow();
            let cfg = st.manager.config();
            (
                cfg.extra_config.get("background_image").cloned(),
                cfg.extra_config
                    .get("background_dim_enabled")
                    .map(|v| v == "1"),
                cfg.extra_config
                    .get("background_dim")
                    .and_then(|v| v.parse::<f32>().ok()),
            )
        };
        if let Some(path) = bg_path {
            match crate::background::decode(&path) {
                Some(decoded) => {
                    ui.set_bg_image(slint::Image::from_rgba8(decoded.buffer));
                    ui.set_has_bg_image(true);
                }
                None => {
                    log::warn!("Background restore failed, fallback to solid: {}", path);
                    let mut st = state.borrow_mut();
                    st.manager
                        .config_mut()
                        .extra_config
                        .remove("background_image");
                    st.save_if_modified();
                }
            }
        }
        if let Some(v) = dim_enabled {
            ui.set_bg_dim_enabled(v);
        }
        if let Some(p) = dim_percent {
            ui.set_bg_dim((p / 100.0).clamp(0.0, 1.0));
        }
    }

    // 菜单栏/空状态/确认按钮/侧栏文本（本地化）；主题键按当前主题选择，避免文案错配
    tray_ui::apply_localized_texts(&ui, &loc, is_dark);

    // 初始视图模式按当前分类的 view_type（默认 List）
    let initial_is_icon = state
        .borrow()
        .manager
        .categories()
        .first()
        .map(|c| matches!(c.view_type, ViewType::Icon))
        .unwrap_or(false);
    ui.set_is_icon_view(initial_is_icon);

    ui_sync::refresh_items_ui(&ui, &state.borrow(), &current_category_id, "");

    if let Some(corrupt_path) = &corrupt_quarantined {
        let msg = loc
            .borrow()
            .tr("Status_CorruptQuarantined")
            .replace("{0}", corrupt_path);
        ui_sync::show_toast(&ui, msg, true);
    }

    handlers::wire_handler(&ui, state.clone(), &loc, hotkey_mgr.clone(), hk_tx_shared);

    // 窗口就绪后统一装配：尺寸恢复 + 拖放 + 标题栏图标。
    // winit 窗口在事件循环 Resumed/ensure_window 时才真正创建，在此之前
    // window_handle() 返回 Err（曾导致拖放被静默禁用、图标设置失效）。
    // P0：拖放守卫必须存入 AppState 存活到进程结束；消息接收端交给消费线程。
    // 回调经 when_window_ready 在事件循环（UI 线程）执行，可捕获 Rc<AppState>。
    {
        let weak_ui = ui.as_weak();
        let state_setup = state.clone();
        // P3-11：传共享标志（非启动时快照），设置开关切换即时生效
        let auto_hide_drop = state.borrow().auto_hide_flag.clone();
        geometry::when_window_ready(weak_ui, move |ui| {
            log::info!(
                "Window ready; actual scale factor: {:.2}",
                ui.window().scale_factor()
            );
            // 1) 恢复窗口尺寸：窗口已存在，Physical 1:1 精确生效，无 DPI 换算
            if let Some((w, h)) = pending_window_size {
                use slint::PhysicalSize;
                ui.window().set_size(PhysicalSize::new(w, h));
            }
            // 2) 拖放装配
            if let Some((guard, rx)) = drop_handler::setup_drop_handler(ui) {
                drop_handler::spawn_drop_consumer(ui.as_weak(), rx, auto_hide_drop);
                state_setup.borrow_mut().drop_handler = Some(guard);
            } else {
                log::warn!("Drop handler setup returned None; drag-and-drop disabled");
            }
            // 3) 标题栏图标（嵌入 app.ico，资源 ID 1，与托盘同源）
            if let Some(hwnd) = drop_handler::try_get_hwnd(ui) {
                platform::window_icon::apply_app_icon(hwnd);
            } else {
                log::warn!("Window handle unavailable; title bar icon not set");
            }
        });
    }

    log::info!("Entering main event loop");
    // until_quit：隐藏窗口（热键/ESC/X/拖出自动隐藏）不再触发 keepalive 归零退出进程，
    // 托盘常驻；仅显式 quit_event_loop（托盘/菜单"退出"）结束循环
    if let Err(e) = ui.show().and_then(|_| slint::run_event_loop_until_quit()) {
        log::error!("Main event loop error: {}", e);
    }
    log::info!("Main event loop ended");

    let mut state = state.borrow_mut();
    // 退出前强制落盘几何与软保存（去抖窗口可能还没到）
    state.flush_geometry_if_due(true);
    state.flush_soft_if_due(true);
    state.save_if_modified();
    // 清空拖放子类化（窗口仍在时按序移除，避免回调读到已释放 refdata）
    state.drop_handler = None;
}
