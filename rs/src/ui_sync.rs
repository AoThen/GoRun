//! UI 模型同步/刷新（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::icon_cache::IconCache;
use crate::localization::Localization;
use crate::model::{Item, RunError};
use crate::util;
use crate::{CategoryModel, EditDialog, ItemModel, MainWindow};
use slint::ComponentHandle;

pub fn spawn_icon_refresh(ui: &MainWindow, icon_cache: IconCache, items: Vec<Item>) {
    if items.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        for item in &items {
            if let Some(path) = icon_cache.refresh_icon(item) {
                let weak = weak.clone();
                let id = item.id.clone();
                let path_str = path.to_string_lossy().to_string();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        use slint::Model;
                        let model = ui.get_items();
                        for i in 0..model.row_count() {
                            if let Some(row) = model.row_data(i) {
                                if row.id.as_str() == id.as_str() {
                                    let mut new_row = row;
                                    new_row.icon_path =
                                        slint::Image::load_from_path(std::path::Path::new(&path_str))
                                            .unwrap_or_default();
                                    model.set_row_data(i, new_row);
                                    break;
                                }
                            }
                        }
                    }
                })
                .ok();
            }
        }
    });
}

/// 将 RunError 映射为本地化错误文案（P1-4：显示处本地化，替代 runner 内硬编码中文）
pub fn run_error_text(loc: &Rc<RefCell<Localization>>, error: &RunError) -> String {
    let key = match error {
        RunError::None => "Err_None",
        RunError::FileNotFound => "Err_FileNotFound",
        RunError::PathNotFound => "Err_PathNotFound",
        RunError::AccessDenied => "Err_AccessDenied",
        RunError::OutOfMemory => "Err_OutOfMemory",
        RunError::DllNotFound => "Err_DllNotFound",
        RunError::Cancelled => "Err_Cancelled",
        RunError::UnsafeSearchParam => "Err_UnsafeSearchParam",
        RunError::Unknown => "Err_Unknown",
    };
    loc.borrow().tr(key).to_string()
}

/// 运行指定索引的项目（双击/回车共用）；搜索态按搜索结果取项，错误写状态栏
pub fn run_item_at_index(
    weak: &slint::Weak<MainWindow>,
    state: &Rc<RefCell<AppState>>,
    index: i32,
) {
    let st = state.borrow();
    let items = if st.search_query.is_empty() {
        st.manager.items(&st.current_category_id)
    } else {
        st.manager.search_items(&st.search_query)
    };
    let (item, search_query) = match items.get(index as usize) {
        Some(it) => (it.clone(), st.search_query.clone()),
        None => return,
    };
    let loc = st.loc.clone();
    drop(st);
    // P0-1：勾选"管理员运行"的项目，双击/回车直接走提权路径（勾选即意图，弹 UAC）
    let result = if item.run_as_admin {
        crate::runner::run_as_admin(&item, &search_query)
    } else {
        crate::runner::run(&item, &search_query)
    };
    if result.success {
        log::info!("Launched: {}", item.name);
        let mut s = state.borrow_mut();
        s.manager.increment_run_count(&item.id);
        s.save_if_modified();
    } else {
        log::error!("Failed to launch {}: {}", item.name, result.error_message);
        let err_text = run_error_text(&loc, &result.error);
        let msg = util::tr_fill(&loc, "Status_RunFail", &format!("{}: {}", item.name, err_text));
        let weak2 = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak2.upgrade() {
                ui.set_status_text(msg.into());
            }
        });
    }
}

pub fn refresh_items_ui(ui: &MainWindow, state: &AppState, category_id: &str, query: &str) {
    let items = if query.is_empty() {
        state.manager.items(category_id)
    } else {
        state.manager.search_items(query)
    };

    let mut pending: Vec<Item> = Vec::new();
    let ui_items: Vec<ItemModel> = items
        .iter()
        .map(|item| {
            let icon_path = if !item.icon_path.is_empty() {
                item.icon_path.clone()
            } else if let Some(p) = state.icon_cache.get_icon_path(item) {
                p.to_string_lossy().to_string()
            } else {
                pending.push(item.clone());
                String::new()
            };
            ItemModel {
                id: item.id.clone().into(),
                name: item.name.clone().into(),
                target: item.target.clone().into(),
                icon_path: if icon_path.is_empty() {
                    slint::Image::default()
                } else {
                    slint::Image::load_from_path(std::path::Path::new(&icon_path))
                        .unwrap_or_default()
                },
                search_active: !query.is_empty(),
            }
        })
        .collect();

    let model = Rc::new(slint::VecModel::from(ui_items));
    ui.set_items(model.into());

    if !pending.is_empty() {
        spawn_icon_refresh(ui, state.icon_cache.clone(), pending);
    }

    let status = util::tr_fill(&state.loc, "Status_Items", &items.len().to_string());
    ui.set_status_text(status.into());
}

pub fn set_edit_dialog_texts(dlg: &EditDialog, loc: &Rc<RefCell<Localization>>) {
    let lb = loc.borrow();
    dlg.set_title_text(lb.tr("Title_EditItem").into());
    dlg.set_label_name(format!("{}:", lb.tr("STR_Name")).into());
    dlg.set_label_target(format!("{}:", lb.tr("STR_Target")).into());
    dlg.set_label_arguments(format!("{}:", lb.tr("STR_Arguments")).into());
    dlg.set_label_directory(format!("{}:", lb.tr("STR_WorkDir")).into());
    dlg.set_label_icon_path(format!("{}:", lb.tr("STR_IconPath")).into());
    dlg.set_label_icon_index(format!("{}:", lb.tr("STR_IconIndex")).into());
    dlg.set_label_keywords(format!("{}:", lb.tr("STR_Keywords")).into());
    dlg.set_label_remark(format!("{}:", lb.tr("STR_Remark")).into());
    dlg.set_label_category(format!("{}:", lb.tr("STR_Category")).into());
    dlg.set_browse_text(format!("{}...", lb.tr("Btn_Browse")).into());
    dlg.set_run_as_admin_text(lb.tr("STR_RunAsAdmin").into());
    dlg.set_save_text(lb.tr("Btn_Save").into());
    dlg.set_cancel_text(lb.tr("Btn_Cancel").into());
}

/// 浏览选择目标后自动回填工作目录（新建时同时填名称）。
pub fn fill_from_target(d: &EditDialog, path: &str, overwrite_name: bool) {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        let dir = parent.to_string_lossy().to_string();
        if !dir.is_empty() {
            d.set_item_working_dir(dir.into());
        }
    }
    if overwrite_name {
        let name = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.is_empty() {
            d.set_item_name(name.into());
        }
    }
}

/// 重建分类侧边栏模型并写回 UI。
pub fn refresh_categories_ui(ui: &MainWindow, state: &AppState) {
    let categories: Vec<CategoryModel> = state
        .manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();
    ui.set_categories(Rc::new(slint::VecModel::from(categories)).into());
    // 侧栏选中态与当前分类保持同步（键盘/程序化变更后刷新）
    if let Some(idx) = state
        .manager
        .categories()
        .iter()
        .position(|c| c.id == state.current_category_id)
    {
        ui.set_sidebar_selected_index(idx as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_category_model() {
        let cat = CategoryModel {
            id: "cat_1".into(),
            name: "Games".into(),
        };
        assert_eq!(cat.id, "cat_1");
        assert_eq!(cat.name, "Games");
    }

    #[test]
    fn test_create_item_model() {
        let item = ItemModel {
            id: "item_1".into(),
            name: "Notepad".into(),
            target: "C:\\Windows\\notepad.exe".into(),
            icon_path: slint::Image::default(),
            search_active: false,
        };
        assert_eq!(item.name, "Notepad");
    }
}
