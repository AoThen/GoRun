//! 编辑对话框辅助（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::model::{Category, ViewType};
use crate::storage::Storage;
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
                    view_type: ViewType::Icon,
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

/// 信任导入外部配置：从隔离备份恢复分类与项目数据，补写 generated_by 标记并落盘。
/// 仅导入数据（categories/items/version），保留当前窗口/热键/语言等会话偏好。
pub fn import_external_config(
    state: &Rc<RefCell<AppState>>,
    weak_ui: &slint::Weak<MainWindow>,
    backup: &str,
) -> bool {
    let imported = match Storage::new(backup.to_string()).load() {
        Some(cfg) => cfg,
        None => {
            log::error!("Failed to parse external config backup: {}", backup);
            return false;
        }
    };
    let new_current = imported
        .data
        .categories
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();
    let mut st = state.borrow_mut();
    {
        let cfg = st.manager.config_mut();
        cfg.version = imported.version;
        cfg.data = imported.data;
        // 保留当前语言/跟随鼠标等偏好，仅补写信任标记（值与主流程一致）
        cfg.extra_config
            .insert("generated_by".to_string(), "gorun-rs".to_string());
    }
    st.current_category_id = new_current;
    st.save_if_modified();
    let cur = st.current_category_id.clone();
    drop(st);
    // 导入成功后移除隔离备份，避免磁盘残留
    if let Err(e) = std::fs::remove_file(backup) {
        log::warn!("Failed to remove external config backup {}: {}", backup, e);
    }
    if let Some(ui) = weak_ui.upgrade() {
        crate::ui_sync::refresh_categories_ui(&ui, &state.borrow());
        crate::ui_sync::refresh_items_ui(&ui, &state.borrow(), &cur, "");
    }
    log::info!("External config imported: {}", backup);
    true
}
