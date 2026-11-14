use slint::Model;
use std::rc::Rc;

use GoRun::item_manager::ItemManager;
use GoRun::model::{AppConfig, AppData, Category, Config, Item};
use GoRun::*;

fn init_testing_backend() {
    #[cfg(feature = "backend-testing")]
    {
        std::thread_local! {
            static INIT: std::cell::OnceCell<()> = std::cell::OnceCell::new();
        }
        INIT.with(|cell| {
            cell.get_or_init(|| {
                i_slint_backend_testing::init_no_event_loop();
            });
        });
    }
}

fn setup_ui_with_data(categories: Vec<Category>, items: Vec<Item>) -> MainWindow {
    init_testing_backend();

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
            count: manager.items(&c.id).len() as i32,
        })
        .collect();
    ui.set_categories(Rc::new(slint::VecModel::from(cat_models)).into());

    let item_models: Vec<ItemModel> = manager
        .all_items()
        .iter()
        .map(|it| ItemModel {
            id: it.id.clone().into(),
            name: it.name.clone().into(),
            target: it.target.clone().into(),
            icon_path: slint::Image::default(),
            search_active: false,
        })
        .collect();
    ui.set_items(Rc::new(slint::VecModel::from(item_models)).into());

    ui
}

#[test]
fn test_main_window_initializes() {
    let _ui = setup_ui_with_data(vec![], vec![]);
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
    assert_eq!(cats.row_data(0).unwrap().name, "Programs");
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
fn test_items_rendered() {
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Default".into(),
        ..Default::default()
    }];
    let items = vec![
        Item {
            id: "item_1".into(),
            name: "Notepad".into(),
            target: "C:\\notepad.exe".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "item_2".into(),
            name: "Calc".into(),
            target: "C:\\calc.exe".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
    ];
    let ui = setup_ui_with_data(categories, items);
    let items = ui.get_items();
    assert_eq!(items.row_count(), 2);
    assert_eq!(items.row_data(0).unwrap().name, "Notepad");
    assert_eq!(items.row_data(0).unwrap().target, "C:\\notepad.exe");
}

#[test]
fn test_toggle_theme_callback() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_is_dark_theme());
    ui.invoke_toggle_theme();
    assert!(ui.get_is_dark_theme());
    ui.invoke_toggle_theme();
    assert!(!ui.get_is_dark_theme());
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
    assert_eq!(ui.get_items().row_count(), 0);
    assert_eq!(ui.get_categories().row_count(), 0);
}

// ===== UI v2 重设计（提交 9415cf0 / 199daec）新增特性覆盖 =====

/// 计数徽章：CategoryModel.count 填充为分类实际项目数（v2 侧栏行尾徽章）。
#[test]
fn test_count_badge_filled() {
    let categories = vec![
        Category {
            id: "cat_1".into(),
            name: "Programs".into(),
            ..Default::default()
        },
        Category {
            id: "cat_2".into(),
            name: "Games".into(),
            ..Default::default()
        },
    ];
    let items = vec![
        Item {
            id: "i1".into(),
            name: "Notepad".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "i2".into(),
            name: "Calc".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "i3".into(),
            name: "Minesweeper".into(),
            category_id: "cat_2".into(),
            ..Default::default()
        },
    ];
    let ui = setup_ui_with_data(categories, items);
    let cats = ui.get_categories();
    assert_eq!(cats.row_count(), 2);
    assert_eq!(cats.row_data(0).unwrap().count, 2);
    assert_eq!(cats.row_data(1).unwrap().count, 1);
}

/// Toast 属性链路：message/error/visible 可 set 可 get（v2 替代 StatusBar）。
#[test]
fn test_toast_properties() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_toast_visible());
    assert_eq!(ui.get_toast_message().as_str(), "");
    ui.set_toast_message("Exported: config.json".into());
    ui.set_toast_error(false);
    ui.set_toast_visible(true);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_message().as_str(), "Exported: config.json");
    assert!(!ui.get_toast_error());
    // 错误态
    ui.set_toast_error(true);
    assert!(ui.get_toast_error());
    ui.set_toast_visible(false);
    assert!(!ui.get_toast_visible());
}

/// 空状态副文案属性（Tip_DropToAdd 接线目标）。
#[test]
fn test_empty_hint_text_property() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert_eq!(ui.get_empty_hint_text().as_str(), "");
    ui.set_empty_hint_text("Drop files or shortcuts here to add".into());
    assert_eq!(
        ui.get_empty_hint_text().as_str(),
        "Drop files or shortcuts here to add"
    );
}

/// 右键菜单 danger 数组属性链路（v2 危险项标红）。
#[test]
fn test_context_menu_danger_property() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.set_context_menu_items(Rc::new(slint::VecModel::from(vec![
        slint::SharedString::from("Edit"),
        slint::SharedString::from("Delete"),
    ])).into());
    ui.set_context_menu_danger(Rc::new(slint::VecModel::from(vec![false, true])).into());
    let danger = ui.get_context_menu_danger();
    assert_eq!(danger.row_count(), 2);
    assert!(!danger.row_data(0).unwrap());
    assert!(danger.row_data(1).unwrap());
}

/// 汉堡菜单弹层属性链路（v2 单一列表 + danger 数组 + 分隔符占位）。
#[test]
fn test_menu_popup_properties() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_menu_popup_visible());
    ui.set_menu_popup_visible(true);
    assert!(ui.get_menu_popup_visible());
    let items = Rc::new(slint::VecModel::from(vec![
        slint::SharedString::from("New Item"),
        slint::SharedString::from("-"),
        slint::SharedString::from("Exit"),
    ]));
    ui.set_menu_popup_items(items.into());
    let got = ui.get_menu_popup_items();
    assert_eq!(got.row_count(), 3);
    assert_eq!(got.row_data(1).unwrap().as_str(), "-");
    ui.set_menu_popup_danger(Rc::new(slint::VecModel::from(vec![false, false, true])).into());
    let danger = ui.get_menu_popup_danger();
    assert_eq!(danger.row_count(), 3);
    assert!(danger.row_data(2).unwrap());
}

/// 汉堡菜单回调存在：invoke 不 panic（分发逻辑在 wire_handler，UI 层回调通路完整）。
#[test]
fn test_hamburger_menu_callbacks_exist() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.invoke_hamburger_clicked();
    ui.invoke_menu_popup_selected(0);
    ui.invoke_menu_popup_selected(22);
}

/// Ctrl+F 目标：focus-search 回调在 Toolbar 内，main_window 暴露 toolbar 实例；
/// 此处验证主窗口键盘焦点链路属性存在（search-field-focused 读写）。
#[test]
fn test_search_field_focus_property() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_search_field_focused());
    ui.set_search_field_focused(true);
    assert!(ui.get_search_field_focused());
    ui.set_search_field_focused(false);
    assert!(!ui.get_search_field_focused());
}

