#![allow(dead_code)]

use crate::model::{AppConfig, Category, Item};

pub struct ItemManager {
    config: AppConfig,
    modified: bool,
}

impl ItemManager {
    pub fn new(config: AppConfig) -> Self {
        log::debug!(
            "ItemManager created with {} categories",
            config.data.categories.len()
        );
        ItemManager {
            config,
            modified: false,
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        self.modified = true;
        &mut self.config
    }

    pub fn categories(&self) -> &Vec<Category> {
        &self.config.data.categories
    }

    pub fn items(&self, category_id: &str) -> Vec<Item> {
        if category_id.is_empty() {
            return self.config.data.items.clone();
        }
        self.config
            .data
            .items
            .iter()
            .filter(|i| i.category_id == category_id)
            .cloned()
            .collect()
    }

    pub fn all_items(&self) -> Vec<Item> {
        self.config.data.items.clone()
    }

    pub fn search_items(&self, query: &str) -> Vec<Item> {
        let q = query.to_lowercase();
        let mut scored: Vec<(Item, i32)> = self
            .config
            .data
            .items
            .iter()
            .filter(|i| {
                i.name.to_lowercase().contains(&q)
                    || i.keywords.to_lowercase().contains(&q)
                    || i.target.to_lowercase().contains(&q)
            })
            .map(|item| {
                let mut score = 0;
                if item.name.to_lowercase() == q {
                    score += 1000;
                } else if item.name.to_lowercase().starts_with(&q) {
                    score += 100;
                } else {
                    score += 10;
                }
                if item.keywords.to_lowercase().contains(&q) {
                    score += 50;
                }
                score += item.run_count;
                (item.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let results: Vec<Item> = scored.into_iter().map(|(item, _)| item).collect();
        log::debug!("Search: query={}, results={}", query, results.len());
        results
    }

    pub fn add_category(&mut self, mut category: Category) {
        if category.id.is_empty() {
            category.id = crate::model::generate_id("cat");
        }
        log::info!("Category added: id={}, name={}", category.id, category.name);
        self.config.data.categories.push(category);
        self.modified = true;
    }

    pub fn update_category(&mut self, category: &Category) -> bool {
        if let Some(c) = self
            .config
            .data
            .categories
            .iter_mut()
            .find(|c| c.id == category.id)
        {
            *c = category.clone();
            self.modified = true;
            log::info!("Category updated: id={}", category.id);
            true
        } else {
            log::warn!("Category update failed: id={} not found", category.id);
            false
        }
    }

    pub fn delete_category(&mut self, id: &str) {
        self.config.data.items.retain(|i| i.category_id != id);
        self.config.data.categories.retain(|c| c.id != id);
        self.modified = true;
        log::info!("Category deleted: id={}", id);
    }

    pub fn add_item(&mut self, mut item: Item) -> bool {
        if item.category_id.is_empty() {
            log::warn!("Item add failed: empty category_id");
            return false;
        }
        if item.id.is_empty() {
            item.id = crate::model::generate_id("item");
        }
        log::info!("Item added: id={}, name={}", item.id, item.name);
        self.config.data.items.push(item);
        self.modified = true;
        true
    }

    pub fn update_item(&mut self, item: &Item) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item.id) {
            *i = item.clone();
            self.modified = true;
            log::info!("Item updated: id={}", item.id);
            true
        } else {
            log::warn!("Item update failed: id={} not found", item.id);
            false
        }
    }

    pub fn delete_item(&mut self, id: &str) -> bool {
        let before = self.config.data.items.len();
        self.config.data.items.retain(|i| i.id != id);
        let removed = self.config.data.items.len() < before;
        if removed {
            self.modified = true;
            log::info!("Item deleted: id={}", id);
        } else {
            log::warn!("Item delete failed: id={} not found", id);
        }
        removed
    }

    pub fn move_item(&mut self, item_id: &str, category_id: &str) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item_id) {
            i.category_id = category_id.to_string();
            self.modified = true;
            log::info!("Item moved: {} -> {}", item_id, category_id);
            true
        } else {
            false
        }
    }

    pub fn increment_run_count(&mut self, item_id: &str) {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item_id) {
            i.run_count += 1;
            self.modified = true;
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn handle_drop(&mut self, files: &[std::path::PathBuf], category_id: &str) -> usize {
        let mut count = 0;
        for path in files {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let target = path.to_string_lossy().to_string();
            let working_dir = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let item = Item {
                id: crate::model::generate_id("item"),
                name,
                target,
                working_dir,
                category_id: category_id.to_string(),
                ..Default::default()
            };

            if self.add_item(item) {
                count += 1;
            }
        }
        count
    }

    pub fn get_item(&self, id: &str) -> Option<&Item> {
        self.config.data.items.iter().find(|i| i.id == id)
    }
}

impl Default for ItemManager {
    fn default() -> Self {
        ItemManager::new(AppConfig::default())
    }
}
