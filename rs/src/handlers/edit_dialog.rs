//! 新建项目对话框展示逻辑（P1-3 自 main.rs 拆出）
//!
//! 仅承载"新建项目"流程。编辑项目仍由 wire_handler 的项目菜单内联处理
//! （因编辑态需在持有 RefMut 的 match 分支内就地构造对话框，拆出需重构借用结构）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::handlers::dialogs;
use crate::model::Item;
use crate::{fileops, ui_sync, EditDialog, MainWindow};
use slint::ComponentHandle;

/// 展示新建项目对话框（名称预填"新建项目"，保存时新增）。
pub fn show_edit_dialog(ui_weak: &slint::Weak<MainWindow>, state: &Rc<RefCell<AppState>>) {
    let (categories, selected_cat_idx, default_name) = {
        let st = state.borrow();
        let cats: Vec<slint::SharedString> = st
            .manager
            .categories()
            .iter()
            .map(|c| c.name.clone().into())
            .collect();
        let idx = st
            .manager
            .categories()
            .iter()
            .position(|c| c.id == st.current_category_id)
            .unwrap_or(0);
        let default_name = st.loc.borrow().tr("Menu_NewItem").to_string();
        (cats, idx, default_name)
    };

    if ui_weak.upgrade().is_some() {
        let dialog = match EditDialog::new() {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to create edit dialog: {}", e);
                return;
            }
        };

        dialog.set_item_name(default_name.into());
        dialog.set_category_names(Rc::new(slint::VecModel::from(categories)).into());
        dialog.set_selected_category_index(selected_cat_idx as i32);
        let loc_new = state.borrow().loc.clone();
        ui_sync::set_edit_dialog_texts(&dialog, &loc_new);
        dialogs::setup_edit_dialog_new_category(&dialog, state.clone(), ui_weak);

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
                            // 新建态回填名称+工作目录
                            ui_sync::fill_from_target(&d, &path, true);
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
            let dialog_weak_save = dialog_weak.clone();
            let state_save = state.clone();
            let weak_ui_save = ui_weak.clone();
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
                    let loc = state_save.borrow().loc.clone();
                    let msg = if name.trim().is_empty() {
                        loc.borrow().tr("Tip_NameEmpty").to_string()
                    } else {
                        loc.borrow().tr("Tip_TargetEmpty").to_string()
                    };
                    dlg.set_error_message(msg.into());
                    return;
                }
                dlg.set_error_message("".into());
                let mut state = state_save.borrow_mut();
                let cat_idx = dlg.get_selected_category_index() as usize;
                let category_id = state
                    .manager
                    .categories()
                    .get(cat_idx)
                    .map(|c| c.id.clone())
                    .unwrap_or_else(|| state.current_category_id.clone());
                let new_item = Item {
                    id: crate::model::generate_id("item"),
                    name: dlg.get_item_name().to_string(),
                    target: dlg.get_item_target().to_string(),
                    arguments: dlg.get_item_arguments().to_string(),
                    working_dir: dlg.get_item_working_dir().to_string(),
                    icon_path: dlg.get_item_icon_path().to_string(),
                    icon_index: dlg.get_item_icon_index(),
                    keywords: dlg.get_item_keywords().to_string(),
                    remark: dlg.get_item_remark().to_string(),
                    run_as_admin: dlg.get_run_as_admin(),
                    category_id,
                    ..Default::default()
                };
                if state.manager.add_item(new_item) {
                    state.save_if_modified();
                    let cat_id = state.current_category_id.clone();
                    let query = state.search_query.clone();
                    drop(state);
                    if let Some(ui) = weak_ui_save.upgrade() {
                        ui_sync::refresh_items_ui(&ui, &state_save.borrow(), &cat_id, &query);
                    }
                } else {
                    drop(state);
                }
                let _ = dlg.hide();
                state_save.borrow_mut().edit_dialog = None;
            });

            let dialog_weak_cancel = dialog_weak.clone();
            let state_cancel = state.clone();
            dlg_handle.on_cancel(move || {
                let dialog_strong = dialog_weak_cancel.upgrade();
                if let Some(d) = &dialog_strong {
                    let _ = d.hide();
                }
                state_cancel.borrow_mut().edit_dialog = None;
            });

            // P2-06：标题栏 X 关闭等价取消，清空 edit_dialog 状态（slint 默认关闭=hide）
            let dialog_weak_close = dialog_weak.clone();
            let state_close = state.clone();
            dlg_handle.window().on_close_requested(move || {
                if let Some(d) = dialog_weak_close.upgrade() {
                    let _ = d.hide();
                }
                state_close.borrow_mut().edit_dialog = None;
                slint::CloseRequestResponse::HideWindow
            });
        }

        state.borrow_mut().edit_dialog = Some(dialog);
        if let Err(e) = state
            .borrow()
            .edit_dialog
            .as_ref()
            .map(|d| d.show())
            .unwrap_or(Ok(()))
        {
            log::error!("Failed to show dialog: {}", e);
        }
    }
}
