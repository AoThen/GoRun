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
