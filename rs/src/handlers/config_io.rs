//! 配置导出/导入（F8）：Settings 菜单 → 导出配置 / 导入配置。
//! 导出用存储的 RawFile 序列化（与落盘同格式）；
//! 导入整体替换数据与偏好设置，但保留语言/跟随鼠标/自动隐藏等会话偏好。

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::MainWindow;

/// 导入配置时保留的会话偏好键（语言/跟随鼠标/自动隐藏/低驻留）。
/// 这些属于当前会话运行状态，不应被导入文件整体覆盖。
const SESSION_PREF_KEYS: [&str; 4] = [
    "language",
    "follow_mouse",
    "auto_hide",
    "low_resident",
];

pub(crate) fn session_pref_keys() -> &'static [&'static str] {
    &SESSION_PREF_KEYS
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

/// 应用全局热键：注销旧键 → 注册新键；失败时回滚重注册旧键（P1-7）。
/// 成功写配置并落盘，失败将配置回滚为旧键，保证"配置值 = 实际注册值"。
/// 供改键流程（wire_handler input_dialog）与导入配置（import_config_apply）共用。
/// 返回是否成功注册；失败时调用方负责 UI 提示（Tip_HotkeyRegisterFail）。
/// `old_hk` 为回滚目标旧键：由调用方在替换配置前捕获传入（导入路径配置已替换，
/// 内部读 state 会拿到 new_hk，必须显式传导入前旧键）。
pub fn apply_global_hotkey(
    state: &Rc<RefCell<AppState>>,
    hotkey_mgr: &Rc<RefCell<crate::hotkey::HotkeyManager>>,
    hk_tx: &Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
    new_hk: &str,
    old_hk: &str,
) -> bool {
    // 重注册需保留 notifier：unregister 后复用原 Sender 重新 spawn pump
    let ok = {
        let mut mgr = hotkey_mgr.borrow_mut();
        if !mgr.unregister() {
            log::warn!("Hotkey unregister failed, attempting re-register anyway");
        }
        match hk_tx.borrow().clone() {
            Some(tx) => mgr.register_with_notifier(new_hk, tx),
            None => false,
        }
    };
    if ok {
        let mut s = state.borrow_mut();
        s.manager.config_mut().config.global_hotkey = new_hk.to_string();
        s.save_if_modified();
        log::info!("Global hotkey updated to: {}", new_hk);
    } else {
        // P1-7：新键注册失败 → 回滚重注册旧键，避免热键静默失效
        let rollback_ok = {
            let mut mgr = hotkey_mgr.borrow_mut();
            if !mgr.unregister() {
                log::warn!("Hotkey unregister failed during rollback");
            }
            match hk_tx.borrow().clone() {
                Some(tx) => mgr.register_with_notifier(old_hk, tx),
                None => false,
            }
        };
        if !rollback_ok {
            log::error!("Hotkey rollback failed, old hotkey lost: {}", old_hk);
        }
        log::warn!("Hotkey re-registration failed for: {}", new_hk);
    }
    ok
}

/// Settings 菜单 → 导出配置：用户选目标文件，写当前配置 JSON。
pub fn export_config(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    let loc = state.borrow().loc.clone();
    let (title, filter) = {
        let lb = loc.borrow();
        (
            lb.tr("Title_ExportConfig").to_string(),
            format!("{}\0*.json\0\0", lb.tr("STR_FilterConfig")),
        )
    };
    let Some(path) = crate::fileops::save_file_dialog(&title, &filter) else {
        return;
    };
    let st = state.borrow();
    if st.storage.export_to(&path, st.manager.config()) {
        log::info!("Exported config to {}", path);
        let msg = crate::util::tr_fill(&loc, "Status_Exported", &path);
        crate::ui_sync::show_toast(ui, msg, false);
    } else {
        log::error!("Failed to export config to {}", path);
        let msg = loc.borrow().tr("Status_ExportFailed").to_string();
        crate::ui_sync::show_toast(ui, msg, true);
    }
}

/// 导入配置第一步：选文件并预解析。解析失败时自行提示并返回 None；
/// 成功返回 (文件路径, 解析结果)，供调用方统计导入规模并弹出"覆盖确认"框，
/// 用户确认后再交 import_config_apply 执行实际替换（防误选文件静默覆盖数据）。
pub fn prepare_import(
    ui: &MainWindow,
    state: &Rc<RefCell<AppState>>,
) -> Option<(String, crate::model::AppConfig)> {
    let loc = state.borrow().loc.clone();
    let (title, filter) = {
        let lb = loc.borrow();
        (
            lb.tr("Title_ImportConfig").to_string(),
            format!("{}\0*.json\0\0", lb.tr("STR_FilterConfig")),
        )
    };
    let path = crate::fileops::browse_file_dialog(&title, &filter)?;
    let imported = match crate::storage::Storage::new(path.clone()).load() {
        Some(c) => c,
        None => {
            log::error!("Failed to parse import config: {}", path);
            let msg = loc.borrow().tr("Status_ImportFailedInvalid").to_string();
            crate::ui_sync::show_toast(ui, msg, true);
            return None;
        }
    };
    Some((path, imported))
}

/// 导入配置第二步（用户确认覆盖后调用）：整体替换数据与行为设置，
/// 保留语言/跟随鼠标/自动隐藏等会话偏好；成功后刷新侧栏与条目。
/// 导入的配置若更改了全局热键，则重注册使本会话立即生效（与条目热键同步对齐）。
/// `path` 仅用于日志。
pub fn import_config_apply(
    ui: &MainWindow,
    state: &Rc<RefCell<AppState>>,
    hotkey_mgr: &Rc<RefCell<crate::hotkey::HotkeyManager>>,
    hk_tx: &Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
    path: &str,
    imported: crate::model::AppConfig,
) {
    let loc = state.borrow().loc.clone();
    let mut st = state.borrow_mut();
    let (old_hk, new_hk, new_current) = {
        let cfg = st.manager.config_mut();
        // 保留会话偏好（语言/跟随鼠标/自动隐藏/低驻留），其余整体替换
        let keep = filter_session_prefs(&cfg.extra_config);
        let old_hk = cfg.config.global_hotkey.clone();
        cfg.version = imported.version;
        cfg.data = imported.data;
        cfg.config = imported.config;
        cfg.extra_config = keep;
        // P2-8：导入源可能是外部/旧版文件——分类可能为空、条目可能指向
        // 不存在的分类：补默认分类（本地化名与启动行为一致）+ 孤儿条目
        // 归首分类；当前分类取归一化后的首个分类（分类为空时不再落 "全部条目"
        // 视图）
        let default_name = loc.borrow().tr("Default_Category").to_string();
        crate::model::ensure_default_category(&mut cfg.data, &default_name);
        crate::model::attach_orphan_items(&mut cfg.data);
        let new_current = cfg
            .data
            .categories
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_default();
        (old_hk, cfg.config.global_hotkey.clone(), new_current)
    };
    let saved = st.save_if_modified();
    drop(st);

    if !saved {
        log::error!("Failed to save imported config");
        let msg = loc.borrow().tr("Status_ImportFailedSave").to_string();
        crate::ui_sync::show_toast(ui, msg, true);
        return;
    }
    crate::ui_sync::refresh_categories_ui(ui, &state.borrow());
    // H-1：导入属"数据集更换"，须清空搜索态后显示首分类全量条目。
    // 否则界面按空 query 刷新为全量条目，而 run_item_at_index 仍按残留的
    // 旧 search_query 取项，索引错位可能启动错误程序（点 A 却启动 B）。
    crate::ui_sync::switch_category(ui, state, &new_current, false);
    // F8/4.1：导入后重注册条目热键（条目集整体变化）。
    // toast 单槽覆盖：失败列表非空时优先提示冲突，否则提示导入成功。
    let hotkey_failed = crate::ui_sync::sync_item_hotkeys(&state.borrow());
    if hotkey_failed.is_empty() {
        let msg = loc.borrow().tr("Status_Imported").to_string();
        crate::ui_sync::show_toast(ui, msg, false);
    } else {
        let msg = crate::util::tr_fill(&loc, "Status_HotkeyConflict", &hotkey_failed.join(", "));
        crate::ui_sync::show_toast(ui, msg, true);
    }
    // P2-2：导入的配置更改了全局热键时，重注册使本会话立即生效（条目热键已有对等同步）。
    // 注册失败已在 apply_global_hotkey 内回滚旧键，此处将配置同步回滚，保证一致。
    if new_hk != old_hk && !apply_global_hotkey(state, hotkey_mgr, hk_tx, &new_hk, &old_hk) {
        let mut st = state.borrow_mut();
        st.manager.config_mut().config.global_hotkey = old_hk.clone();
        st.save_if_modified();
        let msg = loc.borrow().tr("Tip_HotkeyRegisterFail").to_string();
        crate::ui_sync::show_toast(ui, msg, true);
    }
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
        assert_eq!(keys.len(), 4);
        assert!(keys.contains(&"language"));
        assert!(keys.contains(&"follow_mouse"));
        assert!(keys.contains(&"auto_hide"));
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
        assert!(
            !kept.contains_key("windowWidth"),
            "windowWidth 非会话偏好应被丢弃"
        );
    }

    #[test]
    fn test_filter_keeps_all_pref_keys() {
        let extra = hm(&[
            ("language", "en"),
            ("follow_mouse", "0"),
            ("auto_hide", "1"),
            ("low_resident", "1"),
        ]);
        let kept = filter_session_prefs(&extra);
        assert_eq!(kept.len(), 4, "全部 4 个会话偏好键都应保留");
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

    /// 构造最小 AppState（1 空分类；storage 指向独立临时文件），供热键测试用。
    fn make_hotkey_state() -> Rc<RefCell<AppState>> {
        use crate::icon_cache::IconCache;
        use crate::item_manager::ItemManager;
        use crate::model::{AppConfig, AppData, Category, Config};
        use crate::storage::Storage;
        let app_config = AppConfig {
            version: "test".into(),
            data: AppData {
                categories: vec![Category {
                    id: "cat_1".into(),
                    name: "P".into(),
                    ..Default::default()
                }],
                items: vec![],
            },
            config: Config {
                global_hotkey: "Ctrl+/".into(),
                ..Default::default()
            },
            extra_config: Default::default(),
        };
        Rc::new(RefCell::new(AppState {
            manager: ItemManager::new(app_config),
            storage: Storage::new(
                std::env::temp_dir()
                    .join(format!("gorun_cfgio_{}.json", std::process::id()))
                    .to_string_lossy()
                    .to_string(),
            ),
            icon_cache: IconCache::new(),
            current_category_id: "cat_1".into(),
            search_query: String::new(),
            edit_dialog: None,
            loc: Rc::new(RefCell::new(crate::localization::Localization::new("zh-CN"))),
            save_fail_notified: false,
            pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            icon_extract_attempted: std::sync::Mutex::new(std::collections::HashSet::new()),
            drop_handler: None,
            geometry_dirty: false,
            last_geometry_change: None,
            soft_dirty: false,
            last_soft_change: None,
            pending_run_results: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
            item_hotkey_tx: {
                let (tx, _rx) = std::sync::mpsc::channel::<String>();
                tx
            },
            low_resident: Rc::new(std::cell::Cell::new(false)),
            auto_hide_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }))
    }

    #[test]
    fn test_apply_global_hotkey_no_channel_fails_and_keeps_old() {
        // hk_tx=None：注册通知通道不可用 → 新键注册失败 → 回滚，配置保持旧键
        let state = make_hotkey_state();
        let hk_mgr = Rc::new(RefCell::new(crate::hotkey::HotkeyManager::new()));
        let hk_tx: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>> = Rc::new(RefCell::new(None));
        let ok = apply_global_hotkey(&state, &hk_mgr, &hk_tx, "Ctrl+Shift+G", "Ctrl+/");
        assert!(!ok, "注册通道不可用时应返回失败");
        assert_eq!(
            state.borrow().manager.config().config.global_hotkey,
            "Ctrl+/",
            "失败后配置应保持旧键"
        );
    }

    #[test]
    fn test_apply_global_hotkey_invalid_key_rolls_back() {
        // 非法键字符串：任何平台均解析/注册失败 → 回滚旧键
        let state = make_hotkey_state();
        let hk_mgr = Rc::new(RefCell::new(crate::hotkey::HotkeyManager::new()));
        let (tx, _rx) = std::sync::mpsc::channel::<()>();
        let hk_tx: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>> =
            Rc::new(RefCell::new(Some(tx)));
        let ok = apply_global_hotkey(&state, &hk_mgr, &hk_tx, "NotAHotkey", "Ctrl+/");
        assert!(!ok, "非法热键应注册失败");
        assert_eq!(
            state.borrow().manager.config().config.global_hotkey,
            "Ctrl+/",
            "失败后配置应回滚为旧键"
        );
    }
}
