//! 事件处理器模块（P1-3 自 main.rs 拆出）
//!
//! 按职责分组：
//! - `dialogs`：编辑对话框辅助（分类新增按钮、外部配置静默恢复导入）
//! - `edit_dialog`：新建/编辑项目对话框的共享展示逻辑
//! - `wire_handler`：主窗口事件回调注册（D3 自 main.rs 迁入）

pub mod config_io;
pub mod dialogs;
pub mod edit_dialog;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::handlers::edit_dialog::{browse_texts, validate_hotkey_input, validate_item_input, BrowseKind};
use crate::localization::Localization;
use crate::model::{Category, ViewType};
use crate::{clipboard, fileops, geometry, hotkey, model, ui_sync, util, EditDialog, MainWindow};
use slint::ComponentHandle;

/// 应用背景图到 UI 并持久化路径。解码失败返回 false（调用方 Toast 提示，UI 保持现状）。
/// 菜单/弹层内容总高：项 28px、分隔线("-") 1px、上下 padding 4px×2——供挂载处限高滚动
fn menu_content_height(items: &[slint::SharedString]) -> f32 {
    items
        .iter()
        .map(|s| if s.as_str() == "-" { 1.0 } else { 28.0 })
        .sum::<f32>()
        + 8.0
}

fn apply_background(ui: &MainWindow, state: &Rc<RefCell<AppState>>, path: &str) -> bool {
    let Some(decoded) = crate::background::decode(path) else {
        log::warn!("Background decode failed: {}", path);
        return false;
    };
    ui.set_bg_image(slint::Image::from_rgba8(decoded.buffer));
    ui.set_has_bg_image(true);
    let mut st = state.borrow_mut();
    st.manager
        .config_mut()
        .extra_config
        .insert("background_image".into(), path.into());
    st.save_if_modified();
    true
}

pub fn wire_handler(
    ui: &MainWindow,
    state: Rc<RefCell<AppState>>,
    loc: &Rc<RefCell<Localization>>,
    hotkey_mgr: Rc<RefCell<hotkey::HotkeyManager>>,
    hk_tx_shared: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
) {
    let right_clicked_index: Rc<std::cell::Cell<Option<i32>>> = Rc::new(std::cell::Cell::new(None));
    let pending_delete: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    // 右键菜单来源：0=项目 1=背景 2=分类（单个 ContextMenu 复用，按来源分发）
    let menu_kind: Rc<std::cell::Cell<u8>> = Rc::new(std::cell::Cell::new(0));
    let right_clicked_category: Rc<std::cell::Cell<Option<i32>>> =
        Rc::new(std::cell::Cell::new(None));
    let pending_category_delete: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    let pending_category_rename: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    // 运行全部确认（背景右键菜单"运行全部"→确认弹窗→on_confirm_dialog_ok 执行）
    let pending_run_all: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    let pending_run_admin: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    // InputDialog 模式：0=无 1=分类重命名 2=快捷键设置
    let input_mode: Rc<std::cell::Cell<u8>> = Rc::new(std::cell::Cell::new(0));

    let state_file = state.clone();
    let weak_ui_file = ui.as_weak();
    ui.on_file_dropped(move |name, target| {
        log::info!(
            "File dropped: name={}, target={}",
            util::sanitize_for_log(&name),
            util::sanitize_for_log(&target)
        );
        let path = PathBuf::from(target.to_string());
        let mut state = state_file.borrow_mut();
        let cat_id = state.current_category_id.clone();
        let added_ids = state
            .manager
            .handle_drop(std::slice::from_ref(&path), &cat_id);
        state.save_if_modified();
        // P0：小批量拖入同步提取图标（对齐 C++ 的即时显示体验，拖入即见图标）；
        // 大批量仍走 refresh_items_ui 的异步提取队列兜底，避免阻塞 UI 线程。
        const SYNC_ICON_LIMIT: usize = 8;
        // 重复拖入被幂等跳过时，若已存在条目缺图标（历史记录提取失败/缓存丢失），
        // 同样同步补齐——用户视角"拖入就该有图标"，无论是否首次添加。
        let mut sync_ids = added_ids;
        if sync_ids.is_empty() {
            let drop_target = crate::item_manager::normalize_drop_target(&path);
            if let Some(existing) = state.manager.find_item_by_target(&cat_id, &drop_target) {
                if existing.icon_path.is_empty()
                    && state.icon_cache.get_icon_path(&existing).is_none()
                {
                    sync_ids.push(existing.id);
                }
            }
        }
        if !sync_ids.is_empty() && sync_ids.len() <= SYNC_ICON_LIMIT {
            for id in &sync_ids {
                let item = match state.manager.get_item(id) {
                    Some(it) => it.clone(),
                    None => continue,
                };
                if let Some(p) = state.icon_cache.refresh_icon(&item) {
                    state.manager.set_cached_icon(id, &p.to_string_lossy());
                }
            }
            state.save_if_modified();
        }
        let query = state.search_query.clone();
        drop(state);
        if let Some(ui) = weak_ui_file.upgrade() {
            ui_sync::refresh_items_ui(&ui, &state_file.borrow(), &cat_id, &query);
        }
    });

    let state_search = state.clone();
    let weak_ui_search = ui.as_weak();
    // 搜索防抖（P2 #3，对齐 C++ 300ms）：Timer 由闭包持有存活
    let search_timer = Rc::new(slint::Timer::default());
    ui.on_search_changed(move |text| {
        let text = text.to_string();
        log::debug!("Search changed: {}", text);
        let mut state = state_search.borrow_mut();
        state.search_query = text.clone();
        drop(state);
        let timer = search_timer.clone();
        let weak = weak_ui_search.clone();
        let state_cb = state_search.clone();
        timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(300),
            move || {
                if let Some(ui) = weak.upgrade() {
                    // P1-2：回调内重新读取当前分类。若在 300ms 窗口内切换了分类，
                    // 捕获的旧 cat_id 会把界面刷回旧分类内容。
                    let (cat_id, query) = {
                        let st = state_cb.borrow();
                        (st.current_category_id.clone(), st.search_query.clone())
                    };
                    if query != text {
                        // 期间又输入了新内容，交给更新的一次 Timer 处理
                        return;
                    }
                    ui_sync::refresh_items_ui(&ui, &state_cb.borrow(), &cat_id, &query);
                }
            },
        );
    });

    let state_cat = state.clone();
    let weak_ui_cat = ui.as_weak();
    ui.on_category_selected(move |index| {
        log::debug!("Category selected: index={}", index);
        let mut state = state_cat.borrow_mut();
        if let Some(cat) = state.manager.categories().get(index as usize) {
            let cat_id = cat.id.clone();
            let is_icon = matches!(cat.view_type, ViewType::Icon);
            state.current_category_id = cat_id.clone();
            let query = state.search_query.clone();
            drop(state);
            if let Some(ui) = weak_ui_cat.upgrade() {
                ui.set_is_icon_view(is_icon);
                ui_sync::refresh_items_ui(&ui, &state_cat.borrow(), &cat_id, &query);
            }
        }
    });

    let state_newcat = state.clone();
    let weak_ui_newcat = ui.as_weak();
    ui.on_new_category(move || {
        log::debug!("New category requested");
        let loc_nc = state_newcat.borrow().loc.clone();
        let mut state = state_newcat.borrow_mut();
        let sort_order = state.manager.categories().len() as i32;
        let new_cat = Category {
            id: model::generate_id("cat"),
            name: format!(
                "{} {}",
                loc_nc.borrow().tr("Default_NewCategory"),
                sort_order + 1
            ),
            sort_order,
            view_type: ViewType::Icon,
            icon_size: 48,
        };
        let cat_id = new_cat.id.clone();
        state.manager.add_category(new_cat);
        state.current_category_id = cat_id.clone();
        state.save_if_modified();
        drop(state);
        if let Some(ui) = weak_ui_newcat.upgrade() {
            ui_sync::refresh_categories_ui(&ui, &state_newcat.borrow());
            ui_sync::refresh_items_ui(&ui, &state_newcat.borrow(), &cat_id, "");
        }
    });

    let state_view = state.clone();
    let weak_ui_view = ui.as_weak();
    ui.on_toggle_view(move || {
        log::debug!("View toggled");
        // Rust 的 on_ 回调覆盖 slint 层 `toggle-view => { is-icon-view = !is-icon-view; }`
        // handler，需在此自行翻转并写回 UI 属性。
        if let Some(ui) = weak_ui_view.upgrade() {
            let new_icon = !ui.get_is_icon_view();
            ui.set_is_icon_view(new_icon);
        }
        let mut state = state_view.borrow_mut();
        let cat_id = state.current_category_id.clone();
        if let Some(cat) = state
            .manager
            .config_mut()
            .data
            .categories
            .iter_mut()
            .find(|c| c.id == cat_id)
        {
            cat.view_type = match cat.view_type {
                ViewType::Icon => ViewType::List,
                ViewType::List => ViewType::Icon,
            };
        }
        state.save_if_modified();
        let query = state.search_query.clone();
        drop(state);
        if let Some(ui) = weak_ui_view.upgrade() {
            ui_sync::refresh_items_ui(&ui, &state_view.borrow(), &cat_id, &query);
        }
    });

    let state_theme = state.clone();
    let weak_ui_theme = ui.as_weak();
    ui.on_toggle_theme(move || {
        // Rust 的 on_ 回调覆盖 slint 层 `toggle-theme => { is-dark-theme = !is-dark-theme;
        // Theme.set-dark(...) }` handler，需在此自行翻转并应用主题，否则界面不切换。
        let dark = weak_ui_theme
            .upgrade()
            .map(|ui| {
                let new_dark = !ui.get_is_dark_theme();
                ui.set_is_dark_theme(new_dark);
                new_dark
            })
            .unwrap_or(false);
        let mut state = state_theme.borrow_mut();
        state
            .manager
            .config_mut()
            .extra_config
            .insert("theme".into(), if dark { "1".into() } else { "0".into() });
        state.save_if_modified();
    });

    // 菜单"快捷键"入口：打开 InputDialog 预填当前快捷键（S4）
    let input_mode_hk = input_mode.clone();
    let state_hk = state.clone();
    let weak_ui_hk = ui.as_weak();
    ui.on_hotkey(move || {
        input_mode_hk.set(2);
        if let Some(ui) = weak_ui_hk.upgrade() {
            let hk = state_hk
                .borrow()
                .manager
                .config()
                .config
                .global_hotkey
                .clone();
            let title = state_hk.borrow().loc.borrow().tr("STR_HotKey").to_string();
            ui.set_input_dialog_title(title.into());
            ui.set_input_dialog_text(hk.into());
            ui.set_input_dialog_error("".into());
            ui.set_input_dialog_visible(true);
        }
    });

    // 设置页：选择背景图（png/jpg；解码最长边 2560 封顶，见 background.rs）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_select_background(move || {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            let loc_bg = state_bg.borrow().loc.clone();
            let (bg_title, bg_filter) = {
                let lb = loc_bg.borrow();
                (
                    lb.tr("Title_SelectBgImage").to_string(),
                    format!("{}\0*.png;*.jpg;*.jpeg\0\0", lb.tr("STR_FilterImages")),
                )
            };
            let Some(path) = fileops::browse_file_dialog(&bg_title, &bg_filter) else {
                return;
            };
            if apply_background(&ui, &state_bg, &path) {
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                ui.set_settings_bg_status(name.into());
            } else {
                let loc = state_bg.borrow().loc.clone();
                let msg = loc.borrow().tr("Tip_BG_LoadFail").to_string();
                ui_sync::show_toast(&ui, msg, true);
            }
        });
    }
    // 设置页：清除背景图（恢复纯色 bg-0）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_clear_background(move || {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_has_bg_image(false);
            ui.set_bg_image(slint::Image::default());
            let none_text = {
                let mut st = state_bg.borrow_mut();
                st.manager
                    .config_mut()
                    .extra_config
                    .remove("background_image");
                st.save_if_modified();
                // 先绑定再返回：避免临时 Ref 活过 st 的 drop（E0597）
                let text = st.loc.borrow().tr("Dlg_BG_None").to_string();
                text
            };
            ui.set_settings_bg_status(none_text.into());
        });
    }
    // 设置页：遮罩开关（低频点击，立即落盘）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_dim_toggled(move |enabled| {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_bg_dim_enabled(enabled);
            let mut st = state_bg.borrow_mut();
            st.manager.config_mut().extra_config.insert(
                "background_dim_enabled".into(),
                if enabled { "1".into() } else { "0".into() },
            );
            st.save_if_modified();
        });
    }
    // 设置页：遮罩浓度滑块（拖动中仅更新内存与 UI，关闭设置页时统一落盘）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_dim_changed(move |percent| {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_bg_dim((percent / 100.0).clamp(0.0, 1.0));
            let mut st = state_bg.borrow_mut();
            st.manager.config_mut().extra_config.insert(
                "background_dim".into(),
                (percent.round() as i32).clamp(0, 100).to_string(),
            );
        });
    }
    // 设置页关闭：把拖动中暂存的遮罩浓度统一落盘
    {
        let state_bg = state.clone();
        ui.on_settings_closed(move || {
            state_bg.borrow_mut().save_if_modified();
        });
    }

    let state_new = state.clone();
    let weak_ui_new = ui.as_weak();
    ui.on_new_item_clicked(move || {
        log::debug!("New item clicked");
        edit_dialog::show_edit_dialog(&weak_ui_new, &state_new);
    });

    let state_refresh = state.clone();
    let weak_ui_refresh = ui.as_weak();
    ui.on_refresh_icons(move || {
        log::debug!("Refresh icons");
        let state = state_refresh.borrow();
        let items = state.manager.items(&state.current_category_id);
        let icon_cache = state.icon_cache.clone();
        let pending = state.pending_icon_paths.clone();
        drop(state);
        if let Some(ui) = weak_ui_refresh.upgrade() {
            ui_sync::spawn_icon_refresh(&ui, icon_cache, items, pending);
        }
    });

    // 双击 / 回车（键盘导航）运行指定索引项目
    let state_dbl = state.clone();
    let weak_ui_dbl = ui.as_weak();
    // P1-3：以项目 id（而非下标）判定双击 —— 切换分类/搜索后同一下标指向不同项目，
    // 用下标会把"切分类后点第一项"误判为双击并启动错误程序。
    let last_click: Rc<std::cell::Cell<Option<(String, std::time::Instant)>>> =
        Rc::new(std::cell::Cell::new(None));
    let weak_ui_sel = weak_ui_dbl.clone();
    let state_sel = state_dbl.clone();
    ui.on_item_tile_clicked(move |index| {
        let now = std::time::Instant::now();
        let clicked_id = {
            let st = state_dbl.borrow();
            let items = if st.search_query.is_empty() {
                st.manager.items(&st.current_category_id)
            } else {
                st.manager.search_items(&st.search_query)
            };
            match items.get(index as usize) {
                Some(it) => it.id.clone(),
                None => return,
            }
        };
        let is_double = matches!(
            last_click.take(),
            Some((last_id, last_time))
                if last_id == clicked_id
                    && now.duration_since(last_time).as_millis() < 500
        );
        if is_double {
            log::debug!("Item double-clicked: index={}", index);
            ui_sync::run_item_at_index(&weak_ui_dbl, &state_dbl, index);
        } else {
            last_click.set(Some((clicked_id, now)));
        }
    });

    // 项目拖拽排序（F2-1，DragArea/DropArea）：
    // 1) tile down → payload-requested(index) → Rust 设 drag-payload = 源索引字符串。
    // 2) 拖到目标 tile → dropped-on(target, data) → Rust 读 data 得源索引 → 重排。
    let weak_ui_payload = ui.as_weak();
    ui.on_payload_requested(move |index| {
        log::debug!("Drag payload requested: index={}", index);
        if let Some(ui) = weak_ui_payload.upgrade() {
            use slint::private_unstable_api::re_exports::DataTransfer;
            let dt = DataTransfer::from(slint::SharedString::from(index.to_string()));
            ui.set_drag_payload(dt);
        }
    });

    let state_drop = state.clone();
    let weak_ui_drop = ui.as_weak();
    ui.on_dropped_on(move |target, data| {
        // data 载荷 = 源索引字符串（payload-requested 时写入）
        let source: usize = data
            .plain_text()
            .ok()
            .and_then(|s| s.to_string().parse().ok())
            .unwrap_or(usize::MAX);
        let target = target.max(0) as usize;
        log::debug!("Item dropped: source={}, target={}", source, target);
        if source == usize::MAX || source == target {
            return;
        }
        let mut state = state_drop.borrow_mut();
        if !state.search_query.is_empty() {
            log::debug!("Drop ignored during search");
            return;
        }
        let cat_id = state.current_category_id.clone();
        if cat_id.is_empty() {
            return;
        }
        if !state.manager.move_item_to_index(&cat_id, source, target) {
            return;
        }
        state.save_if_modified();
        drop(state);
        if let Some(ui) = weak_ui_drop.upgrade() {
            ui_sync::refresh_items_ui(&ui, &state_drop.borrow(), &cat_id, "");
        }
    });

    // 分类拖拽排序（F2-1，DragArea/DropArea）：down 设载荷 = 分类索引；拖入目标分类
    // → category-dropped-on(target, data) → 读 data 得源索引 → 重排 + 落盘 + 刷新侧栏。
    let weak_ui_cat_payload = ui.as_weak();
    ui.on_category_payload_requested(move |index| {
        log::debug!("Category drag payload requested: index={}", index);
        if let Some(ui) = weak_ui_cat_payload.upgrade() {
            use slint::private_unstable_api::re_exports::DataTransfer;
            let dt = DataTransfer::from(slint::SharedString::from(index.to_string()));
            ui.set_drag_payload(dt);
        }
    });

    let state_cdrop = state.clone();
    let weak_ui_cdrop = ui.as_weak();
    ui.on_category_dropped_on(move |target, data| {
        let source: usize = data
            .plain_text()
            .ok()
            .and_then(|s| s.to_string().parse().ok())
            .unwrap_or(usize::MAX);
        let target = target.max(0) as usize;
        log::debug!("Category dropped: source={}, target={}", source, target);
        if source == usize::MAX || source == target {
            return;
        }
        let mut state = state_cdrop.borrow_mut();
        if !state.manager.move_category_to_index(source, target) {
            return;
        }
        state.save_if_modified();
        drop(state);
        if let Some(ui) = weak_ui_cdrop.upgrade() {
            ui_sync::refresh_categories_ui(&ui, &state_cdrop.borrow());
        }
    });

    // 滚轮切页（F7/参考特性）：当前分类 ± delta，首尾循环回绕。
    let state_ws = state.clone();
    let weak_ui_ws = ui.as_weak();
    ui.on_wheel_switch(move |delta| {
        log::debug!("Wheel switch: delta={}", delta);
        let mut state = state_ws.borrow_mut();
        let n = state.manager.categories().len();
        if n == 0 {
            return;
        }
        let cur_idx = state
            .manager
            .categories()
            .iter()
            .position(|c| c.id == state.current_category_id)
            .unwrap_or(0);
        let new_idx = ((cur_idx as i32 + delta).rem_euclid(n as i32)) as usize;
        let cat = state.manager.categories()[new_idx].clone();
        state.current_category_id = cat.id.clone();
        let is_icon = matches!(cat.view_type, ViewType::Icon);
        drop(state);
        if let Some(ui) = weak_ui_ws.upgrade() {
            ui.set_sidebar_selected_index(new_idx as i32);
            ui.set_is_icon_view(is_icon);
            ui_sync::refresh_items_ui(&ui, &state_ws.borrow(), &cat.id, "");
        }
    });

    ui.on_item_selected(move || {
        let ui = match weak_ui_sel.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = ui.get_selected_item_index();
        log::debug!("Enter: run item index={}", idx);
        ui_sync::run_item_at_index(&weak_ui_sel, &state_sel, idx);
    });
    // 项目右键菜单（v2 重构）：动作映射分发（对齐汉堡菜单模式）。
    // MoveTo 携带目标分类 id（替代 moveto_base + 位置的索引换算）。
    #[derive(Clone, PartialEq)]
    enum ItemMenuAction {
        Edit,
        RunAsAdmin,
        Delete,
        CopyPath,
        OpenLocation,
        MoveUp,
        MoveDown,
        Run,
        RefreshIcon,
        Properties,
        MoveTo(String),
        OpenFolder,
    }
    let item_menu_actions: Rc<RefCell<Vec<ItemMenuAction>>> = Rc::new(RefCell::new(Vec::new()));

    let rci = right_clicked_index.clone();
    let mk_item = menu_kind.clone();
    let state_search_menu = state.clone();
    let weak_ui_right = ui.as_weak();
    let item_actions_build = item_menu_actions.clone();
    ui.on_item_right_clicked(move |index, x, y| {
        log::debug!("Item right-clicked: index={}", index);
        mk_item.set(0);
        rci.set(Some(index));
        // 搜索态排序语义与全局向量不一致，隐藏 Move Up/Down（建议④）
        let st = state_search_menu.borrow();
        let searching = !st.search_query.is_empty();
        let cats: Vec<(String, String)> = st
            .manager
            .categories()
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        // 菜单文本与动作平行构建（对齐汉堡菜单模式，增删项不再依赖手写索引）
        let mut menu_vec: Vec<slint::SharedString> = Vec::new();
        let mut actions: Vec<ItemMenuAction> = Vec::new();
        menu_vec.push(lb.tr("Menu_Edit").into());
        actions.push(ItemMenuAction::Edit);
        menu_vec.push(lb.tr("Menu_RunAsAdmin").into());
        actions.push(ItemMenuAction::RunAsAdmin);
        menu_vec.push(lb.tr("Menu_DelItem").into());
        actions.push(ItemMenuAction::Delete);
        menu_vec.push(lb.tr("Menu_CopyPath").into());
        actions.push(ItemMenuAction::CopyPath);
        menu_vec.push(lb.tr("Menu_OpenLocationFile").into());
        actions.push(ItemMenuAction::OpenLocation);
        if !searching {
            menu_vec.push(lb.tr("Menu_MoveUp").into());
            actions.push(ItemMenuAction::MoveUp);
            menu_vec.push(lb.tr("Menu_MoveDown").into());
            actions.push(ItemMenuAction::MoveDown);
        }
        menu_vec.push(lb.tr("Menu_Run").into());
        actions.push(ItemMenuAction::Run);
        menu_vec.push(lb.tr("Menu_RefreshIcon").into());
        actions.push(ItemMenuAction::RefreshIcon);
        menu_vec.push(lb.tr("Menu_Properties").into());
        actions.push(ItemMenuAction::Properties);
        let move_to = lb.tr("Menu_MoveTo").to_string();
        let open_folder = lb.tr("Menu_OpenFolder").to_string();
        drop(lb);
        for (cat_id, name) in &cats {
            menu_vec.push(slint::SharedString::from(format!("{} {}", move_to, name)));
            actions.push(ItemMenuAction::MoveTo(cat_id.clone()));
        }
        menu_vec.push(open_folder.into());
        actions.push(ItemMenuAction::OpenFolder);
        *item_actions_build.borrow_mut() = actions;
        let item_count = menu_vec.len();
        let content_h = menu_content_height(&menu_vec);
        let menu_items = Rc::new(slint::VecModel::from(menu_vec));
        // v2 危险项标红：删除项为 danger；由 action 映射生成，避免索引错位
        let danger: Vec<bool> = item_actions_build
            .borrow()
            .iter()
            .map(|a| matches!(a, ItemMenuAction::Delete))
            .collect();
        let danger_model = Rc::new(slint::VecModel::from(danger));
        if let Some(ui) = weak_ui_right.upgrade() {
            ui.set_context_menu_content_height(content_h);
            ui.set_context_menu_items(menu_items.into());
            ui.set_context_menu_danger(danger_model.into());
            geometry::position_menu_at_cursor(&ui, x, y, item_count);
        }
    });

    // 背景右键菜单（P1-1）：新建项目/分类、刷新图标、粘贴、排序
    let mk_bg = menu_kind.clone();
    let rci_bg = right_clicked_index.clone();
    let state_bg = state.clone();
    let weak_ui_bg = ui.as_weak();
    ui.on_background_right_clicked(move |x, y| {
        log::debug!("Background right-clicked");
        mk_bg.set(1);
        rci_bg.set(None);
        let st = state_bg.borrow();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        let sort_tpl = lb.tr("Menu_SortBy").to_string();
        let sort_name = lb.tr("Menu_Sort_Name").to_string();
        let sort_usage = lb.tr("Menu_Sort_RunCount").to_string();
        let menu_vec = vec![
            slint::SharedString::from(lb.tr("Menu_NewItem")),
            slint::SharedString::from(lb.tr("Menu_NewC")),
            slint::SharedString::from(lb.tr("Menu_RefreshIcon")),
            slint::SharedString::from(lb.tr("Menu_Paste")),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_name)),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_usage)),
            slint::SharedString::from(lb.tr("Menu_RunAll")),
        ];
        let content_h = menu_content_height(&menu_vec);
        if let Some(ui) = weak_ui_bg.upgrade() {
            ui.set_context_menu_content_height(content_h);
            ui.set_context_menu_items(Rc::new(slint::VecModel::from(menu_vec)).into());
            // v2：背景菜单无危险项
            let danger: Vec<bool> = vec![false; 7];
            ui.set_context_menu_danger(Rc::new(slint::VecModel::from(danger)).into());
            geometry::position_menu_at_cursor(&ui, x, y, 7);
        }
    });

    // 分类右键菜单（P1-3）：重命名/删除/上移/下移（决策①：右键上移下移替代拖拽）
    let mk_cat = menu_kind.clone();
    let rcc_cat = right_clicked_category.clone();
    let state_cat_menu = state.clone();
    let weak_ui_cat_right = ui.as_weak();
    ui.on_category_right_clicked(move |index, x, y| {
        log::debug!("Category right-clicked: index={}", index);
        mk_cat.set(2);
        rcc_cat.set(Some(index));
        let st = state_cat_menu.borrow();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        let menu_vec = vec![
            slint::SharedString::from(lb.tr("Menu_ReName")),
            slint::SharedString::from(lb.tr("Menu_DelC")),
            slint::SharedString::from(lb.tr("Menu_MoveUp")),
            slint::SharedString::from(lb.tr("Menu_MoveDown")),
        ];
        let content_h = menu_content_height(&menu_vec);
        if let Some(ui) = weak_ui_cat_right.upgrade() {
            ui.set_context_menu_content_height(content_h);
            ui.set_context_menu_items(Rc::new(slint::VecModel::from(menu_vec)).into());
            // v2 危险项标红：删除分类(index 1)
            let danger: Vec<bool> = vec![false, true, false, false];
            ui.set_context_menu_danger(Rc::new(slint::VecModel::from(danger)).into());
            geometry::position_menu_at_cursor(&ui, x, y, 4);
        }
    });

    let state_menu = state.clone();
    let weak_ui_menu = ui.as_weak();
    let pd_menu = pending_delete.clone();
    let mk_menu = menu_kind.clone();
    let rcc_menu = right_clicked_category.clone();
    let pcd_menu = pending_category_delete.clone();
    let pcr_menu = pending_category_rename.clone();
    let pra_menu = pending_run_all.clone();
    let pra_admin_menu = pending_run_admin.clone();
    let im_menu = input_mode.clone();
    let item_menu_actions_menu = item_menu_actions.clone();

    // 汉堡菜单（v2）：组装单一列表 + 危险项数组；动作经 action 映射分发
    //（不依赖易错的手写索引，增删菜单项时对应关系自动保持）。仅中文，无语言切换项。
    #[derive(Clone, Copy, PartialEq)]
    enum MenuAction {
        NewItem,
        NewCategory,
        DelCategory,
        RenameCategory,
        Hotkey,
        ToggleView,
        ToggleTheme,
        FollowMouse,
        AutoStart,
        Export,
        Import,
        Settings,
        About,
        Exit,
    }
    let menu_actions: Rc<RefCell<Vec<Option<MenuAction>>>> = Rc::new(RefCell::new(Vec::new()));
    let state_hb = state.clone();
    let weak_ui_hb = ui.as_weak();
    let menu_actions_hb = menu_actions.clone();
    ui.on_hamburger_clicked(move || {
        log::debug!("Hamburger menu opened");
        let ui = match weak_ui_hb.upgrade() {
            Some(u) => u,
            None => return,
        };
        let dark = ui.get_is_dark_theme();
        let icon_view = ui.get_is_icon_view();
        let items: Vec<slint::SharedString> = {
            let st = state_hb.borrow();
            let lb = st.loc.borrow();
            let mut items: Vec<slint::SharedString> = Vec::new();
            let mut actions: Vec<Option<MenuAction>> = Vec::new();
            let mut add = |s: slint::SharedString, a: Option<MenuAction>| {
                items.push(s);
                actions.push(a);
            };
            add(lb.tr("Menu_NewItem").into(), Some(MenuAction::NewItem));
            add(lb.tr("Menu_NewC").into(), Some(MenuAction::NewCategory));
            add("-".into(), None);
            add(
                lb.tr("Menu_DelCurrentC").into(),
                Some(MenuAction::DelCategory),
            );
            add("-".into(), None);
            add(
                lb.tr("Menu_RenameCurrentC").into(),
                Some(MenuAction::RenameCategory),
            );
            add(lb.tr("Menu_ViewHotKey").into(), Some(MenuAction::Hotkey));
            add("-".into(), None);
            // 切换视图：当前图标视图显示"列表"，反之显示"图标"
            if icon_view {
                add(lb.tr("Menu_ViewIcon1").into(), Some(MenuAction::ToggleView));
            } else {
                add(lb.tr("Menu_ViewIcon0").into(), Some(MenuAction::ToggleView));
            }
            let theme_key = if dark {
                "Menu_SwitchLight"
            } else {
                "Menu_SwitchDark"
            };
            add(lb.tr(theme_key).into(), Some(MenuAction::ToggleTheme));
            add("-".into(), None);
            add(
                lb.tr("Menu_FollowMouse").into(),
                Some(MenuAction::FollowMouse),
            );
            add(lb.tr("Tray_AutoStart").into(), Some(MenuAction::AutoStart));
            add("-".into(), None);
            add(lb.tr("Menu_ExportConfig").into(), Some(MenuAction::Export));
            add(lb.tr("Menu_ImportConfig").into(), Some(MenuAction::Import));
            add("-".into(), None);
            add(lb.tr("Menu_Settings").into(), Some(MenuAction::Settings));
            add(lb.tr("Menu_About").into(), Some(MenuAction::About));
            add(lb.tr("Menu_Exit").into(), Some(MenuAction::Exit));
            *menu_actions_hb.borrow_mut() = actions;
            items
        };
        // 危险项：删除当前分类、退出（由 action 映射生成，避免索引错位）
        let danger: Vec<bool> = menu_actions_hb
            .borrow()
            .iter()
            .map(|a| matches!(a, Some(MenuAction::DelCategory) | Some(MenuAction::Exit)))
            .collect();
        // 弹层内容总高：项 28px、分隔线 1px、上下 padding 4px——供挂载处限高滚动
        let content_h = menu_content_height(&items);
        ui.set_menu_popup_content_height(content_h);
        ui.set_menu_popup_items(Rc::new(slint::VecModel::from(items)).into());
        ui.set_menu_popup_danger(Rc::new(slint::VecModel::from(danger)).into());
    });

    let state_mps = state.clone();
    let weak_ui_mps = ui.as_weak();
    let pcd_mps = pending_category_delete.clone();
    let pcr_mps = pending_category_rename.clone();
    let im_mps = input_mode.clone();
    let menu_actions_mps = menu_actions.clone();
    ui.on_menu_popup_selected(move |index| {
        log::debug!("Menu popup item selected: index={}", index);
        let ui = match weak_ui_mps.upgrade() {
            Some(u) => u,
            None => return,
        };
        let action = menu_actions_mps
            .borrow()
            .get(index as usize)
            .copied()
            .flatten();
        match action {
            Some(MenuAction::NewItem) => ui.invoke_new_item_clicked(),
            Some(MenuAction::NewCategory) => ui.invoke_new_category(),
            Some(MenuAction::DelCategory) => {
                // 删除当前分类：复用分类右键删除流程（确认后级联删除）
                let state = state_mps.borrow_mut();
                let cat_id = state.current_category_id.clone();
                if cat_id.is_empty() {
                    return;
                }
                let loc_m = state.loc.clone();
                let cat_name = state
                    .manager
                    .categories()
                    .iter()
                    .find(|c| c.id == cat_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                *pcd_mps.borrow_mut() = Some(cat_id);
                drop(state);
                ui.set_confirm_dialog_title(loc_m.borrow().tr("Title_ConfirmDelete").into());
                ui.set_confirm_dialog_message(
                    util::tr_fill(&loc_m, "Tip_DelCate", &cat_name).into(),
                );
                ui.set_confirm_confirm_text(loc_m.borrow().tr("Btn_Del").to_string().into());
                ui.set_confirm_dialog_visible(true);
            }
            Some(MenuAction::RenameCategory) => {
                // 重命名当前分类：复用分类右键重命名流程（InputDialog）
                let state = state_mps.borrow_mut();
                let cat_id = state.current_category_id.clone();
                if cat_id.is_empty() {
                    return;
                }
                let loc_m = state.loc.clone();
                let cat_name = state
                    .manager
                    .categories()
                    .iter()
                    .find(|c| c.id == cat_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                *pcr_mps.borrow_mut() = Some(cat_id);
                im_mps.set(1);
                drop(state);
                ui.set_input_dialog_title(loc_m.borrow().tr("Title_RenameCategory").into());
                ui.set_input_dialog_text(cat_name.into());
                ui.set_input_dialog_error("".into());
                ui.set_input_dialog_visible(true);
            }
            Some(MenuAction::Hotkey) => ui.invoke_hotkey(),
            Some(MenuAction::ToggleView) => {
                // 视图切换（v2 单一"切换"项，无条件 toggle）
                ui.invoke_toggle_view();
            }
            Some(MenuAction::ToggleTheme) => ui.invoke_toggle_theme(),
            Some(MenuAction::FollowMouse) => crate::tray_ui::handle_tray_ui_action(
                crate::tray_ui::TrayUiAction::ToggleFollowMouse,
                &ui,
                &state_mps,
            ),
            Some(MenuAction::AutoStart) => {
                let _ = crate::tray::toggle_autostart();
            }
            Some(MenuAction::Export) => crate::handlers::config_io::export_config(&ui, &state_mps),
            Some(MenuAction::Import) => crate::handlers::config_io::import_config(&ui, &state_mps),
            Some(MenuAction::Settings) => {
                // 打开设置对话框：回填背景图与遮罩当前状态
                let st = state_mps.borrow();
                let cfg = st.manager.config();
                let status = match cfg.extra_config.get("background_image") {
                    Some(p) => std::path::Path::new(p)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.clone()),
                    None => st.loc.borrow().tr("Dlg_BG_None").to_string(),
                };
                let dim_enabled = cfg
                    .extra_config
                    .get("background_dim_enabled")
                    .map(|v| v == "1")
                    .unwrap_or(true);
                let dim_percent: f32 = cfg
                    .extra_config
                    .get("background_dim")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(35.0);
                let log_index = match cfg.extra_config.get("log_level").map(|s| s.as_str()) {
                    Some("info") => 1,
                    Some("debug") => 2,
                    // 未设置（空）与实际生效档位一致：默认 Debug（见 logger::apply_level）
                    None | Some("") => 2,
                    _ => 0,
                };
                drop(st);
                ui.set_settings_bg_status(status.into());
                ui.set_settings_bg_dim_enabled(dim_enabled);
                ui.set_settings_bg_dim_percent(dim_percent);
                ui.set_settings_log_index(log_index);
                ui.set_settings_dialog_visible(true);
            }
            Some(MenuAction::About) => ui.set_about_dialog_visible(true),
            Some(MenuAction::Exit) => {
                // 退出：真正结束进程（与托盘"退出"一致）；几何由主循环收尾强制落盘
                let _ = slint::quit_event_loop();
            }
            None => {}
        }
    });

    ui.on_menu_item_selected(move |index| {
        log::debug!("Menu item selected: index={}, kind={}", index, mk_menu.get());

        // 分类菜单（P1-3）：重命名/删除/上移/下移
        if mk_menu.get() == 2 {
            let cat_idx = rcc_menu.get();
            if let Some(idx) = cat_idx {
                match index {
                    0 => {
                        // 重命名：打开 InputDialog
                        let state = state_menu.borrow_mut();
                        let loc_m = state.loc.clone();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            let cat_name = cat.name.clone();
                            *pcr_menu.borrow_mut() = Some(cat_id);
                            im_menu.set(1);
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui.set_input_dialog_title(
                                    loc_m.borrow().tr("Title_RenameCategory").into(),
                                );
                                ui.set_input_dialog_text(cat_name.into());
                                ui.set_input_dialog_error("".into());
                                ui.set_input_dialog_visible(true);
                            }
                        }
                    }
                    1 => {
                        // 删除：确认后级联删除项目
                        let state = state_menu.borrow_mut();
                        let loc_m = state.loc.clone();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            let cat_name = cat.name.clone();
                            *pcd_menu.borrow_mut() = Some(cat_id);
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui.set_confirm_dialog_title(
                                    loc_m.borrow().tr("Title_ConfirmDelete").into(),
                                );
                                ui.set_confirm_dialog_message(
                                    util::tr_fill(&loc_m, "Tip_DelCate", &cat_name).into(),
                                );
                                ui.set_confirm_confirm_text(
                                    loc_m.borrow().tr("Btn_Del").to_string().into(),
                                );
                                ui.set_confirm_dialog_visible(true);
                            }
                        }
                    }
                    2 => {
                        let mut state = state_menu.borrow_mut();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            if state.manager.move_category_up(&cat_id) {
                                state.save_if_modified();
                                let cur = state.current_category_id.clone();
                                let q = state.search_query.clone();
                                drop(state);
                                if let Some(ui) = weak_ui_menu.upgrade() {
                                    ui_sync::refresh_categories_ui(&ui, &state_menu.borrow());
                                    ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cur, &q);
                                }
                            }
                        }
                    }
                    3 => {
                        let mut state = state_menu.borrow_mut();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            if state.manager.move_category_down(&cat_id) {
                                state.save_if_modified();
                                let cur = state.current_category_id.clone();
                                let q = state.search_query.clone();
                                drop(state);
                                if let Some(ui) = weak_ui_menu.upgrade() {
                                    ui_sync::refresh_categories_ui(&ui, &state_menu.borrow());
                                    ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cur, &q);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            return;
        }

        // 背景菜单（P1-1）：新建项目/分类、刷新图标、粘贴、排序
        if mk_menu.get() == 1 {
            match index {
                0 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_new_item_clicked();
                    }
                }
                1 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_new_category();
                    }
                }
                2 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_refresh_icons();
                    }
                }
                3 => {
                    // 粘贴：读剪贴板文件路径并加入当前分类
                    let paths = clipboard::clipboard_files();
                    if !paths.is_empty() {
                        let mut state = state_menu.borrow_mut();
                        let cat_id = state.current_category_id.clone();
                        let added = state.manager.handle_drop(&paths, &cat_id);
                        let count = added.len();
                        state.save_if_modified();
                        let q = state.search_query.clone();
                        let loc_m = state.loc.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            ui_sync::show_toast(
                                &ui,
                                util::tr_fill(&loc_m, "Status_Pasted", &count.to_string()),
                                false,
                            );
                        }
                    }
                }
                4 => {
                    let mut state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    state.manager.sort_items_by_name(&cat_id);
                    state.save_if_modified();
                    let q = state.search_query.clone();
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                    }
                }
                5 => {
                    let mut state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    state.manager.sort_items_by_usage(&cat_id);
                    state.save_if_modified();
                    let q = state.search_query.clone();
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                    }
                }
                6 => {
                    // 运行全部：弹确认框（Tip_RunAll），确认后逐一执行分类内项目
                    let state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    let cat_name = state
                        .manager
                        .categories()
                        .iter()
                        .find(|c| c.id == cat_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let loc_m = state.loc.clone();
                    *pra_menu.borrow_mut() = Some(cat_id);
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.set_confirm_dialog_title(
                            loc_m.borrow().tr("Title_ConfirmRunAll").into(),
                        );
                        ui.set_confirm_dialog_message(
                            util::tr_fill(&loc_m, "Tip_RunAll", &cat_name).into(),
                        );
                        ui.set_confirm_confirm_text(
                            loc_m.borrow().tr("Btn_Execute").to_string().into(),
                        );
                        ui.set_confirm_dialog_visible(true);
                    }
                }
                _ => {}
            }
            return;
        }

        // 项目菜单（现有逻辑 + Move Up/Down + Run/RefreshIcon/Properties/MoveTo）
        if let Some(item_idx) = right_clicked_index.get() {
            let mut state = state_menu.borrow_mut();
            let items = if state.search_query.is_empty() {
                state.manager.items(&state.current_category_id)
            } else {
                state.manager.search_items(&state.search_query)
            };
            if let Some(item) = items.get(item_idx as usize) {
                let item = item.clone();
                // 动作映射分发（对齐汉堡菜单模式）：搜索态构建菜单时已排除
                // MoveUp/MoveDown，无需索引换算与 normal_mode 守卫。
                let action = item_menu_actions_menu.borrow().get(index as usize).cloned();
                match action {
                    Some(ItemMenuAction::Edit) => {
                        log::info!("Edit item: {} ({})", item.name, item.id);
                        let categories: Vec<slint::SharedString> = state
                            .manager
                            .categories()
                            .iter()
                            .map(|c| c.name.clone().into())
                            .collect();
                        let selected_cat_idx = state
                            .manager
                            .categories()
                            .iter()
                            .position(|c| c.id == item.category_id)
                            .unwrap_or(0);

                        if let Some(_ui) = weak_ui_menu.upgrade() {
                            let dialog = match EditDialog::new() {
                                Ok(d) => d,
                                Err(e) => {
                                    log::error!("Failed to create edit dialog: {}", e);
                                    return;
                                }
                            };
                            dialog.set_item_name(item.name.clone().into());
                            dialog.set_item_target(item.target.clone().into());
                            dialog.set_item_arguments(item.arguments.clone().into());
                            dialog.set_item_working_dir(item.working_dir.clone().into());
                            dialog.set_item_icon_path(item.icon_path.clone().into());
                            dialog.set_item_icon_index(item.icon_index);
                            dialog.set_item_keywords(item.keywords.clone().into());
                            dialog.set_item_remark(item.remark.clone().into());
                            dialog.set_item_hotkey(item.hotkey.clone().into());
                            dialog.set_run_as_admin(item.run_as_admin);
                            dialog.set_category_names(
                                Rc::new(slint::VecModel::from(categories)).into(),
                            );
                            dialog.set_selected_category_index(selected_cat_idx as i32);
                            ui_sync::set_edit_dialog_texts(&dialog, &state.loc.clone());
                            dialogs::setup_edit_dialog_new_category(
                                &dialog,
                                state_menu.clone(),
                                &weak_ui_menu,
                            );

                            let dialog_weak = dialog.as_weak();
                            let loc_browse = state.loc.clone();
                            if let Some(dlg_handle) = dialog_weak.upgrade() {
                                {
                                    let dlg_browse = dialog_weak.clone();
                                    let (t_title, t_filter) =
                                        browse_texts(&loc_browse, BrowseKind::Target);
                                    dlg_handle.on_browse_target(move || {
                                        if let Some(d) = dlg_browse.upgrade() {
                                            if let Some(path) =
                                                fileops::browse_file_dialog(&t_title, &t_filter)
                                            {
                                                d.set_item_target(path.clone().into());
                                                // 编辑态：仅回填工作目录（不覆盖已有名）
                                                ui_sync::fill_from_target(&d, &path, false);
                                            }
                                        }
                                    });
                                }
                                {
                                    let dlg_icon = dialog_weak.clone();
                                    let (i_title, i_filter) =
                                        browse_texts(&loc_browse, BrowseKind::Icon);
                                    dlg_handle.on_browse_icon(move || {
                                        if let Some(d) = dlg_icon.upgrade() {
                                            if let Some(path) =
                                                fileops::browse_file_dialog(&i_title, &i_filter)
                                            {
                                                d.set_item_icon_path(path.into());
                                            }
                                        }
                                    });
                                }
                                let item_for_save = item.clone();
                                let dialog_weak_save = dialog_weak.clone();
                                let state_edit = state_menu.clone();
                                let weak_ui_edit = weak_ui_menu.clone();
                                dlg_handle.on_save(move || {
                                    let dialog_strong = dialog_weak_save.upgrade();
                                    let dlg = match &dialog_strong {
                                        Some(d) => d,
                                        None => {
                                            log::error!("Edit dialog no longer exists");
                                            return;
                                        }
                                    };
                                    // 输入验证：名称/目标非空 + 热键格式可解析（复用新建对话框校验）
                                    let name = dlg.get_item_name().to_string();
                                    let target = dlg.get_item_target().to_string();
                                    let hotkey = dlg.get_item_hotkey().to_string();
                                    let err_key = validate_item_input(&name, &target)
                                        .or_else(|| validate_hotkey_input(&hotkey));
                                    if let Some(err_key) = err_key {
                                        let loc = state_edit.borrow().loc.clone();
                                        let msg = loc.borrow().tr(err_key).to_string();
                                        dlg.set_error_message(msg.into());
                                        return;
                                    }
                                    dlg.set_error_message("".into());
                                    let mut state = state_edit.borrow_mut();
                                    state.icon_cache.delete_cache(&item_for_save);
                                    let mut updated_item = item_for_save.clone();
                                    updated_item.name = dlg.get_item_name().to_string();
                                    updated_item.target = dlg.get_item_target().to_string();
                                    updated_item.arguments = dlg.get_item_arguments().to_string();
                                    updated_item.working_dir =
                                        dlg.get_item_working_dir().to_string();
                                    updated_item.icon_path =
                                        dlg.get_item_icon_path().to_string();
                                    updated_item.icon_index = dlg.get_item_icon_index();
                                    // P0：图标源（icon_path/icon_index/target）可能已变，
                                    // 清空旧缓存路径，交由 refresh_items_ui 重新提取并回写，
                                    // 否则会继续显示编辑前的旧图标。
                                    updated_item.cached_icon = String::new();
                                    updated_item.keywords = dlg.get_item_keywords().to_string();
                                    updated_item.remark = dlg.get_item_remark().to_string();
                                    updated_item.hotkey = dlg.get_item_hotkey().to_string();
                                    updated_item.run_as_admin = dlg.get_run_as_admin();
                                    let cat_idx = dlg.get_selected_category_index() as usize;
                                    if let Some(cat) = state.manager.categories().get(cat_idx) {
                                        updated_item.category_id = cat.id.clone();
                                    }

                                    if state.manager.update_item(&updated_item) {
                                        state.save_if_modified();
                                        let cat_id = state.current_category_id.clone();
                                        let query = state.search_query.clone();
                                        drop(state);
                                        if let Some(ui) = weak_ui_edit.upgrade() {
                                            ui_sync::refresh_items_ui(
                                                &ui,
                                                &state_edit.borrow(),
                                                &cat_id,
                                                &query,
                                            );
                                        }
                                        // F8/4.1：编辑条目后重注册条目热键；失败列表非空时提示冲突
                                        let hotkey_failed =
                                            ui_sync::sync_item_hotkeys(&state_edit.borrow());
                                        if let (Some(ui), false) = (
                                            weak_ui_edit.upgrade(),
                                            hotkey_failed.is_empty(),
                                        ) {
                                            let loc = state_edit.borrow().loc.clone();
                                            let msg = util::tr_fill(
                                                &loc,
                                                "Status_HotkeyConflict",
                                                &hotkey_failed.join(", "),
                                            );
                                            ui_sync::show_toast(&ui, msg, true);
                                        }
                                    } else {
                                        drop(state);
                                    }
                                    let _ = dlg.hide();
                                    state_edit.borrow_mut().edit_dialog = None;
                                });
                                let dialog_weak_cancel = dialog_weak.clone();
                                let state_cancel = state_menu.clone();
                                dlg_handle.on_cancel(move || {
                                    let dialog_strong = dialog_weak_cancel.upgrade();
                                    if let Some(d) = &dialog_strong {
                                        let _ = d.hide();
                                    }
                                    state_cancel.borrow_mut().edit_dialog = None;
                                });
                                // P2-06：标题栏 X 关闭等价取消
                                let dialog_weak_close = dialog_weak.clone();
                                let state_close = state_menu.clone();
                                dlg_handle.window().on_close_requested(move || {
                                    if let Some(d) = dialog_weak_close.upgrade() {
                                        let _ = d.hide();
                                    }
                                    state_close.borrow_mut().edit_dialog = None;
                                    slint::CloseRequestResponse::HideWindow
                                });
                            }

                            // P0：打开新对话框前关闭旧实例，避免多实例并存导致状态跟踪不一致
                            if let Some(old) = state.edit_dialog.take() {
                                let _ = old.hide();
                            }
                            state.edit_dialog = Some(dialog);

                            if let Err(e) = state
                                .edit_dialog
                                .as_ref()
                                .map(|d| d.show())
                                .unwrap_or(Ok(()))
                            {
                                log::error!("Failed to show dialog: {}", e);
                            }
                        }
                    }
                    Some(ItemMenuAction::RunAsAdmin) => {
                        log::info!("Run as admin: {} ({})", item.name, item.id);
                        let item_id = item.id.clone();
                        let item_name = item.name.clone();
                        let item_target = item.target.clone();
                        let loc_a = state.loc.clone();
                        *pra_admin_menu.borrow_mut() = Some(item_id);
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui.set_confirm_dialog_title(
                                loc_a.borrow().tr("Title_ConfirmRunAsAdmin").into(),
                            );
                            let msg = format!(
                                "{} — {}\n{}",
                                item_name,
                                item_target,
                                loc_a.borrow().tr("Tip_UndoNotPossible")
                            );
                            ui.set_confirm_dialog_message(msg.into());
                            ui.set_confirm_confirm_text(
                                loc_a.borrow().tr("Btn_Execute").to_string().into(),
                            );
                            ui.set_confirm_dialog_visible(true);
                        }
                    }
                    Some(ItemMenuAction::Delete) => {
                        log::info!("Delete item: {} ({})", item.name, item.id);
                        let item_id = item.id.clone();
                        let item_name = item.name.clone();
                        let loc_m = state.loc.clone();
                        *pd_menu.borrow_mut() = Some(item_id);
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui.set_confirm_dialog_title(
                                loc_m.borrow().tr("Title_ConfirmDelete").into(),
                            );
                            ui.set_confirm_dialog_message(
                                util::tr_fill(&loc_m, "Tip_DelItem", &item_name).into(),
                            );
                            ui.set_confirm_confirm_text(
                                loc_m.borrow().tr("Btn_Del").to_string().into(),
                            );
                            ui.set_confirm_dialog_visible(true);
                        }
                    }
                    Some(ItemMenuAction::CopyPath) => {
                        clipboard::copy_to_clipboard(&item.target);
                    }
                    Some(ItemMenuAction::OpenLocation) => {
                        if crate::runner::is_url(&item.target) {
                            log::debug!(
                                "Skip open file location for URL target: {}",
                                item.target
                            );
                        } else {
                            fileops::open_file_location(&item.target);
                        }
                    }
                    Some(ItemMenuAction::Run) => {
                        log::info!("Run item: {} ({})", item.name, item.id);
                        let search_query = state.search_query.clone();
                        drop(state);
                        // P0-1：右键"运行"同样尊重 run_as_admin 标志（与双击/回车一致）
                        let result = if item.run_as_admin {
                            crate::runner::run_as_admin(&item, &search_query)
                        } else {
                            crate::runner::run(&item, &search_query)
                        };
                        if result.success {
                            let mut state = state_menu.borrow_mut();
                            state.manager.increment_run_count(&item.id);
                            state.save_if_modified();
                        } else {
                            log::error!("Failed to launch {}: {}", item.name, result.error_message);
                            let msg = {
                                let loc_m = state_menu.borrow().loc.clone();
                                let err_text = ui_sync::run_error_text(&loc_m, &result.error);
                                util::tr_fill(
                                    &loc_m,
                                    "Status_RunFail",
                                    &format!("{}: {}", item.name, err_text),
                                )
                            };
                            let weak = weak_ui_menu.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = weak.upgrade() {
                                    ui_sync::show_toast(&ui, msg, true);
                                }
                            });
                        }
                    }
                    Some(ItemMenuAction::RefreshIcon) => {
                        log::info!("Refresh icon: {} ({})", item.name, item.id);
                        let icon_cache = state.icon_cache.clone();
                        let pending = state.pending_icon_paths.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::spawn_icon_refresh(&ui, icon_cache, vec![item.clone()], pending);
                        }
                    }
                    Some(ItemMenuAction::Properties) => {
                        log::info!("Properties: {} ({})", item.target, item.id);
                        if item.target.trim().is_empty() {
                            // 空目标条目（如历史占位 "New Item"）：明确提示而非静默无动作
                            let msg = {
                                let loc_m = state.loc.clone();
                                let lb = loc_m.borrow();
                                lb.tr("Err_EmptyTarget").to_string()
                            };
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::show_toast(&ui, msg, true);
                            }
                        } else {
                            drop(state);
                            fileops::show_properties(&item.target);
                        }
                    }
                    Some(ItemMenuAction::MoveTo(target_cat)) => {
                        if state.manager.move_item(&item.id, &target_cat) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                        }
                    }
                    Some(ItemMenuAction::MoveUp) => {
                        if state.manager.move_item_up(&item.id) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                        }
                    }
                    Some(ItemMenuAction::MoveDown) => {
                        if state.manager.move_item_down(&item.id) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                        }
                    }
                    Some(ItemMenuAction::OpenFolder) => {
                        crate::fileops::open_folder(&item.target);
                    }
                    None => {}
                }
            }
        }
    });

    let state_confirm = state.clone();
    let weak_ui_confirm = ui.as_weak();
    let pd_ok = pending_delete.clone();
    let pcd_ok = pending_category_delete.clone();
    let pra_ok = pending_run_all.clone();
    let pra_admin_ok = pending_run_admin.clone();
    ui.on_confirm_dialog_ok(move || {
        // 分类删除优先（级联删除项目 + 清理图标缓存）
        if let Some(cat_id) = pcd_ok.borrow_mut().take() {
            let mut state = state_confirm.borrow_mut();
            // 先读取待删分类下的项目，用于后续清理 icon_cache（delete_category 后 items 已移除）
            let cat_items = state.manager.items(&cat_id);
            // P1-1：至少保留一个分类，删不动时提示并退出
            if !state.manager.delete_category(&cat_id) {
                // .to_string() 避免 tr() 返回 &str 借用临时 Ref（E0716）
                let tip = state.loc.borrow().tr("Tip_NeedOneCategory").to_string();
                drop(state);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    ui_sync::show_toast(&ui, tip, true);
                }
                return;
            }
            // 清理该分类所有项目的图标缓存
            for it in &cat_items {
                state.icon_cache.delete_cache(it);
            }
            state.save_if_modified();
            // 若删除的是当前分类，切换到第一个分类
            if state.current_category_id == cat_id {
                state.current_category_id = state
                    .manager
                    .categories()
                    .first()
                    .map(|c| c.id.clone())
                    .unwrap_or_default();
            }
            let cur = state.current_category_id.clone();
            let q = state.search_query.clone();
            drop(state);
            if let Some(ui) = weak_ui_confirm.upgrade() {
                ui_sync::refresh_categories_ui(&ui, &state_confirm.borrow());
                ui_sync::refresh_items_ui(&ui, &state_confirm.borrow(), &cur, &q);
            }
            return;
        }
        let id = pd_ok.borrow_mut().take();
        if let Some(id) = id {
            let mut state = state_confirm.borrow_mut();
            if let Some(item) = state.manager.get_item(&id) {
                state.icon_cache.delete_cache(item);
            }
            if state.manager.delete_item(&id) {
                state.save_if_modified();
                let cat_id = state.current_category_id.clone();
                let query = state.search_query.clone();
                drop(state);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    // 条目数变化影响侧栏分类计数，需与条目列表一并刷新
                    ui_sync::refresh_categories_ui(&ui, &state_confirm.borrow());
                    ui_sync::refresh_items_ui(&ui, &state_confirm.borrow(), &cat_id, &query);
                }
                // F8/4.1：删除条目后重注册条目热键
                ui_sync::sync_item_hotkeys(&state_confirm.borrow());
            }
            // 删除项处理完毕后必须返回，避免与后续提权/运行全部 pending 级联执行
            return;
        }
        // 提权运行确认（右键菜单 → 确认弹窗 → 执行）
        if let Some(id) = pra_admin_ok.borrow_mut().take() {
            let (item, search_query) = {
                let state = state_confirm.borrow();
                (
                    state.manager.get_item(&id).cloned(),
                    state.search_query.clone(),
                )
            };
            if let Some(item) = item {
                let result = crate::runner::run_as_admin(&item, &search_query);
                if result.success {
                    let mut state = state_confirm.borrow_mut();
                    state.manager.increment_run_count(&item.id);
                    state.save_if_modified();
                } else {
                    log::error!(
                        "Failed to launch as admin {}: {}",
                        item.name,
                        result.error_message
                    );
                    let msg = {
                        let loc_m = state_confirm.borrow().loc.clone();
                        let err_text = ui_sync::run_error_text(&loc_m, &result.error);
                        util::tr_fill(
                            &loc_m,
                            "Status_RunFail",
                            &format!("{}: {}", item.name, err_text),
                        )
                    };
                    let weak = weak_ui_confirm.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui_sync::show_toast(&ui, msg, true);
                        }
                    });
                }
            } else {
                // P5：项目已被删除时静默失败会误导用户，状态栏提示。
                log::warn!("Run-as-admin: item {} not found", id);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    let loc_m = state_confirm.borrow().loc.clone();
                    let msg = loc_m.borrow().tr("Tip_ItemNotFound").to_string();
                    ui_sync::show_toast(&ui, msg, true);
                }
            }
            return;
        }
        // 运行全部（背景右键菜单 → 确认弹窗 → 逐一执行分类内项目）。
        // Timer 分批执行（每批 8 项，批间让出事件循环），避免大分类同步循环冻结 UI。
        if let Some(cat_id) = pra_ok.borrow_mut().take() {
            let items: Vec<crate::model::Item> = {
                let state = state_confirm.borrow();
                state.manager.items(&cat_id).to_vec()
            };
            let search_query = state_confirm.borrow().search_query.clone();
            if !items.is_empty() {
                // 进度状态：(下一批起始索引, 成功数, 失败数)
                let progress = Rc::new(RefCell::new((0usize, 0i32, 0i32)));
                let weak = weak_ui_confirm.clone();
                let state_r = state_confirm.clone();
                let search_q = search_query.clone();
                // slot 持有 Timer；回调尾部 take()+stop() 打破 闭包↔Timer 循环引用。
                // 安全性：slint 事件循环触发回调期间持有 TimerInner 引用，
                // 回调内 drop 我们的 Rc<Timer> 不会释放正在执行的闭包。
                let timer_slot: Rc<RefCell<Option<Rc<slint::Timer>>>> =
                    Rc::new(RefCell::new(Some(Rc::new(slint::Timer::default()))));
                let slot_cb = timer_slot.clone();
                if let Some(timer) = timer_slot.borrow().as_ref() {
                    timer.clone().start(
                        slint::TimerMode::Repeated,
                        std::time::Duration::from_millis(0),
                        move || {
                            const BATCH: usize = 8;
                            let mut pr = progress.borrow_mut();
                            let mut state = state_r.borrow_mut();
                            let end = (pr.0 + BATCH).min(items.len());
                            for it in &items[pr.0..end] {
                                // 尊重项目"管理员运行"标志，与双击/回车行为一致（P0）
                                let result = if it.run_as_admin {
                                    crate::runner::run_as_admin(it, &search_q)
                                } else {
                                    crate::runner::run(it, &search_q)
                                };
                                if result.success {
                                    pr.1 += 1;
                                    state.manager.increment_run_count(&it.id);
                                } else {
                                    pr.2 += 1;
                                    log::warn!(
                                        "Run all: failed to run {} ({}) - {}",
                                        it.name,
                                        it.id,
                                        result.error_message
                                    );
                                }
                            }
                            pr.0 = end;
                            let (next, success, failed) = *pr;
                            // 每批落盘（"每次操作后立即保存"），也防中途中断丢计数
                            state.save_if_modified();
                            drop(state);
                            if next >= items.len() {
                                // 完成：打破 Timer 循环引用，再汇总提示
                                if let Some(t) = slot_cb.borrow_mut().take() {
                                    t.stop();
                                }
                                drop(pr);
                                let msg = {
                                    let loc = state_r.borrow().loc.clone();
                                    let loc_b = loc.borrow();
                                    if failed == 0 {
                                        loc_b
                                            .tr("Status_RunAllDone")
                                            .replace("{0}", &success.to_string())
                                    } else {
                                        loc_b
                                            .tr("Status_RunAllDonePartial")
                                            .replace("{0}", &success.to_string())
                                            .replace("{1}", &failed.to_string())
                                    }
                                };
                                if let Some(ui) = weak.upgrade() {
                                    ui_sync::show_toast(&ui, msg, failed > 0);
                                }
                            }
                        },
                    );
                }
            }
        }
    });

    let state_confirm_cancel = pending_delete.clone();
    let pra_cancel = pending_run_all.clone();
    let pra_admin_cancel = pending_run_admin.clone();
    ui.on_confirm_dialog_cancel(move || {
        state_confirm_cancel.borrow_mut().take();
        pending_category_delete.borrow_mut().take();
        pra_cancel.borrow_mut().take();
        pra_admin_cancel.borrow_mut().take();
    });

    // 分类重命名确认（InputDialog）
    let pcr_ok = pending_category_rename.clone();
    let input_mode_ok = input_mode.clone();
    let state_input = state.clone();
    let weak_ui_input = ui.as_weak();
    ui.on_input_dialog_ok(move || {
        // 快捷键设置模式（菜单入口）：校验 → 重注册 → 持久化
        if input_mode_ok.get() == 2 {
            let new_hk = weak_ui_input
                .upgrade()
                .map(|ui| ui.get_input_dialog_text().to_string())
                .unwrap_or_default();
            if hotkey::parse_hotkey_string(&new_hk).is_none() {
                let loc = state_input.borrow().loc.clone();
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui.set_input_dialog_error(loc.borrow().tr("Tip_InvalidHotkey").into());
                    ui.set_input_dialog_visible(true);
                }
                return;
            }
            // 当前已注册快捷键（改键失败时回滚用）
            let old_hk = state_input
                .borrow()
                .manager
                .config()
                .config
                .global_hotkey
                .clone();
            // 重注册需保留 notifier：unregister 后复用原 Sender 重新 spawn pump
            let ok = {
                let mut mgr = hotkey_mgr.borrow_mut();
                if !mgr.unregister() {
                    log::warn!("Hotkey unregister failed, attempting re-register anyway");
                }
                match hk_tx_shared.borrow().clone() {
                    Some(tx) => mgr.register_with_notifier(&new_hk, tx),
                    None => false,
                }
            };
            if ok {
                input_mode_ok.set(0);
                let mut s = state_input.borrow_mut();
                s.manager.config_mut().config.global_hotkey = new_hk;
                s.save_if_modified();
                log::info!(
                    "Global hotkey updated to: {}",
                    s.manager.config().config.global_hotkey
                );
            } else {
                // P1-7：新键注册失败 → 回滚重注册旧键，避免热键静默失效
                let rollback_ok = {
                    let mut mgr = hotkey_mgr.borrow_mut();
                    if !mgr.unregister() {
                        log::warn!("Hotkey unregister failed during rollback");
                    }
                    match hk_tx_shared.borrow().clone() {
                        Some(tx) => mgr.register_with_notifier(&old_hk, tx),
                        None => false,
                    }
                };
                if !rollback_ok {
                    log::error!("Hotkey rollback failed, old hotkey lost: {}", old_hk);
                }
                log::warn!("Hotkey re-registration failed for: {}", new_hk);
                let loc = state_input.borrow().loc.clone();
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui.set_input_dialog_error(loc.borrow().tr("Tip_HotkeyRegisterFail").into());
                    ui.set_input_dialog_visible(true);
                }
            }
            return;
        }
        // 分类重命名模式
        let cat_id = pcr_ok.borrow_mut().take();
        if let Some(cat_id) = cat_id {
            let mut state = state_input.borrow_mut();
            if let Some(cat) = state
                .manager
                .config_mut()
                .data
                .categories
                .iter_mut()
                .find(|c| c.id == cat_id)
            {
                let name = weak_ui_input
                    .upgrade()
                    .map(|ui| ui.get_input_dialog_text().to_string())
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    // 空名：保持对话框打开并提示（slint confirm 已先行关闭，此处重新显示）
                    let loc = state.loc.clone();
                    if let Some(ui) = weak_ui_input.upgrade() {
                        ui.set_input_dialog_error(loc.borrow().tr("Tip_NameEmpty").into());
                        ui.set_input_dialog_visible(true);
                    }
                    return;
                }
                cat.name = name.trim().to_string();
                state.save_if_modified();
                let cur = state.current_category_id.clone();
                let q = state.search_query.clone();
                drop(state);
                input_mode_ok.set(0);
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui_sync::refresh_categories_ui(&ui, &state_input.borrow());
                    ui_sync::refresh_items_ui(&ui, &state_input.borrow(), &cur, &q);
                }
            }
        }
    });
    let input_mode_cancel = input_mode.clone();
    ui.on_input_dialog_cancel(move || {
        input_mode_cancel.set(0);
        pending_category_rename.borrow_mut().take();
    });

    ui.on_background_clicked(move || {
        log::debug!("Background clicked");
    });

    // 日志档位切换（设置对话框）：持久化 + 运行时即时生效
    let state_loglvl = state.clone();
    let weak_ui_loglvl = ui.as_weak();
    ui.on_settings_log_changed(move |index| {
        let (level, level_name) = match index {
            1 => ("info", "SI_Log_Std"),
            2 => ("debug", "SI_Log_Dbg"),
            _ => ("warn", "SI_Log_Min"),
        };
        {
            let mut st = state_loglvl.borrow_mut();
            st.manager
                .config_mut()
                .extra_config
                .insert("log_level".into(), level.into());
            st.save_if_modified();
        }
        crate::logger::apply_level(level);
        log::info!("Log level changed to {}", level);
        if let Some(ui) = weak_ui_loglvl.upgrade() {
            let loc = state_loglvl.borrow().loc.clone();
            let name = loc.borrow().tr(level_name).to_string();
            let msg = util::tr_fill(&loc, "Status_LogLevelSet", &name);
            ui_sync::show_toast(&ui, msg, false);
        }
    });

    // Escape：清搜索并隐藏窗口（P2 #11）
    let state_esc = state.clone();
    let weak_ui_esc = ui.as_weak();
    ui.on_escape_pressed(move || {
        log::debug!("Escape pressed");
        if let Some(ui) = weak_ui_esc.upgrade() {
            ui.set_search_text("".into());
            let mut state = state_esc.borrow_mut();
            state.search_query = String::new();
            let cat = state.current_category_id.clone();
            drop(state);
            ui_sync::refresh_items_ui(&ui, &state_esc.borrow(), &cat, "");
            // 隐藏前强制落盘几何：托盘退出等路径不会走 on_close_requested
            geometry::flush_window_geometry(&ui, &state_esc);
            let _ = ui.hide();
        }
    });
}
