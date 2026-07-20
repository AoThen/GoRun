use std::fs;
use std::path::PathBuf;

use crate::config;

pub struct IconCache {
    cache_dir: PathBuf,
}

impl IconCache {
    pub fn new() -> Self {
        let dir = config::icons_dir();
        let _ = fs::create_dir_all(&dir);
        log::info!("IconCache initialized: dir={:?}", dir);
        IconCache { cache_dir: dir }
    }

    pub fn cache_path(&self, item_id: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.png", item_id))
    }

    pub fn get_icon_path(&self, item_id: &str) -> Option<PathBuf> {
        let path = self.cache_path(item_id);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn delete_cache(&self, item_id: &str) {
        let path = self.cache_path(item_id);
        if path.exists() {
            log::debug!("Icon cache deleted: id={}", item_id);
            let _ = fs::remove_file(&path);
        }
    }
}
