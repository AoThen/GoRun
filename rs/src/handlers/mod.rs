//! 事件处理器模块（P1-3 自 main.rs 拆出）
//!
//! 按职责分组：
//! - `dialogs`：编辑对话框辅助（分类新增按钮、外部配置信任导入）
//! - `edit_dialog`：新建/编辑项目对话框的共享展示逻辑
//! - `wire_handler`：主窗口事件回调注册（D3 自 main.rs 迁入）

pub mod dialogs;
pub mod edit_dialog;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::localization::Localization;
use crate::model::{Category, Item, ViewType};
use crate::{
    clipboard, fileops, geometry, hotkey, model, ui_sync, util, EditDialog, MainWindow,
};
use slint::ComponentHandle;

pub fn wire_handler(
    ui: &MainWindow,
    state: Rc<RefCell<AppState>>,
    loc: &Rc<RefCell<Localization>>,
    hotkey_mgr: Rc<RefCell<hotkey::HotkeyManager>>,
    hk_tx_shared: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
    external_backup_path: Option<String>,
) {
    let right_clicked_index: Rc<std::cell::Cell<Option<i32>>> = Rc::new(std::cell::Cell::new(None));
    let pending_delete: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    // 外部配置信任导入：Some(隔离备份路径) 时弹确认框，确认后导入（P0-8）
    let pending_external_trust: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(external_backup_path));
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
        let count = state.manager.handle_drop(&[path], &cat_id);
        state.save_if_modified();
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
        let cat_id = state.current_category_id.clone();
        drop(state);
        let timer = search_timer.clone();
        let weak = weak_ui_search.clone();
        let state_cb = state_search.clone();
        timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(300),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui_sync::refresh_items_ui(&ui, &state_cb.borrow(), &cat_id, &text);
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
    let loc_theme = loc.clone();
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
        drop(state);
        // 主题切换后刷新菜单主题键（本地化最小版）
        if let Some(ui) = weak_ui_theme.upgrade() {
            let key = if dark { "Menu_SwitchLight" } else { "Menu_SwitchDark" };
            ui.set_menu_text_toggle_theme(loc_theme.borrow().tr(key).into());
        }
    });

    // 菜单"快捷键"入口：打开 InputDialog 预填当前快捷键（S4）
    let input_mode_hk = input_mode.clone();
    let state_hk = state.clone();
    let weak_ui_hk = ui.as_weak();
    ui.on_hotkey(move || {
        input_mode_hk.set(2);
        if let Some(ui) = weak_ui_hk.upgrade() {
            let hk = state_hk.borrow().manager.config().config.global_hotkey.clone();
            let title = state_hk.borrow().loc.borrow().tr("STR_HotKey").to_string();
            ui.set_input_dialog_title(title.into());
            ui.set_input_dialog_text(hk.into());
            ui.set_input_dialog_error("".into());
            ui.set_input_dialog_visible(true);
        }
    });

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
        drop(state);
        if let Some(ui) = weak_ui_refresh.upgrade() {
            ui_sync::spawn_icon_refresh(&ui, icon_cache, items);
        }
    });

    // 双击 / 回车（键盘导航）运行指定索引项目
    let state_dbl = state.clone();
    let weak_ui_dbl = ui.as_weak();
    let last_click: Rc<std::cell::Cell<Option<(i32, std::time::Instant)>>> =
        Rc::new(std::cell::Cell::new(None));
    let weak_ui_sel = weak_ui_dbl.clone();
    let state_sel = state_dbl.clone();
    ui.on_item_tile_clicked(move |index| {
        let now = std::time::Instant::now();
        let is_double = match last_click.get() {
            Some((last_idx, last_time))
                if last_idx == index
                    && now.duration_since(last_time).as_millis() < 500 =>
            {
                true
            }
            _ => false,
        };
        if is_double {
            last_click.set(None);
            log::debug!("Item double-clicked: index={}", index);
            ui_sync::run_item_at_index(&weak_ui_dbl, &state_dbl, index);
        } else {
            last_click.set(Some((index, now)));
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
    let rci = right_clicked_index.clone();
    let mk_item = menu_kind.clone();
    let state_search_menu = state.clone();
    let weak_ui_right = ui.as_weak();
    ui.on_item_right_clicked(move |index, x, y| {
        log::debug!("Item right-clicked: index={}", index);
        mk_item.set(0);
        rci.set(Some(index));
        // 搜索态排序语义与全局向量不一致，隐藏 Move Up/Down（建议④）
        let st = state_search_menu.borrow();
        let searching = !st.search_query.is_empty();
        let cat_names: Vec<String> = st.manager.categories().iter().map(|c| c.name.clone()).collect();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        let mut menu_vec = vec![
            slint::SharedString::from(lb.tr("Menu_Edit")),
            slint::SharedString::from(lb.tr("Menu_RunAsAdmin")),
            slint::SharedString::from(lb.tr("Menu_DelItem")),
            slint::SharedString::from(lb.tr("Menu_CopyPath")),
            slint::SharedString::from(lb.tr("Menu_OpenLocation")),
        ];
        if !searching {
            menu_vec.push(slint::SharedString::from(lb.tr("Menu_MoveUp")));
            menu_vec.push(slint::SharedString::from(lb.tr("Menu_MoveDown")));
        }
        menu_vec.push(slint::SharedString::from(lb.tr("Menu_Run")));
        menu_vec.push(slint::SharedString::from(lb.tr("Menu_RefreshIcon")));
        menu_vec.push(slint::SharedString::from(lb.tr("Menu_Properties")));
        let move_to = lb.tr("Menu_MoveTo").to_string();
        drop(lb);
        for name in &cat_names {
            menu_vec.push(slint::SharedString::from(format!("{} {}", move_to, name)));
        }
        let item_count = menu_vec.len();
        let menu_items = Rc::new(slint::VecModel::from(menu_vec));
        if let Some(ui) = weak_ui_right.upgrade() {
            ui.set_context_menu_items(menu_items.into());
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
        let menu_items = Rc::new(slint::VecModel::from(vec![
            slint::SharedString::from(lb.tr("Menu_NewItem")),
            slint::SharedString::from(lb.tr("Menu_NewC")),
            slint::SharedString::from(lb.tr("Menu_RefreshIcon")),
            slint::SharedString::from(lb.tr("Menu_Paste")),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_name)),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_usage)),
            slint::SharedString::from(lb.tr("Menu_RunAll")),
        ]));
        if let Some(ui) = weak_ui_bg.upgrade() {
            ui.set_context_menu_items(menu_items.into());
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
        let menu_items = Rc::new(slint::VecModel::from(vec![
            slint::SharedString::from(lb.tr("Menu_ReName")),
            slint::SharedString::from(lb.tr("Menu_DelC")),
            slint::SharedString::from(lb.tr("Menu_MoveUp")),
            slint::SharedString::from(lb.tr("Menu_MoveDown")),
        ]));
        if let Some(ui) = weak_ui_cat_right.upgrade() {
            ui.set_context_menu_items(menu_items.into());
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
    ui.on_menu_item_selected(move |index| {
        log::debug!("Menu item selected: index={}, kind={}", index, mk_menu.get());

        // 分类菜单（P1-3）：重命名/删除/上移/下移
        if mk_menu.get() == 2 {
            let cat_idx = rcc_menu.get();
            if let Some(idx) = cat_idx {
                match index {
                    0 => {
                        // 重命名：打开 InputDialog
                        let mut state = state_menu.borrow_mut();
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
                        let mut state = state_menu.borrow_mut();
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
                        let count = state.manager.handle_drop(&paths, &cat_id);
                        state.save_if_modified();
                        let q = state.search_query.clone();
                        let loc_m = state.loc.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            ui.set_status_text(
                                util::tr_fill(&loc_m, "Status_Pasted", &count.to_string()).into(),
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
                    let mut state = state_menu.borrow_mut();
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
                // 搜索态隐藏 MoveUp/Down，其 Run/RefreshIcon/Properties/MoveTo 索引前移 2。
                // normal_mode = 查询为空（非搜索态），此时 MoveUp/Down 存在于菜单索引 5/6。
                let normal_mode = state.search_query.is_empty();
                let move_offset = if normal_mode { 2 } else { 0 };
                let run_idx = move_offset + 5;
                let refresh_idx = move_offset + 6;
                let props_idx = move_offset + 7;
                let moveto_base = move_offset + 8;
                match index {
                    0 => {
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

                        if let Some(ui) = weak_ui_menu.upgrade() {
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
                            if let Some(dlg_handle) = dialog_weak.upgrade() {
                                {
                                    let dlg_browse = dialog_weak.clone();
                                    dlg_handle.on_browse_target(move || {
                                        if let Some(d) = dlg_browse.upgrade() {
                                            if let Some(path) = fileops::browse_file_dialog(
                                                "Select Target",
                                                "All Files (*.*)\0*.*\0Programs (*.exe;*.lnk;*.bat;*.cmd)\0*.exe;*.lnk;*.bat;*.cmd\0\0",
                                            ) {
                                                d.set_item_target(path.clone().into());
                                                // 编辑态：仅回填工作目录（不覆盖已有名）
                                                ui_sync::fill_from_target(&d, &path, false);
                                            }
                                        }
                                    });
                                }
                                {
                                    let dlg_icon = dialog_weak.clone();
                                    dlg_handle.on_browse_icon(move || {
                                        if let Some(d) = dlg_icon.upgrade() {
                                            if let Some(path) = fileops::browse_file_dialog(
                                                "Select Icon",
                                                "Icons (*.ico;*.exe;*.png;*.jpg;*.jpeg;*.bmp)\0*.ico;*.exe;*.png;*.jpg;*.jpeg;*.bmp\0All Files (*.*)\0*.*\0\0",
                                            ) {
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
                                    // 输入验证：名称与目标非空（对齐 C++ EditDialog）
                                    let name = dlg.get_item_name().to_string();
                                    let target = dlg.get_item_target().to_string();
                                    if name.trim().is_empty() || target.trim().is_empty() {
                                        let loc = state_edit.borrow().loc.clone();
                                        let msg = if name.trim().is_empty() {
                                            loc.borrow().tr("Tip_NameEmpty").to_string()
                                        } else {
                                            loc.borrow().tr("Tip_TargetEmpty").to_string()
                                        };
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
                                    updated_item.keywords = dlg.get_item_keywords().to_string();
                                    updated_item.remark = dlg.get_item_remark().to_string();
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
                    1 => {
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
                        return;
                    }
                    2 => {
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
                        return;
                    }
                    3 => {
                        clipboard::copy_to_clipboard(&item.target);
                    }
                    4 => {
                        if crate::runner::is_url(&item.target) {
                            log::debug!(
                                "Skip open file location for URL target: {}",
                                item.target
                            );
                        } else {
                            fileops::open_file_location(&item.target);
                        }
                    }
                    x if x == run_idx => {
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
                                    ui.set_status_text(msg.into());
                                }
                            });
                        }
                        return;
                    }
                    x if x == refresh_idx => {
                        log::info!("Refresh icon: {} ({})", item.name, item.id);
                        let icon_cache = state.icon_cache.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::spawn_icon_refresh(&ui, icon_cache, vec![item.clone()]);
                        }
                    }
                    x if x == props_idx => {
                        log::info!("Properties: {} ({})", item.target, item.id);
                        fileops::show_properties(&item.target);
                    }
                    x if x >= moveto_base => {
                        let cat_pos = (x - moveto_base) as usize;
                        let target_cat = {
                            let cats = state.manager.categories();
                            cats.get(cat_pos).map(|c| c.id.clone())
                        };
                        if let Some(tcat) = target_cat {
                            if state.manager.move_item(&item.id, &tcat) {
                                state.save_if_modified();
                                let cat_id = state.current_category_id.clone();
                                let q = state.search_query.clone();
                                drop(state);
                                if let Some(ui) = weak_ui_menu.upgrade() {
                                    ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                                }
                                return;
                            }
                        }
                    }
                    // P4：MoveUp/MoveDown 固定位于索引 5/6（仅非搜索态存在）；
                    // guard 带 normal_mode 使搜索态（索引 5/6 被 Run/Refresh 占用）不误路由。
                    x if x == 5 && normal_mode => {
                        if state.manager.move_item_up(&item.id) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                            return;
                        }
                    }
                    x if x == 6 && normal_mode => {
                        if state.manager.move_item_down(&item.id) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                            return;
                        }
                    }
                    _ => {}
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
    let pet_ok = pending_external_trust.clone();
    ui.on_confirm_dialog_ok(move || {
        // 外部配置信任导入优先（P0-8）
        if let Some(backup) = pet_ok.borrow_mut().take() {
            log::info!("User trusted external config, importing: {}", backup);
            let imported = dialogs::import_external_config(&state_confirm, &weak_ui_confirm, &backup);
            if let Some(ui) = weak_ui_confirm.upgrade() {
                let key = if imported {
                    "Status_ExternalImported"
                } else {
                    "Status_ExternalImportFail"
                };
                let msg = state_confirm.borrow().loc.borrow().tr(key).to_string();
                ui.set_status_text(msg.into());
            }
            return;
        }
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
                    ui.set_status_text(tip.into());
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
                    ui_sync::refresh_items_ui(&ui, &state_confirm.borrow(), &cat_id, &query);
                }
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
                            ui.set_status_text(msg.into());
                        }
                    });
                }
            } else {
                // P5：项目已被删除时静默失败会误导用户，状态栏提示。
                log::warn!("Run-as-admin: item {} not found", id);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    let loc_m = state_confirm.borrow().loc.clone();
                    let msg = loc_m.borrow().tr("Tip_ItemNotFound").to_string();
                    ui.set_status_text(msg.into());
                }
            }
            return;
        }
        // 运行全部（背景右键菜单 → 确认弹窗 → 逐一执行分类内项目）
        if let Some(cat_id) = pra_ok.borrow_mut().take() {
            let state = state_confirm.borrow();
            let items = state.manager.items(&cat_id);
            let search_query = state.search_query.clone();
            drop(state);
            if !items.is_empty() {
                let results = crate::runner::run_all(&items, &search_query);
                let mut state = state_confirm.borrow_mut();
                let mut success = 0i32;
                let mut failed = 0i32;
                for (idx, result) in results.iter().enumerate() {
                    if result.success {
                        success += 1;
                        state.manager.increment_run_count(&items[idx].id);
                    } else {
                        failed += 1;
                    }
                }
                state.save_if_modified();
                drop(state);
                let msg = {
                    let loc = state_confirm.borrow().loc.clone();
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
                let weak = weak_ui_confirm.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_status_text(msg.into());
                    }
                });
            }
        }
    });

    let state_confirm_cancel = pending_delete.clone();
    let pra_cancel = pending_run_all.clone();
    let pra_admin_cancel = pending_run_admin.clone();
    let pet_cancel = pending_external_trust.clone();
    ui.on_confirm_dialog_cancel(move || {
        pet_cancel.borrow_mut().take();
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
            let old_hk = state_input.borrow().manager.config().config.global_hotkey.clone();
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
            let _ = ui.hide();
        }
    });

    // P0-8：检测到外部/未信任配置时弹确认框，信任导入或保持隔离
    // 先 clone 到局部变量，避免 Ref 临时值在 if-let 块结束前被借用检查器认为仍存活（E0597）
    let pending_trust = pending_external_trust.borrow().clone();
    if let Some(backup) = pending_trust {
        ui.set_confirm_dialog_title(loc.borrow().tr("Tip_ExternalConfigTitle").to_string().into());
        // P1-05：追加导入规模计数，帮助用户在盲信导入前了解数据量
        // P0-3：进一步展示目标路径（前 10 个 + 剩余计数），让用户对导入内容知情。
        let cfg_opt = crate::storage::Storage::new(backup).load();
        let count_msg = cfg_opt
            .as_ref()
            .map(|cfg| {
                loc.borrow()
                    .tr("Tip_ExternalConfigCount")
                    .replace("{0}", &cfg.data.categories.len().to_string())
                    .replace("{1}", &cfg.data.items.len().to_string())
            })
            .unwrap_or_default();
        let targets_msg = cfg_opt
            .as_ref()
            .and_then(|cfg| {
                if cfg.data.items.is_empty() {
                    return None;
                }
                const MAX_SHOWN: usize = 10;
                let shown: Vec<&str> = cfg
                    .data
                    .items
                    .iter()
                    .take(MAX_SHOWN)
                    .map(|i| i.target.as_str())
                    .collect();
                let mut lines = shown.join("\n");
                let remaining = cfg.data.items.len().saturating_sub(MAX_SHOWN);
                if remaining > 0 {
                    lines.push_str("\n");
                    lines.push_str(
                        &loc.borrow()
                            .tr("Tip_ExternalConfigMore")
                            .replace("{0}", &remaining.to_string()),
                    );
                }
                Some(
                    loc.borrow()
                        .tr("Tip_ExternalConfigTargets")
                        .replace("{0}", &lines),
                )
            })
            .unwrap_or_default();
        let base_msg = loc.borrow().tr("Tip_ExternalConfigMsg").to_string();
        let mut msg = if count_msg.is_empty() {
            base_msg
        } else {
            format!("{}\n{}", base_msg, count_msg)
        };
        if !targets_msg.is_empty() {
            msg.push('\n');
            msg.push_str(&targets_msg);
        }
        ui.set_confirm_dialog_message(msg.into());
        ui.set_confirm_confirm_text(loc.borrow().tr("Btn_TrustImport").to_string().into());
        ui.set_confirm_dialog_visible(true);
    }
}
