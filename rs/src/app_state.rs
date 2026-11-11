//! 应用状态容器（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::icon_cache::IconCache;
use crate::item_manager::ItemManager;
use crate::localization::Localization;
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
}

impl AppState {
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
        AppState {
            manager: ItemManager::new(AppConfig::default()),
            storage: Storage::new(path.to_string()),
            icon_cache: IconCache::new(),
            current_category_id: String::new(),
            search_query: String::new(),
            edit_dialog: None,
            loc: Rc::new(RefCell::new(Localization::new("zh-CN"))),
            save_fail_notified: false,
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
}
