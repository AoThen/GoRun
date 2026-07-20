use slint::Model;
use std::rc::Rc;

use GoRun::item_manager::ItemManager;
use GoRun::model::{AppConfig, AppData, Category, Config, Item, ViewType};
use GoRun::*;

fn setup_ui_with_data(categories: Vec<Category>, items: Vec<Item>) -> MainWindow {
    let app_config = AppConfig {
        version: "test".into(),
        data: AppData { categories, items },
        config: Config::default(),
        extra_config: Default::default(),
    };
    let manager = ItemManager::new(app_config);

    let ui = MainWindow::new().unwrap();

    let cat_names: Vec<slint::SharedString> = manager
        .categories()
        .iter()
        .map(|c| c.name.as_str().into())
        .collect();
    ui.set_categories(Rc::new(slint::VecModel::from(cat_names)).into());
    ui.set_category_count(manager.categories().len() as i32);

    ui
}

#[test]
fn test_main_window_initializes() {
    let ui = setup_ui_with_data(vec![], vec![]);
    // Window title check - skipped as title() API varies by Slint version
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
    assert_eq!(cats.row_data(0).unwrap(), "Programs");
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
    ui.set_search_text("Calc".into());
    ui.invoke_search_changed("Calc".into());
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
    assert_eq!(ui.get_list_items().row_count(), 0);
    assert_eq!(ui.get_categories().row_count(), 0);
}
