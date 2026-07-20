use slint::Model;

use gorun::item_manager::ItemManager;
use gorun::model::{AppConfig, AppData, Category, Config, Item, ViewType};

slint::include_modules!();

fn setup_ui_with_data(categories: Vec<Category>, items: Vec<Item>) -> MainWindow {
    let app_config = AppConfig {
        version: "test".into(),
        data: AppData { categories, items },
        config: Config::default(),
        extra_config: Default::default(),
    };
    let manager = ItemManager::new(app_config);

    let ui = MainWindow::new().unwrap();

    let cat_models: Vec<CategoryModel> = manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();
    ui.set_categories(slint::VecModel::from(cat_models).into());

    let item_models: Vec<ItemModel> = manager
        .items("")
        .iter()
        .map(|i| ItemModel {
            id: i.id.clone().into(),
            name: i.name.clone().into(),
            target: i.target.clone().into(),
            icon_path: i.icon_path.clone().into(),
        })
        .collect();
    ui.set_items(slint::VecModel::from(item_models).into());

    ui
}

#[test]
fn test_main_window_initializes() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert_eq!(ui.window().title(), "GoRun");
}

#[test]
fn test_categories_rendered() {
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Programs".into(),
        ..Default::default()
    }];
    let ui = setup_ui_with_data(categories, vec![]);
    let cats = ui.get_categories();
    assert_eq!(cats.row_count(), 1);
    let first = cats.row_data(0).unwrap();
    assert_eq!(first.name, "Programs");
}

#[test]
fn test_items_rendered() {
    let items = vec![Item {
        id: "item_1".into(),
        name: "Test Item".into(),
        target: "C:\\test.exe".into(),
        ..Default::default()
    }];
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Default".into(),
        ..Default::default()
    }];
    let ui = setup_ui_with_data(categories, items);
    let its = ui.get_items();
    assert_eq!(its.row_count(), 1);
    assert_eq!(its.row_data(0).unwrap().name, "Test Item");
}

#[test]
fn test_search_changed_callback() {
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Default".into(),
        ..Default::default()
    }];
    let items = vec![
        Item {
            id: "item_1".into(),
            name: "Notepad".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "item_2".into(),
            name: "Calculator".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
    ];
    let ui = setup_ui_with_data(categories, items);

    ui.set_search_text("Calc");
    ui.invoke_search_changed("Calc");
    assert_eq!(ui.get_search_text(), "Calc");
}

#[test]
fn test_toggle_view_callback() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(ui.get_is_icon_view());
    ui.invoke_toggle_view();
    assert!(!ui.get_is_icon_view());
    ui.invoke_toggle_view();
    assert!(ui.get_is_icon_view());
}

#[test]
fn test_toggle_theme_callback() {
    let ui = setup_ui_with_data(vec![], vec![]);
    let initial = ui.get_is_dark_theme();
    ui.invoke_toggle_theme();
    assert_ne!(ui.get_is_dark_theme(), initial);
}

#[test]
fn test_new_item_click_callback() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.invoke_new_item_clicked();
}

#[test]
fn test_category_selected_callback() {
    let categories = vec![
        Category {
            id: "cat_1".into(),
            name: "First".into(),
            ..Default::default()
        },
        Category {
            id: "cat_2".into(),
            name: "Second".into(),
            ..Default::default()
        },
    ];
    let ui = setup_ui_with_data(categories, vec![]);
    ui.invoke_category_selected(1);
}

#[test]
fn test_file_dropped_callback() {
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Default".into(),
        ..Default::default()
    }];
    let ui = setup_ui_with_data(categories, vec![]);
    ui.invoke_file_dropped("NewApp".into(), "C:\\newapp.exe".into());
}

#[test]
fn test_refresh_icons_callback() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.invoke_refresh_icons();
}

#[test]
fn test_multiple_categories_and_items() {
    let categories = (0..5)
        .map(|i| Category {
            id: format!("cat_{}", i),
            name: format!("Category {}", i),
            ..Default::default()
        })
        .collect();
    let items: Vec<Item> = (0..10)
        .map(|i| Item {
            id: format!("item_{}", i),
            name: format!("Item {}", i),
            target: format!("C:\\apps\\app{}.exe", i),
            category_id: format!("cat_{}", i % 5),
            ..Default::default()
        })
        .collect();

    let ui = setup_ui_with_data(categories, items);
    assert_eq!(ui.get_categories().row_count(), 5);
}

#[test]
fn test_empty_state_renders() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert_eq!(ui.get_items().row_count(), 0);
    assert_eq!(ui.get_categories().row_count(), 0);
}

#[test]
fn test_status_bar_initial_text() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert_eq!(ui.get_status_text(), "Ready");
}

#[test]
fn test_drop_active_flag() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_drop_active());
    ui.set_drop_active(true);
    assert!(ui.get_drop_active());
}
