mod config;
mod hotkey;
mod icon_cache;
mod item_manager;
mod localization;
mod model;
mod runner;
mod storage;
mod tray;

use item_manager::ItemManager;
use model::{AppConfig, Category, Item, ViewType};
use storage::Storage;

slint::include_modules!();

fn main() {
    let storage = Storage::new(
        config::config_path().to_str().unwrap_or("config.json").to_string(),
    );

    let mut app_config = storage.load().unwrap_or_default();
    if app_config.data.categories.is_empty() {
        let default_cat = Category {
            id: model::generate_id("cat"),
            name: "Default".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        };
        app_config.data.categories.push(default_cat);
    }

    let mut manager = ItemManager::new(app_config);
    let _icon_cache = icon_cache::IconCache::new();
    let _loc = localization::Localization::new("zh-CN");

    let (_tray, _tray_tx) = tray::TrayIcon::new();

    let ui = MainWindow::new().unwrap();

    wire_handler(&ui, &mut manager);

    let categories: Vec<CategoryModel> = manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();
    let category_model = std::rc::Rc::new(slint::VecModel::from(categories));
    ui.set_categories(category_model.into());

    ui.run().unwrap();

    if manager.is_modified() {
        let _ = storage.save(manager.config());
    }
}

fn wire_handler(ui: &MainWindow, _manager: &mut ItemManager) {
    ui.on_file_dropped(move |_name, _target| {
        // Placeholder: real manager integration would go here
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_category_model() {
        let cat = CategoryModel {
            id: "cat_1".into(),
            name: "Games".into(),
        };
        assert_eq!(cat.id, "cat_1");
        assert_eq!(cat.name, "Games");
    }

    #[test]
    fn test_create_item_model() {
        let item = ItemModel {
            id: "item_1".into(),
            name: "Notepad".into(),
            target: "C:\\Windows\\notepad.exe".into(),
            icon_path: "".into(),
        };
        assert_eq!(item.name, "Notepad");
    }
}
