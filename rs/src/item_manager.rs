use crate::model::{AppConfig, Category, Item, ViewType};

pub struct ItemManager {
    config: AppConfig,
    modified: bool,
}

impl ItemManager {
    pub fn new(config: AppConfig) -> Self {
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
        self.config
            .data
            .items
            .iter()
            .filter(|i| i.category_id == category_id)
            .cloned()
            .collect()
    }

    pub fn search_items(&self, query: &str) -> Vec<Item> {
        let q = query.to_lowercase();
        self.config
            .data
            .items
            .iter()
            .filter(|i| {
                i.name.to_lowercase().contains(&q)
                    || i.keywords.to_lowercase().contains(&q)
                    || i.target.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    pub fn add_category(&mut self, mut category: Category) {
        if category.id.is_empty() {
            category.id = crate::model::generate_id("cat");
        }
        self.config.data.categories.push(category);
        self.modified = true;
    }

    pub fn update_category(&mut self, category: &Category) -> bool {
        if let Some(c) = self.config.data.categories.iter_mut().find(|c| c.id == category.id) {
            *c = category.clone();
            self.modified = true;
            true
        } else {
            false
        }
    }

    pub fn delete_category(&mut self, id: &str) {
        self.config.data.items.retain(|i| i.category_id != id);
        self.config.data.categories.retain(|c| c.id != id);
        self.modified = true;
    }

    pub fn add_item(&mut self, mut item: Item) -> bool {
        if item.category_id.is_empty() {
            return false;
        }
        if item.id.is_empty() {
            item.id = crate::model::generate_id("item");
        }
        self.config.data.items.push(item);
        self.modified = true;
        true
    }

    pub fn update_item(&mut self, item: &Item) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item.id) {
            *i = item.clone();
            self.modified = true;
            true
        } else {
            false
        }
    }

    pub fn delete_item(&mut self, id: &str) -> bool {
        let before = self.config.data.items.len();
        self.config.data.items.retain(|i| i.id != id);
        let removed = self.config.data.items.len() < before;
        if removed {
            self.modified = true;
        }
        removed
    }

    pub fn move_item(&mut self, item_id: &str, category_id: &str) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item_id) {
            i.category_id = category_id.to_string();
            self.modified = true;
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
}

impl Default for ItemManager {
    fn default() -> Self {
        ItemManager::new(AppConfig::default())
    }
}
