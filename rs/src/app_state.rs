//! 应用状态容器（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::icon_cache::IconCache;
use crate::item_manager::ItemManager;
use crate::localization::Localization;
use crate::platform::DropHandler;
use crate::storage::Storage;
use crate::EditDialog;

pub struct AppState {
    pub manager: ItemManager,
    pub storage: Storage,
    pub icon_cache: IconCache,
    pub current_category_id: String,
    pub search_query: String,
    pub edit_dialog: Option<EditDialog>,
    pub loc: Rc<RefCell<Localization>>,
    /// 保存失败标志：置位后由周期 Timer 一次性写状态栏提示，避免静默丢数据。
    pub save_fail_notified: bool,
    /// 后台提取出的图标缓存路径（item_id -> png 路径），由 Timer 回写模型并落盘（P0）。
    pub pending_icon_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    /// OLE/子类化拖放处理器：必须随应用存活，Drop 时移除子类化并释放 refdata。
    pub drop_handler: Option<DropHandler>,
    /// 窗口几何已变化但尚未落盘（P1-5 去抖）。
    pub geometry_dirty: bool,
    /// 最近一次几何变化时刻，用于判断去抖窗口是否已过。
    pub last_geometry_change: Option<std::time::Instant>,
    /// 条目级全局热键管理器（F8/4.1）：多热键注册/分发。
    pub item_hotkey_mgr: Rc<RefCell<crate::hotkey::ItemHotkeyManager>>,
    /// 条目热键触发通道发送端（item_id），由消费线程收。
    pub item_hotkey_tx: std::sync::mpsc::Sender<String>,
}

impl AppState {
    /// 几何变化是否已到可落盘时机（force=true 表示强制立即写）。
    pub fn geometry_should_flush(&self, force: bool) -> bool {
        if !self.geometry_dirty {
            return false;
        }
        if force {
            return true;
        }
        self.last_geometry_change
            .map(|t| t.elapsed() >= crate::geometry::GEOMETRY_SAVE_DEBOUNCE)
            .unwrap_or(true)
    }

    /// 若满足时机则落盘几何；返回是否已落盘成功。
    pub fn flush_geometry_if_due(&mut self, force: bool) -> bool {
        if !self.geometry_should_flush(force) {
            return true;
        }
        self.flush_geometry()
    }

    /// 立即落盘几何（几何单独走 storage.save，不轮转备份）。
    ///
    /// 注意：`storage.save` 写入的是**完整配置**（含数据变更），因此这里
    /// **不能** `reset_modified()` —— 否则数据变更的"已修改"标记会被几何写盘抹掉，
    /// 导致本该走 `save_with_backup`（轮转备份）的数据变更被当成普通写丢弃备份。
    /// 仅清除几何自身的 dirty 标记。
    pub fn flush_geometry(&mut self) -> bool {
        if !self.geometry_dirty {
            return true;
        }
        if self.storage.save(self.manager.config()) {
            self.geometry_dirty = false;
            self.last_geometry_change = None;
            true
        } else {
            log::warn!("Failed to save window geometry (will retry)");
            false
        }
    }

    /// 保存修改。返回是否成功（无修改视为成功）。
    /// 失败时置位 `save_fail_notified`，供 UI 层轮询提示。
    pub fn save_if_modified(&mut self) -> bool {
        if self.manager.is_modified() {
            if self.storage.save_with_backup(self.manager.config()) {
                self.manager.reset_modified();
                self.save_fail_notified = false;
                true
            } else {
                log::error!("Failed to save config after modification");
                self.save_fail_notified = true;
                false
            }
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppConfig;

    fn make_state(path: &str) -> AppState {
        let (ihk_tx, _ihk_rx) = std::sync::mpsc::channel::<String>();
        AppState {
            manager: ItemManager::new(AppConfig::default()),
            storage: Storage::new(path.to_string()),
            icon_cache: IconCache::new(),
            current_category_id: String::new(),
            search_query: String::new(),
            edit_dialog: None,
            loc: Rc::new(RefCell::new(Localization::new("zh-CN"))),
            save_fail_notified: false,
            pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            drop_handler: None,
            geometry_dirty: false,
            last_geometry_change: None,
            item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
            item_hotkey_tx: ihk_tx,
        }
    }

    #[test]
    fn test_save_if_modified_unmodified_returns_true() {
        let ok = std::env::temp_dir()
            .join(format!("gorun_appstate_unmod_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&ok);
        assert!(!state.manager.is_modified());
        assert!(state.save_if_modified());
        assert!(!state.save_fail_notified);
        let _ = std::fs::remove_file(&ok);
    }

    #[test]
    fn test_save_if_modified_success_clears_flag() {
        let ok = std::env::temp_dir()
            .join(format!("gorun_appstate_ok_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&ok);
        state.save_fail_notified = true;
        let _ = state.manager.config_mut();
        assert!(state.manager.is_modified());
        assert!(state.save_if_modified());
        assert!(!state.manager.is_modified());
        assert!(!state.save_fail_notified);
        let _ = std::fs::remove_file(&ok);
    }

    #[test]
    fn test_save_if_modified_failure_sets_flag() {
        // 父路径为文件 → save 写盘失败，验证失败路径置位 save_fail_notified 且保留 modified
        let blocker = std::env::temp_dir().join(format!("gorun_block_{}.txt", std::process::id()));
        std::fs::write(&blocker, b"x").unwrap();
        let fail_path = format!("{}/config.json", blocker.to_str().unwrap());
        let mut state = make_state(&fail_path);
        let _ = state.manager.config_mut();
        assert!(state.manager.is_modified());
        assert!(!state.save_if_modified());
        assert!(state.manager.is_modified());
        assert!(state.save_fail_notified);
        let _ = std::fs::remove_file(&blocker);
    }

    #[test]
    fn test_geometry_debounce_not_dirty_by_default() {
        let mut state = make_state("/tmp/gorun_geom_default.json");
        assert!(!state.geometry_dirty);
        assert!(!state.geometry_should_flush(false));
        // 未 dirty 时强制落盘也视为成功（无事可做）
        assert!(state.flush_geometry_if_due(true));
    }

    #[test]
    fn test_geometry_debounce_waits_window_then_flushes() {
        let path = std::env::temp_dir()
            .join(format!("gorun_geom_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&path);
        // 模拟"刚记录一次几何变化"
        let _ = state.manager.config_mut();
        state.geometry_dirty = true;
        state.last_geometry_change = Some(std::time::Instant::now());

        // 刚变化：去抖窗口未到，非强制不落盘
        assert!(!state.geometry_should_flush(false));
        assert!(state.flush_geometry_if_due(false));

        // 强制（退出路径）立即落盘
        assert!(state.flush_geometry_if_due(true));
        assert!(!state.geometry_dirty);
        assert!(std::path::Path::new(&path).exists());
        // 几何写盘不得清除数据变更标记（否则数据变更会绕过备份轮转）
        assert!(state.manager.is_modified());
        let _ = std::fs::remove_file(&path);
    }
}
