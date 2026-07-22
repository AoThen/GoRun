use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{AppConfig, AppData, Category, Config, Item, ViewType};

#[derive(Serialize, Deserialize)]
struct RawFile {
    version: String,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    items: Vec<RawItem>,
    #[serde(default)]
    config: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct RawCategory {
    id: String,
    name: String,
    #[serde(rename = "sortOrder", default)]
    sort_order: i32,
    #[serde(rename = "viewType", default)]
    view_type: i32,
    #[serde(rename = "iconSize", default = "default_icon_size")]
    icon_size: i32,
}

fn default_icon_size() -> i32 {
    48
}

#[derive(Serialize, Deserialize)]
struct RawItem {
    id: String,
    name: String,
    target: String,
    #[serde(default)]
    arguments: String,
    #[serde(rename = "workingDir", default)]
    working_dir: String,
    #[serde(rename = "iconPath", default)]
    icon_path: String,
    #[serde(rename = "iconIndex", default)]
    icon_index: i32,
    #[serde(rename = "runAsAdmin", default)]
    run_as_admin: bool,
    #[serde(rename = "runCount", default)]
    run_count: i32,
    #[serde(default)]
    keywords: String,
    #[serde(default)]
    remark: String,
    #[serde(rename = "categoryId", default)]
    category_id: String,
    #[serde(rename = "sortOrder", default)]
    sort_order: i32,
}

pub struct Storage {
    path: String,
}

impl Storage {
    pub fn new(path: String) -> Self {
        Storage { path }
    }

    pub fn load(&self) -> Option<AppConfig> {
        if !Path::new(&self.path).exists() {
            log::warn!("Config file not found: {}", self.path);
            return None;
        }
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => {
                log::error!("Failed to open config file: {}", self.path);
                return None;
            }
        };
        let reader = BufReader::new(file);
        let raw: RawFile = match serde_json::from_reader(reader) {
            Ok(r) => r,
            Err(_) => {
                log::error!("Failed to parse config JSON: {}", self.path);
                return None;
            }
        };

        let mut extra_config = raw.config.clone();
        extra_config.remove("globalHotkey");
        extra_config.remove("windowX");
        extra_config.remove("windowY");
        extra_config.remove("windowWidth");
        extra_config.remove("windowHeight");

        let config = Config {
            global_hotkey: raw
                .config
                .get("globalHotkey")
                .cloned()
                .unwrap_or_else(|| "Ctrl+Alt+M".to_string()),
            window_x: raw
                .config
                .get("windowX")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            window_y: raw
                .config
                .get("windowY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            window_width: raw
                .config
                .get("windowWidth")
                .and_then(|v| v.parse().ok())
                .unwrap_or(800),
            window_height: raw
                .config
                .get("windowHeight")
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
        };

        let categories: Vec<Category> = raw
            .categories
            .into_iter()
            .map(|c| Category {
                id: c.id,
                name: c.name,
                sort_order: c.sort_order,
                view_type: if c.view_type == 1 {
                    ViewType::List
                } else {
                    ViewType::Icon
                },
                icon_size: c.icon_size,
            })
            .collect();

        let items: Vec<Item> = raw
            .items
            .into_iter()
            .map(|i| Item {
                id: i.id,
                name: i.name,
                target: i.target,
                arguments: i.arguments,
                working_dir: i.working_dir,
                icon_path: i.icon_path,
                icon_index: i.icon_index.max(0),
                run_as_admin: i.run_as_admin,
                run_count: i.run_count.max(0),
                keywords: i.keywords,
                remark: i.remark,
                category_id: i.category_id,
                sort_order: i.sort_order,
            })
            .collect();

        log::info!(
            "Config loaded: {} categories, {} items",
            categories.len(),
            items.len()
        );

        Some(AppConfig {
            version: raw.version,
            data: AppData { categories, items },
            config,
            extra_config,
        })
    }

    pub fn save(&self, app_config: &AppConfig) -> bool {
        let raw_categories: Vec<RawCategory> = app_config
            .data
            .categories
            .iter()
            .map(|c| RawCategory {
                id: c.id.clone(),
                name: c.name.clone(),
                sort_order: c.sort_order,
                view_type: c.view_type.clone() as i32,
                icon_size: c.icon_size,
            })
            .collect();

        let raw_items: Vec<RawItem> = app_config
            .data
            .items
            .iter()
            .map(|i| RawItem {
                id: i.id.clone(),
                name: i.name.clone(),
                target: i.target.clone(),
                arguments: i.arguments.clone(),
                working_dir: i.working_dir.clone(),
                icon_path: i.icon_path.clone(),
                icon_index: i.icon_index,
                run_as_admin: i.run_as_admin,
                run_count: i.run_count,
                keywords: i.keywords.clone(),
                remark: i.remark.clone(),
                category_id: i.category_id.clone(),
                sort_order: i.sort_order,
            })
            .collect();

        let mut config_map = app_config.extra_config.clone();
        config_map.insert(
            "globalHotkey".to_string(),
            app_config.config.global_hotkey.clone(),
        );
        config_map.insert(
            "windowX".to_string(),
            app_config.config.window_x.to_string(),
        );
        config_map.insert(
            "windowY".to_string(),
            app_config.config.window_y.to_string(),
        );
        config_map.insert(
            "windowWidth".to_string(),
            app_config.config.window_width.to_string(),
        );
        config_map.insert(
            "windowHeight".to_string(),
            app_config.config.window_height.to_string(),
        );

        let raw = RawFile {
            version: app_config.version.clone(),
            categories: raw_categories,
            items: raw_items,
            config: config_map,
        };

        let json = match serde_json::to_string(&raw) {
            Ok(s) => s,
            Err(_) => {
                log::error!("Failed to serialize config to JSON");
                return false;
            }
        };

        let path = Path::new(&self.path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let tmp_path = format!("{}.tmp", self.path);
        if fs::write(&tmp_path, &json).is_err() {
            log::error!("Failed to write temp file: {}", tmp_path);
            return false;
        }
        if fs::rename(&tmp_path, &self.path).is_err() {
            let _ = fs::remove_file(&tmp_path);
            log::error!("Failed to rename temp to config: {}", self.path);
            return false;
        }
        log::info!("Config saved to: {}", self.path);
        true
    }

    pub fn rotate_backups(&self) -> bool {
        let path = Path::new(&self.path);
        if !path.exists() {
            return true;
        }

        let bak3 = format!("{}.bak3", self.path);
        if Path::new(&bak3).exists() {
            if fs::remove_file(&bak3).is_err() {
                log::error!("Failed to delete old backup: {}", bak3);
                return false;
            }
        }

        let bak2 = format!("{}.bak2", self.path);
        if Path::new(&bak2).exists() {
            if fs::rename(&bak2, &bak3).is_err() {
                log::error!("Failed to rotate backup .bak2 to .bak3");
                return false;
            }
        }

        let bak1 = format!("{}.bak1", self.path);
        if Path::new(&bak1).exists() {
            if fs::rename(&bak1, &bak2).is_err() {
                log::error!("Failed to rotate backup .bak1 to .bak2");
                return false;
            }
        }

        if fs::copy(&self.path, &bak1).is_err() {
            log::error!("Failed to create backup .bak1");
            return false;
        }

        log::info!("Backup rotation completed");
        true
    }

    pub fn load_from_backup(&self) -> Option<AppConfig> {
        for suffix in &[".bak1", ".bak2", ".bak3"] {
            let backup_path = format!("{}{}", self.path, suffix);
            if Path::new(&backup_path).exists() {
                log::info!("Trying to load from backup: {}", backup_path);
                let backup_storage = Storage::new(backup_path);
                if let Some(config) = backup_storage.load() {
                    log::info!("Successfully loaded from backup: {}", backup_path);
                    if self.save(&config) {
                        log::info!("Restored backup to main config file");
                    }
                    return Some(config);
                }
            }
        }
        log::warn("No valid backup found");
        None
    }

    pub fn save_with_backup(&self, app_config: &AppConfig) -> bool {
        if !self.rotate_backups() {
            log::error!("Backup rotation failed, aborting save");
            return false;
        }
        self.save(app_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::generate_id;

    #[test]
    fn test_save_and_load_roundtrip() {
        let mut config = AppConfig::default();
        config.version = "1.0.0".to_string();
        config.data.categories.push(Category {
            id: generate_id("cat"),
            name: "Test Category".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        });
        config.data.items.push(Item {
            id: generate_id("item"),
            name: "Test Item".to_string(),
            target: "C:\\Windows\\notepad.exe".to_string(),
            ..Default::default()
        });
        config.config.global_hotkey = "Ctrl+Alt+M".to_string();

        let path = std::env::temp_dir()
            .join("gorun_test_config.json")
            .to_str()
            .unwrap()
            .to_string();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.data.categories.len(), 1);
        assert_eq!(loaded.data.categories[0].name, "Test Category");
        assert_eq!(loaded.data.items.len(), 1);
        assert_eq!(loaded.data.items[0].target, "C:\\Windows\\notepad.exe");
        assert_eq!(loaded.config.global_hotkey, "Ctrl+Alt+M");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let storage = Storage::new("/tmp/nonexistent_gorun_config_12345.json".to_string());
        assert!(storage.load().is_none());
    }
}
