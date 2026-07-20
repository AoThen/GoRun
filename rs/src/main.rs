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

    let categories: Vec<CategoryModel> = manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();

    let category_model = slint::VecModel::from(categories);
    ui.set_categories(category_model.clone().into());

    let items: Vec<ItemModel> = manager
        .items("")
        .iter()
        .map(|i| ItemModel {
            id: i.id.clone().into(),
            name: i.name.clone().into(),
            target: i.target.clone().into(),
            icon_path: i.icon_path.clone().into(),
        })
        .collect();

    let item_model = slint::VecModel::from(items);
    ui.set_items(item_model.into());

    ui.run().unwrap();

    if manager.is_modified() {
        let _ = storage.save(manager.config());
    }
}

#[derive(Clone, Default, slint::PartialEq)]
pub struct CategoryModel {
    pub id: slint::SharedString,
    pub name: slint::SharedString,
}

#[derive(Clone, Default, slint::PartialEq)]
pub struct ItemModel {
    pub id: slint::SharedString,
    pub name: slint::SharedString,
    pub target: slint::SharedString,
    pub icon_path: slint::SharedString,
}
