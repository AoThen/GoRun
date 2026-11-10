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
}

impl AppState {
    pub fn save_if_modified(&mut self) {
        if self.manager.is_modified() {
            if self.storage.save_with_backup(self.manager.config()) {
                self.manager.reset_modified();
            } else {
                log::error!("Failed to save config after modification");
            }
        }
    }
}
