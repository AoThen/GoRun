//! 配置导出/导入（F8）：Settings 菜单 → 导出配置 / 导入配置。
//! 导出用存储的 RawFile 序列化（与落盘同格式，含完整性值）；
//! 导入整体替换数据与偏好设置，但保留语言/跟随鼠标/自动隐藏等会话偏好。

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::MainWindow;

/// 导入配置时保留的会话偏好键（语言/跟随鼠标/自动隐藏/信任标记/低驻留）。
/// 这些属于当前会话运行状态，不应被导入文件整体覆盖。
pub(crate) fn session_pref_keys() -> [&'static str; 5] {
    ["language", "follow_mouse", "auto_hide", "generated_by", "low_resident"]
}

/// 从 extra_config 中筛出需保留的会话偏好（纯逻辑，可单测）。
pub(crate) fn filter_session_prefs(
    extra: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let keys = session_pref_keys();
    extra
        .iter()
        .filter(|(k, _)| keys.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Settings 菜单 → 导出配置：用户选目标文件，写当前配置 JSON。
pub fn export_config(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    let Some(path) = crate::fileops::save_file_dialog(
        "Export Config",
        "Config (*.json)\0*.json\0",
    ) else {
        return;
    };
    let st = state.borrow();
    if st.storage.export_to(&path, st.manager.config()) {
        log::info!("Exported config to {}", path);
        crate::ui_sync::show_toast(&ui, format!("Exported: {}", path), false);
    } else {
        log::error!("Failed to export config to {}", path);
        crate::ui_sync::show_toast(&ui, "Export failed", true);
    }
}

/// Settings 菜单 → 导入配置：用户选配置文件，整体替换数据与行为设置，
/// 保留语言/跟随鼠标/自动隐藏等会话偏好；成功后刷新侧栏与条目。
pub fn import_config(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    let Some(path) = crate::fileops::browse_file_dialog(
        "Import Config",
        "Config (*.json)\0*.json\0",
    ) else {
        return;
    };
    let imported = match crate::storage::Storage::new(path.clone()).load() {
        Some(c) => c,
        None => {
            log::error!("Failed to parse import config: {}", path);
            crate::ui_sync::show_toast(&ui, "Import failed: invalid config file", true);
            return;
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
        // 保留会话偏好（语言/跟随鼠标/自动隐藏/信任标记），其余整体替换
        let keep = filter_session_prefs(&cfg.extra_config);
        cfg.version = imported.version;
        cfg.data = imported.data;
        cfg.config = imported.config;
        cfg.extra_config = keep;
    }
    st.current_category_id = new_current;
    let saved = st.save_if_modified();
    let cur = st.current_category_id.clone();
    drop(st);

    if !saved {
        log::error!("Failed to save imported config");
        crate::ui_sync::show_toast(&ui, "Import failed: save error", true);
        return;
    }
    crate::ui_sync::refresh_categories_ui(ui, &state.borrow());
    crate::ui_sync::refresh_items_ui(ui, &state.borrow(), &cur, "");
    // F8/4.1：导入后重注册条目热键（条目集整体变化）
    crate::ui_sync::sync_item_hotkeys(&state.borrow());
    crate::ui_sync::show_toast(&ui, "Imported config", false);
    log::info!("Imported config from {}", path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn hm(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_session_pref_keys_complete() {
        let keys = session_pref_keys();
        assert_eq!(keys.len(), 5);
        assert!(keys.contains(&"language"));
        assert!(keys.contains(&"follow_mouse"));
        assert!(keys.contains(&"auto_hide"));
        assert!(keys.contains(&"generated_by"));
        assert!(keys.contains(&"low_resident"));
    }

    #[test]
    fn test_filter_keeps_only_session_prefs() {
        let extra = hm(&[
            ("language", "zh_CN"),
            ("follow_mouse", "1"),
            ("theme", "dark"),
            ("windowWidth", "1000"),
            ("auto_hide", "0"),
        ]);
        let kept = filter_session_prefs(&extra);
        assert_eq!(kept.len(), 3, "应只保留 language/follow_mouse/auto_hide");
        assert_eq!(kept.get("language").unwrap(), "zh_CN");
        assert_eq!(kept.get("follow_mouse").unwrap(), "1");
        assert_eq!(kept.get("auto_hide").unwrap(), "0");
        assert!(!kept.contains_key("theme"), "theme 非会话偏好应被丢弃");
        assert!(!kept.contains_key("windowWidth"), "windowWidth 非会话偏好应被丢弃");
    }

    #[test]
    fn test_filter_keeps_all_pref_keys() {
        let extra = hm(&[
            ("language", "en"),
            ("follow_mouse", "0"),
            ("auto_hide", "1"),
            ("generated_by", "gorun-rs"),
            ("low_resident", "1"),
        ]);
        let kept = filter_session_prefs(&extra);
        assert_eq!(kept.len(), 5, "全部 5 个会话偏好键都应保留");
        assert_eq!(kept.get("generated_by").unwrap(), "gorun-rs");
        assert_eq!(kept.get("low_resident").unwrap(), "1");
    }

    #[test]
    fn test_filter_empty_returns_empty() {
        let kept = filter_session_prefs(&HashMap::new());
        assert!(kept.is_empty());
    }

    #[test]
    fn test_filter_drops_unknown_keys() {
        let extra = hm(&[
            ("theme", "light"),
            ("config_integrity", "abc"),
            ("windowX", "5"),
            ("unknown", "x"),
        ]);
        let kept = filter_session_prefs(&extra);
        assert!(kept.is_empty(), "全非会话偏好应得空: {:?}", kept);
    }
}
