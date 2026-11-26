//! 项目对话框（新建/编辑）展示逻辑（P1-3 自 main.rs 拆出；P2-5 统一装配）。
//!
//! 新建与编辑共用一套回调装配（wire_shared_callbacks + collect_draft），
//! 仅 on_save 的提交动作不同（add_item vs update_item + 图标缓存处理），
//! 消除此前的双路径重复与行为漂移。

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::handlers::dialogs;
use crate::model::Item;
use crate::{fileops, ui_sync, EditDialog, MainWindow};
use slint::ComponentHandle;

/// 校验新建项目输入。返回错误提示的本地化键（名称空优先于目标空），均合法返回 None。
/// 抽为纯函数便于单测输入校验分支（名称/目标非空校验）。
pub fn validate_item_input(name: &str, target: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        Some("Tip_NameEmpty")
    } else if target.trim().is_empty() {
        Some("Tip_TargetEmpty")
    } else {
        None
    }
}

/// 校验条目热键输入。空串视为未设置（合法）；非空且无法解析时返回错误文案键。
pub fn validate_hotkey_input(hotkey: &str) -> Option<&'static str> {
    if hotkey.trim().is_empty() {
        None
    } else if crate::hotkey::parse_hotkey_string(hotkey).is_none() {
        Some("Tip_InvalidHotkey")
    } else {
        None
    }
}

/// 新建对话框文件选择器文案（标题 + 过滤器），编辑对话框共用。
pub(crate) fn browse_texts(
    loc: &std::rc::Rc<std::cell::RefCell<crate::localization::Localization>>,
    kind: BrowseKind,
) -> (String, String) {
    let lb = loc.borrow();
    match kind {
        BrowseKind::Target => (
            lb.tr("Title_SelectTarget").to_string(),
            format!(
                "{}\0*.*\0{}\0*.exe;*.lnk;*.bat;*.cmd\0\0",
                lb.tr("STR_FilterAll"),
                lb.tr("STR_FilterPrograms")
            ),
        ),
        BrowseKind::Icon => (
            lb.tr("Title_SelectIcon").to_string(),
            format!(
                "{}\0*.ico;*.exe;*.png;*.jpg;*.jpeg;*.bmp\0{}\0*.*\0\0",
                lb.tr("STR_FilterIcons"),
                lb.tr("STR_FilterAll")
            ),
        ),
    }
}

pub(crate) enum BrowseKind {
    Target,
    Icon,
}

/// P2-5 共享装配（新建/编辑同一套，消除双路径漂移）：
/// - browse_target：新建态回填名称+工作目录（overwrite_name=true），编辑态仅工作目录
/// - browse_icon / cancel / close_requested / 实例管理（关旧实例+登记+show）两态完全一致
///
/// on_save 两态提交动作不同（add_item vs update_item），由调用方各自注册。
/// dialog 以值传入（slint 组件不实现 Clone，登记到 AppState 需移动所有权）。
fn wire_shared_callbacks(
    dialog: EditDialog,
    state: &Rc<RefCell<AppState>>,
    overwrite_name: bool,
) {
    let dialog_weak = dialog.as_weak();
    if let Some(dlg_handle) = dialog_weak.upgrade() {
        let loc_browse = state.borrow().loc.clone();
        {
            let dlg_browse = dialog_weak.clone();
            let (t_title, t_filter) = browse_texts(&loc_browse, BrowseKind::Target);
            dlg_handle.on_browse_target(move || {
                if let Some(d) = dlg_browse.upgrade() {
                    if let Some(path) = fileops::browse_file_dialog(&t_title, &t_filter) {
                        d.set_item_target(path.clone().into());
                        ui_sync::fill_from_target(&d, &path, overwrite_name);
                    }
                }
            });
        }
        {
            let dlg_icon = dialog_weak.clone();
            let (i_title, i_filter) = browse_texts(&loc_browse, BrowseKind::Icon);
            dlg_handle.on_browse_icon(move || {
                if let Some(d) = dlg_icon.upgrade() {
                    if let Some(path) = fileops::browse_file_dialog(&i_title, &i_filter) {
                        d.set_item_icon_path(path.into());
                    }
                }
            });
        }
        // 取消：隐藏并清空对话框状态
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

    // P0：打开新对话框前关闭旧实例，避免多实例并存导致状态跟踪不一致
    if let Some(old) = state.borrow_mut().edit_dialog.take() {
        let _ = old.hide();
    }
    state.borrow_mut().edit_dialog = Some(Rc::new(dialog));
    if let Err(e) = state
        .borrow()
        .edit_dialog
        .as_ref()
        .map(|d| d.show())
        .unwrap_or(Ok(()))
    {
        log::error!("Failed to show dialog: {}", e);
    }
    // P2：小屏/高缩放设备上把窗口钳到屏幕可用范围内（scale_factor 在 show 后才可靠），
    // 防止固定 500 逻辑 px 高的窗口被系统截断到工作区、底部按钮行不可见无法保存。
    if let Some(d) = state.borrow().edit_dialog.as_ref() {
        let scale = d.window().scale_factor();
        let size = d.window().size();
        let logical_w = size.width as f32 / scale;
        let logical_h = size.height as f32 / scale;
        let (w, h) = crate::util::clamp_dialog_logical_size(logical_w, logical_h, scale);
        if w < logical_w || h < logical_h {
            d.window().set_size(slint::LogicalSize::new(w, h));
        }
    }
}

/// P2-5 保存时共享逻辑：读取对话框填写的条目字段，输入校验 + 同分类重复
/// target 拦截（均在写入数据前，失败时对话框已显示对应错误提示）。
/// 成功返回完整条目（新建态 id 经 generate_id 生成；编辑态保留原 id），未写任何数据。
fn collect_draft(
    dialog: &EditDialog,
    state: &Rc<RefCell<AppState>>,
    base: &Item,
) -> Option<Item> {
    let name = dialog.get_item_name().to_string();
    let target = dialog.get_item_target().to_string();
    let hotkey = dialog.get_item_hotkey().to_string();
    let err_key =
        validate_item_input(&name, &target).or_else(|| validate_hotkey_input(&hotkey));
    if let Some(err_key) = err_key {
        let msg = state.borrow().loc.borrow().tr(err_key).to_string();
        dialog.set_error_message(msg.into());
        return None;
    }
    // 校验通过：清空残留错误提示（对齐旧双路径行为，防上一轮校验文案残留）
    dialog.set_error_message("".into());
    let cat_idx = dialog.get_selected_category_index() as usize;
    let category_id = {
        let st = state.borrow();
        st.manager
            .categories()
            .get(cat_idx)
            // 越界回落：新建态归当前分类；编辑态保留原条目的分类（对齐旧行为）
            .map(|c| c.id.clone())
            .unwrap_or_else(|| {
                if base.id.is_empty() {
                    st.current_category_id.clone()
                } else {
                    base.category_id.clone()
                }
            })
    };
    let mut draft = base.clone();
    draft.id = if base.id.is_empty() {
        crate::model::generate_id("item")
    } else {
        base.id.clone()
    };
    draft.name = name;
    draft.target = target;
    draft.arguments = dialog.get_item_arguments().to_string();
    draft.working_dir = dialog.get_item_working_dir().to_string();
    draft.icon_path = dialog.get_item_icon_path().to_string();
    draft.icon_index = dialog.get_item_icon_index();
    draft.keywords = dialog.get_item_keywords().to_string();
    draft.remark = dialog.get_item_remark().to_string();
    draft.hotkey = hotkey;
    draft.run_as_admin = dialog.get_run_as_admin();
    draft.category_id = category_id;
    // P2-1：同分类重复 target 明确提示（新建拦截静默 add_item 失败；编辑拦截须
    // 在 delete_cache 之前——collect_draft 不触碰缓存，调用方成功返回后再清理）
    let dup = {
        let st = state.borrow();
        st.manager
            .target_exists_in(&draft.category_id, &draft.target, &draft.id)
    };
    if dup {
        dialog.set_error_message(
            state
                .borrow()
                .loc
                .borrow()
                .tr("Tip_DuplicateItem")
                .to_string()
                .into(),
        );
        return None;
    }
    Some(draft)
}

/// 刷新条目 UI + 重注册条目热键（保存成功后的公共收尾，新建/编辑共用）。
fn refresh_after_commit(state: &Rc<RefCell<AppState>>, ui_weak: &slint::Weak<MainWindow>) {
    let (cat_id, query) = {
        let st = state.borrow();
        (st.current_category_id.clone(), st.search_query.clone())
    };
    if let Some(ui) = ui_weak.upgrade() {
        ui_sync::refresh_items_ui(&ui, &state.borrow(), &cat_id, &query);
    }
    // F8/4.1：条目增改后重注册条目热键；失败列表非空时提示冲突
    let hotkey_failed = ui_sync::sync_item_hotkeys(&state.borrow());
    if let (Some(ui), false) = (ui_weak.upgrade(), hotkey_failed.is_empty()) {
        let loc = state.borrow().loc.clone();
        let msg =
            crate::util::tr_fill(&loc, "Status_HotkeyConflict", &hotkey_failed.join(", "));
        ui_sync::show_toast(&ui, msg, true);
    }
}

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
        dialog.set_item_hotkey("".into());
        dialog.set_category_names(Rc::new(slint::VecModel::from(categories)).into());
        dialog.set_selected_category_index(selected_cat_idx as i32);
        let loc_new = state.borrow().loc.clone();
        ui_sync::set_edit_dialog_texts(&dialog, &loc_new);
        dialogs::setup_edit_dialog_new_category(&dialog, state.clone(), ui_weak);

        let dialog_weak = dialog.as_weak();
        if let Some(dlg_handle) = dialog_weak.upgrade() {
            let dialog_weak_save = dialog_weak.clone();
            let state_save = state.clone();
            let weak_ui_save = ui_weak.clone();
            // 新建提交：collect_draft 生成 id → add_item（仅手动新建拦截重复 target，
            // 拖放路径的幂等跳过语义不受影响，见 add_item 注释）
            dlg_handle.on_save(move || {
                let dialog_strong = dialog_weak_save.upgrade();
                let dlg = match &dialog_strong {
                    Some(d) => d,
                    None => {
                        log::error!("Edit dialog no longer exists");
                        return;
                    }
                };
                let draft = match collect_draft(dlg, &state_save, &Item::default()) {
                    Some(d) => d,
                    None => return,
                };
                // 借用块内完成写入，块尾自动释放（后续 refresh/收尾需再次借用）
                let added = {
                    let mut st = state_save.borrow_mut();
                    if st.manager.add_item(draft) {
                        st.save_if_modified();
                        true
                    } else {
                        false
                    }
                };
                if added {
                    refresh_after_commit(&state_save, &weak_ui_save);
                }
                let _ = dlg.hide();
                state_save.borrow_mut().edit_dialog = None;
            });
        }
        wire_shared_callbacks(dialog, state, true);
    }
}

/// P2-5 展示编辑项目对话框（预填当前字段，保存时更新原条目）。
/// 须在调用方释放对状态的独占借用后调用（内部按需借用），条目以引用传入。
pub fn show_edit_dialog_for_item(
    ui_weak: &slint::Weak<MainWindow>,
    state: &Rc<RefCell<AppState>>,
    item: &Item,
) {
    let (categories, selected_cat_idx) = {
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
            .position(|c| c.id == item.category_id)
            .unwrap_or(0);
        (cats, idx)
    };

    if ui_weak.upgrade().is_none() {
        return;
    }
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
    dialog.set_category_names(Rc::new(slint::VecModel::from(categories)).into());
    dialog.set_selected_category_index(selected_cat_idx as i32);
    let loc_edit = state.borrow().loc.clone();
    ui_sync::set_edit_dialog_texts(&dialog, &loc_edit);
    dialogs::setup_edit_dialog_new_category(&dialog, state.clone(), ui_weak);

    let dialog_weak = dialog.as_weak();
    if let Some(dlg_handle) = dialog_weak.upgrade() {
        let dialog_weak_save = dialog_weak.clone();
        let state_edit = state.clone();
        let weak_ui_edit = ui_weak.clone();
        let item_for_save = item.clone();
        // 编辑提交：collect_draft 保留原 id → 清旧图标缓存与防抖标记 → update_item
        // （重复 target 已在 collect_draft 内拦截，不会误删旧图标缓存）
        dlg_handle.on_save(move || {
            let dialog_strong = dialog_weak_save.upgrade();
            let dlg = match &dialog_strong {
                Some(d) => d,
                None => {
                    log::error!("Edit dialog no longer exists");
                    return;
                }
            };
            let mut draft = match collect_draft(dlg, &state_edit, &item_for_save) {
                Some(d) => d,
                None => return,
            };
            // 借用块内完成缓存清理与更新，块尾自动释放（后续收尾需再次借用）
            let updated = {
                let mut st = state_edit.borrow_mut();
                st.icon_cache.delete_cache(&item_for_save);
                // P1-2：编辑可能改变图标源（新键），清除提取防抖标记允许重新入队提取
                {
                    let mut attempted = st
                        .icon_extract_attempted
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    let _ = attempted.remove(&item_for_save.id);
                }
                // M4：编辑可能改变自定义图标源，同步清除解码失败标记
                {
                    let mut failed = st
                        .icon_custom_decode_failed
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    let _ = failed.remove(&item_for_save.id);
                }
                // P0：图标源（icon_path/icon_index/target）可能已变，清空旧缓存路径，
                // 交由 refresh_items_ui 重新提取并回写，否则继续显示编辑前的旧图标
                draft.cached_icon = String::new();
                if st.manager.update_item(&draft) {
                    st.save_if_modified();
                    true
                } else {
                    false
                }
            };
            if updated {
                refresh_after_commit(&state_edit, &weak_ui_edit);
            }
            let _ = dlg.hide();
            state_edit.borrow_mut().edit_dialog = None;
        });
    }
    wire_shared_callbacks(dialog, state, false);
}

#[cfg(test)]
mod tests {
    use super::{validate_hotkey_input, validate_item_input};

    #[test]
    fn test_validate_both_empty_reports_name() {
        assert_eq!(validate_item_input("", ""), Some("Tip_NameEmpty"));
    }

    #[test]
    fn test_validate_name_whitespace_only() {
        assert_eq!(validate_item_input("  ", "target"), Some("Tip_NameEmpty"));
    }

    #[test]
    fn test_validate_target_empty_reports_target() {
        assert_eq!(validate_item_input("name", ""), Some("Tip_TargetEmpty"));
        assert_eq!(validate_item_input("name", "   "), Some("Tip_TargetEmpty"));
    }

    #[test]
    fn test_validate_name_empty_takes_priority_over_target() {
        // 名称空优先于目标空（对齐原 if name.empty || target.empty 后 name 判定）
        assert_eq!(validate_item_input("", ""), Some("Tip_NameEmpty"));
        assert_eq!(validate_item_input("", "  "), Some("Tip_NameEmpty"));
    }

    #[test]
    fn test_validate_valid() {
        assert_eq!(validate_item_input("name", "target"), None);
        assert_eq!(validate_item_input(" name ", " target "), None);
    }

    #[test]
    fn test_validate_hotkey_empty_is_valid() {
        // 空串/纯空白 = 未设置热键，合法
        assert_eq!(validate_hotkey_input(""), None);
        assert_eq!(validate_hotkey_input("   "), None);
    }

    #[test]
    fn test_validate_hotkey_valid_formats() {
        assert_eq!(validate_hotkey_input("Ctrl+/"), None);
        assert_eq!(validate_hotkey_input("Ctrl+Alt+M"), None);
        assert_eq!(validate_hotkey_input("F8"), None);
        assert_eq!(validate_hotkey_input("ctrl+shift+p"), None);
        assert_eq!(validate_hotkey_input("Alt+1"), None);
    }

    #[test]
    fn test_validate_hotkey_invalid_formats() {
        // 修饰键不在末位
        assert_eq!(validate_hotkey_input("Ctrl"), Some("Tip_InvalidHotkey"));
        // 未知键名
        assert_eq!(validate_hotkey_input("Ctrl+Foo"), Some("Tip_InvalidHotkey"));
        // 空段（尾随 +）
        assert_eq!(validate_hotkey_input("Ctrl+"), Some("Tip_InvalidHotkey"));
        assert_eq!(validate_hotkey_input("+"), Some("Tip_InvalidHotkey"));
        // 非 ASCII 单字符
        assert_eq!(validate_hotkey_input("中"), Some("Tip_InvalidHotkey"));
    }
}
