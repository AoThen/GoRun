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

pub fn spawn_icon_refresh(
    ui: &MainWindow,
    icon_cache: IconCache,
    items: Vec<Item>,
    pending_icon_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
) {
    if items.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        // P1：单线程内预收集所有 (id, path) 更新，避免每项一次 invoke_from_event_loop
        // 且每次 O(N) 扫描模型（旧实现总复杂度 O(N²)，大分类时卡顿 UI）。
        let mut updates: Vec<(String, String)> = Vec::new();
        for item in &items {
            if let Some(path) = icon_cache.refresh_icon(item) {
                updates.push((item.id.clone(), path.to_string_lossy().to_string()));
            }
        }
        if updates.is_empty() {
            return;
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                use slint::Model;
                // P0：先在事件循环线程登记缓存路径（供 Timer 回写并落盘），
                // 否则进程重启后图标无从恢复（item_keys 只是内存映射）。
                if let Ok(mut queue) = pending_icon_paths.lock() {
                    for (id, path) in &updates {
                        queue.insert(id.clone(), path.clone());
                    }
                }
                let model = ui.get_items();
                // 构建 id -> path 映射做 O(1) 查找，单次遍历模型（O(N)）完成全部更新。
                let id_to_path: std::collections::HashMap<&str, &str> = updates
                    .iter()
                    .map(|(id, path)| (id.as_str(), path.as_str()))
                    .collect();
                for i in 0..model.row_count() {
                    if let Some(row) = model.row_data(i) {
                        if let Some(path_str) = id_to_path.get(row.id.as_str()) {
                            let mut new_row = row;
                            new_row.icon_path =
                                slint::Image::load_from_path(std::path::Path::new(path_str))
                                    .unwrap_or_default();
                            model.set_row_data(i, new_row);
                        }
                    }
                }
            }
        })
        .ok();
    });
}

/// 把后台提取出的图标缓存路径回写到项目模型并落盘（P0）。
/// 必须调用方持有主线程状态；提取线程只负责产出路径，写模型留在事件循环内。
pub fn persist_cached_icons(
    slot: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    state: &Rc<RefCell<AppState>>,
) {
    let updates: Vec<(String, String)> = {
        let mut map = match slot.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        if map.is_empty() {
            return;
        }
        map.drain().collect()
    };
    let mut st = state.borrow_mut();
    let mut changed = false;
    for (item_id, path) in updates {
        if st.manager.set_cached_icon(&item_id, &path) {
            changed = true;
        }
    }
    if changed {
        st.save_if_modified();
    }
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

/// F8/4.1：按 item_id 运行条目（条目级全局热键触发）。搜索参数为空（热键无搜索语境）。
pub fn run_item_by_id(state: &Rc<RefCell<AppState>>, item_id: &str) {
    let item = {
        let st = state.borrow();
        st.manager
            .all_items()
            .iter()
            .find(|i| i.id == item_id)
            .cloned()
    };
    let item = match item {
        Some(i) => i,
        None => {
            log::warn!("Item hotkey: item {} not found", item_id);
            return;
        }
    };
    let result = if item.run_as_admin {
        crate::runner::run_as_admin(&item, "")
    } else {
        crate::runner::run(&item, "")
    };
    if result.success {
        log::info!("Item hotkey launched: {}", item.name);
        let mut s = state.borrow_mut();
        s.manager.increment_run_count(&item.id);
        s.save_if_modified();
    } else {
        log::error!(
            "Item hotkey run failed for {}: {}",
            item.name,
            result.error_message
        );
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
            // P0：优先用户自定义图标源 → 已落盘的提取缓存 → 异步提取（并回写缓存路径）
            let icon_path = if !item.icon_path.is_empty() {
                item.icon_path.clone()
            } else if let Some(p) = state.icon_cache.restored_icon_path(item) {
                p.to_string_lossy().to_string()
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
        spawn_icon_refresh(
            ui,
            state.icon_cache.clone(),
            pending,
            state.pending_icon_paths.clone(),
        );
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
    dlg.set_label_hotkey(format!("{}:", lb.tr("STR_HotKey")).into());
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

/// F8/4.1：同步条目级全局热键——收集所有非空 item.hotkey，重注册到 ItemHotkeyManager。
/// 条目增删改/导入后调用；空列表时注销全部（无条目热键）。
pub fn sync_item_hotkeys(state: &AppState) {
    let hotkeys: Vec<(String, String)> = state
        .manager
        .all_items()
        .iter()
        .filter(|i| !i.hotkey.trim().is_empty())
        .map(|i| (i.id.clone(), i.hotkey.clone()))
        .collect();
    let tx = state.item_hotkey_tx.clone();
    let ok = state.item_hotkey_mgr.borrow_mut().register_all(&hotkeys, tx);
    if !ok {
        log::warn!("Item hotkey registration reported failure (partial)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_error_text_maps_each_error_to_localized_key() {
        let loc = Rc::new(RefCell::new(Localization::new("zh-CN")));
        assert_eq!(run_error_text(&loc, &RunError::None), "成功");
        assert_eq!(run_error_text(&loc, &RunError::FileNotFound), "找不到指定的文件");
        assert_eq!(run_error_text(&loc, &RunError::PathNotFound), "找不到指定的路径");
        assert_eq!(run_error_text(&loc, &RunError::AccessDenied), "访问被拒绝");
        assert_eq!(run_error_text(&loc, &RunError::OutOfMemory), "内存不足");
        assert_eq!(
            run_error_text(&loc, &RunError::DllNotFound),
            "找不到指定的 DLL 文件"
        );
        assert_eq!(run_error_text(&loc, &RunError::Cancelled), "已取消提权请求");
        assert_eq!(
            run_error_text(&loc, &RunError::UnsafeSearchParam),
            "搜索参数含不安全命令字符，已拒绝执行"
        );
        assert_eq!(run_error_text(&loc, &RunError::Unknown), "未知错误");
    }

    #[test]
    fn test_run_error_text_changes_with_language() {
        // 语言切换影响输出（行为测试，非实现自证）
        let zh = Rc::new(RefCell::new(Localization::new("zh-CN")));
        let en = Rc::new(RefCell::new(Localization::new("en-US")));
        assert_eq!(run_error_text(&zh, &RunError::FileNotFound), "找不到指定的文件");
        assert_eq!(run_error_text(&en, &RunError::FileNotFound), "File not found");
    }
}
