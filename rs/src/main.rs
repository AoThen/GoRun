mod config;
mod hotkey;
mod icon_cache;
mod item_manager;
mod localization;
mod logger;
mod model;
mod runner;
mod storage;
mod tray;

use item_manager::ItemManager;
use model::{AppConfig, Category, Item, ViewType};
use storage::Storage;

slint::include_modules!();

fn main() {
    logger::init().ok();
    log::info!("GoRun starting...");

    let storage = Storage::new(
        config::config_path().to_str().unwrap_or("config.json").to_string(),
    );

    let mut app_config = storage.load().unwrap_or_default();
    if app_config.data.categories.is_empty() {
        log::warn!("No categories found, created default category");
        let default_cat = Category {
            id: model::generate_id("cat"),
            name: "Default".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        };
        app_config.data.categories.push(default_cat);
    }

    log::info!("Config loaded from: {:?}", config::config_path());

    let mut manager = ItemManager::new(app_config);
    let _icon_cache = icon_cache::IconCache::new();
    let _loc = localization::Localization::new("zh-CN");

    let (_tray, _tray_tx) = tray::TrayIcon::new();

    let ui = MainWindow::new().unwrap();
    log::info!("MainWindow created successfully");

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

    log::info!("Entering main event loop");
    ui.run().unwrap();
    log::info!("Main event loop ended, modified={}", manager.is_modified());

    if manager.is_modified() {
        let _ = storage.save(manager.config());
    }
}

fn wire_handler(ui: &MainWindow, _manager: &mut ItemManager) {
    let right_clicked_index: std::rc::Rc<std::cell::Cell<Option<i32>>> =
        std::rc::Rc::new(std::cell::Cell::new(None));

    ui.on_file_dropped(move |name, target| {
        log::info!("File dropped: name={}, target={}", name, target);
    });

    ui.on_search_changed(move |text| {
        log::debug!("Search changed: {}", text);
    });

    ui.on_category_selected(move |index| {
        log::debug!("Category selected: index={}", index);
    });

    ui.on_toggle_view(move || {
        log::debug!("View toggled");
    });

    ui.on_toggle_theme(move || {
        log::debug!("Theme toggled");
    });

    ui.on_new_item_clicked(move || {
        log::debug!("New item clicked");
    });

    ui.on_refresh_icons(move || {
        log::debug!("Refresh icons");
    });

    ui.on_item_double_clicked(move |index| {
        log::debug!("Item double-clicked: index={}", index);
    });

    let rci = right_clicked_index.clone();
    let weak_ui = ui.as_weak();
    ui.on_item_right_clicked(move |index, _x, _y| {
        log::debug!("Item right-clicked: index={}", index);
        rci.set(Some(index));
        let menu_items = std::rc::Rc::new(slint::VecModel::from(vec![
            slint::SharedString::from("Edit"),
            slint::SharedString::from("Run as Admin"),
            slint::SharedString::from("Delete"),
            slint::SharedString::from("Copy Path"),
        ]));
        if let Some(ui) = weak_ui.upgrade() {
            ui.set_context_menu_items(menu_items.into());
        }
    });

    ui.on_menu_item_selected(move |index| {
        log::debug!("Menu item selected: {}", index);
        if let Some(item_idx) = right_clicked_index.get() {
            match index {
                0 => log::info!("Edit item index={}", item_idx),
                1 => log::info!("Run as admin item index={}", item_idx),
                2 => log::info!("Delete item index={}", item_idx),
                3 => log::info!("Copy path item index={}", item_idx),
                _ => {}
            }
        }
    });

    ui.on_background_clicked(move || {
        log::debug!("Background clicked");
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
