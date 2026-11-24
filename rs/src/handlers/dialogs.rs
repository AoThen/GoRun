//! 编辑对话框辅助（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::model::{Category, ViewType};
use crate::EditDialog;
use crate::MainWindow;
use slint::ComponentHandle;

/// 编辑对话框内"新建分类"按钮回调：创建分类并回填下拉框与侧栏。
pub fn setup_edit_dialog_new_category(
    dialog: &EditDialog,
    state: Rc<RefCell<AppState>>,
    weak_ui: &slint::Weak<MainWindow>,
) {
    let dialog_weak = dialog.as_weak();
    if let Some(dlg_handle) = dialog_weak.upgrade() {
        let state_nc = state.clone();
        let weak_ui_nc = weak_ui.clone();
        dlg_handle.on_new_category(move || {
            if let Some(d) = dialog_weak.upgrade() {
                let loc_nc = state_nc.borrow().loc.clone();
                let mut state = state_nc.borrow_mut();
                let sort_order = state.manager.categories().len() as i32;
                let new_cat = Category {
                    id: crate::model::generate_id("cat"),
                    name: format!(
                        "{} {}",
                        loc_nc.borrow().tr("Default_NewCategory"),
                        sort_order + 1
                    ),
                    sort_order,
                    view_type: ViewType::List,
                    icon_size: 48,
                };
                let cat_id = new_cat.id.clone();
                state.manager.add_category(new_cat);
                state.save_if_modified();
                drop(state);
                let cats: Vec<slint::SharedString> = state_nc
                    .borrow()
                    .manager
                    .categories()
                    .iter()
                    .map(|c| c.name.clone().into())
                    .collect();
                d.set_category_names(Rc::new(slint::VecModel::from(cats)).into());
                let idx = state_nc
                    .borrow()
                    .manager
                    .categories()
                    .iter()
                    .position(|c| c.id == cat_id)
                    .unwrap_or(0) as i32;
                d.set_selected_category_index(idx);
                if let Some(ui) = weak_ui_nc.upgrade() {
                    crate::ui_sync::refresh_categories_ui(&ui, &state_nc.borrow());
                }
            }
        });
    }
}
