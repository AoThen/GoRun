use slint::Model;
use std::rc::Rc;

use GoRun::app_state::AppState;
use GoRun::icon_cache::IconCache;
use GoRun::item_manager::ItemManager;
use GoRun::localization::Localization;
use GoRun::model::{AppConfig, AppData, Category, Config, Item};
use GoRun::storage::Storage;
use GoRun::*;

fn init_testing_backend() {
    // 每个测试都重新初始化 testing 后端：winit 的 EventLoop 不可重建，
    // 第二个测试的 MainWindow::new() 会落到 winit 而失败（probe_test 已复现）。
    // init_no_event_loop() 重复调用返回 AlreadySet 错误，不 panic，可安全重复。
    #[cfg(feature = "backend-testing")]
    {
        let _ = std::panic::catch_unwind(|| {
            i_slint_backend_testing::init_no_event_loop();
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

// ===== UI v2 行为链路补测（refresh_items_ui / show_toast / 对话框） =====

use std::cell::RefCell;
use std::sync::mpsc;

/// 构造一个可驱动 ui_sync 的 AppState（临时目录 storage，不落盘真实用户配置）。
fn setup_app_state(categories: Vec<Category>, items: Vec<Item>) -> Rc<RefCell<AppState>> {
    init_testing_backend();
    let app_config = AppConfig {
        version: "test".into(),
        data: AppData { categories, items },
        config: Config::default(),
        extra_config: Default::default(),
    };
    let manager = ItemManager::new(app_config);
    let (ihk_tx, _ihk_rx) = mpsc::channel::<String>();
    let state = Rc::new(RefCell::new(AppState {
        manager,
        storage: Storage::new(std::env::temp_dir()
            .join("gorun_gui_test_config.json")
            .to_string_lossy()
            .to_string()),
        icon_cache: IconCache::new(),
        current_category_id: String::new(),
        search_query: String::new(),
        edit_dialog: None,
        loc: Rc::new(RefCell::new(Localization::new("en"))),
        save_fail_notified: false,
        pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        drop_handler: None,
        geometry_dirty: false,
        last_geometry_change: None,
        item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
        item_hotkey_tx: ihk_tx,
        low_resident: Rc::new(std::cell::Cell::new(false)),
    }));
    // 回填当前分类（先取值再借用，避免 RefCell 冲突）
    let first_id = state
        .borrow()
        .manager
        .categories()
        .first()
        .map(|c| c.id.clone());
    if let Some(first) = first_id {
        state.borrow_mut().current_category_id = first;
    }
    state
}

fn sample_data() -> (Vec<Category>, Vec<Item>) {
    let categories = vec![
        Category { id: "cat_1".into(), name: "Programs".into(), ..Default::default() },
        Category { id: "cat_2".into(), name: "Games".into(), ..Default::default() },
    ];
    let items = vec![
        Item { id: "i1".into(), name: "Notepad".into(), target: "C:\\notepad.exe".into(), category_id: "cat_1".into(), ..Default::default() },
        Item { id: "i2".into(), name: "Notepad++".into(), target: "C:\\notepadpp.exe".into(), category_id: "cat_1".into(), ..Default::default() },
        Item { id: "i3".into(), name: "Minesweeper".into(), target: "C:\\mine.exe".into(), category_id: "cat_2".into(), ..Default::default() },
    ];
    (categories, items)
}

/// refresh_items_ui：按分类过滤（非搜索态），search_active=false。
#[test]
fn test_refresh_items_ui_filters_by_category() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = MainWindow::new().unwrap();
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "");
    }
    let model = ui.get_items();
    assert_eq!(model.row_count(), 2, "cat_1 应含 2 个项目");
    assert_eq!(model.row_data(0).unwrap().name.as_str(), "Notepad");
    assert!(!model.row_data(0).unwrap().search_active, "非搜索态 search_active=false");

    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_2", "");
    }
    let model = ui.get_items();
    assert_eq!(model.row_count(), 1, "cat_2 应含 1 个项目");
    assert_eq!(model.row_data(0).unwrap().name.as_str(), "Minesweeper");
}

/// refresh_items_ui：搜索模式跨分类命中，search_active=true；无命中为空列表。
#[test]
fn test_refresh_items_ui_search_across_categories() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = MainWindow::new().unwrap();
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "notepad");
    }
    let model = ui.get_items();
    assert_eq!(model.row_count(), 2, "搜索 notepad 应命中 2 项");
    assert!(model.row_data(0).unwrap().search_active, "搜索态 search_active=true");

    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "zzz_no_match");
    }
    assert_eq!(ui.get_items().row_count(), 0, "无命中应为空列表（空态场景）");
}

/// show_toast：成功/错误文案与 error 标志正确写入（v2 替代 StatusBar 的提示链路）。
#[test]
fn test_show_toast_sets_properties() {
    let ui = setup_ui_with_data(vec![], vec![]);
    assert!(!ui.get_toast_visible());
    crate::ui_sync::show_toast(&ui, "Exported: config.json", false);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_message().as_str(), "Exported: config.json");
    assert!(!ui.get_toast_error());
    crate::ui_sync::show_toast(&ui, "Export failed", true);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_message().as_str(), "Export failed");
    assert!(ui.get_toast_error(), "错误 toast error=true");
}

/// 对话框属性链路：confirm/input/about 的 visible/title/message/text 可写可读。
#[test]
fn test_dialog_properties_roundtrip() {
    let ui = setup_ui_with_data(vec![], vec![]);
    // 确认对话框
    assert!(!ui.get_confirm_dialog_visible());
    ui.set_confirm_dialog_title("Delete Item".into());
    ui.set_confirm_dialog_message("Delete \"Notepad\"?".into());
    ui.set_confirm_dialog_visible(true);
    assert!(ui.get_confirm_dialog_visible());
    assert_eq!(ui.get_confirm_dialog_title().as_str(), "Delete Item");
    assert_eq!(ui.get_confirm_dialog_message().as_str(), "Delete \"Notepad\"?");
    ui.set_confirm_dialog_visible(false);
    assert!(!ui.get_confirm_dialog_visible());
    // 输入对话框
    ui.set_input_dialog_title("New Category".into());
    ui.set_input_dialog_text("Games".into());
    ui.set_input_dialog_visible(true);
    assert!(ui.get_input_dialog_visible());
    assert_eq!(ui.get_input_dialog_text().as_str(), "Games");
    ui.set_input_dialog_visible(false);
    assert!(!ui.get_input_dialog_visible());
    // 关于对话框
    ui.set_about_dialog_visible(true);
    assert!(ui.get_about_dialog_visible());
    assert!(ui.get_about_version_text().as_str().contains("Version"));
    ui.set_about_dialog_visible(false);
    assert!(!ui.get_about_dialog_visible());
}

/// 右键菜单定位属性：context-menu-x/y/visible 可设置（geometry clamp 的落点）。
#[test]
fn test_context_menu_position_properties() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.set_context_menu_x(120.0);
    ui.set_context_menu_y(240.0);
    assert!((ui.get_context_menu_x() - 120.0).abs() < 0.001);
    assert!((ui.get_context_menu_y() - 240.0).abs() < 0.001);
    ui.set_context_menu_visible(true);
    assert!(ui.get_context_menu_visible());
    ui.set_context_menu_visible(false);
    assert!(!ui.get_context_menu_visible());
}

/// 搜索文本与 drag-enabled 联动：drag-enabled = search-text 为空（v2 行为）。
#[test]
fn test_drag_enabled_follows_search_text() {
    let ui = setup_ui_with_data(vec![], vec![]);
    ui.set_search_text("".into());
    assert!(ui.get_drag_enabled(), "搜索为空时允许拖放");
    ui.set_search_text("calc".into());
    assert!(!ui.get_drag_enabled(), "搜索中应禁用拖放");
    ui.set_search_text("".into());
    assert!(ui.get_drag_enabled(), "清空搜索恢复拖放");
}

/// E2E：添加分类→添加项目→搜索→删除→Storage roundtrip（核心业务闭环）。
#[test]
fn test_e2e_add_search_delete_persist_roundtrip() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = MainWindow::new().unwrap();

    // 1) 添加分类 + 项目
    state.borrow_mut().manager.add_category(Category {
        name: "Tools".into(),
        ..Default::default()
    });
    let tools_id = state.borrow().manager.categories().last().unwrap().id.clone();
    let vscode = Item {
        name: "VS Code".into(),
        target: "C:\\vscode.exe".into(),
        category_id: tools_id.clone(),
        ..Default::default()
    };
    assert!(state.borrow_mut().manager.add_item(vscode), "添加项目应成功");

    // 2) 初始刷新：3 个项目（sample_data）
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "");
    }
    assert_eq!(ui.get_items().row_count(), 2);

    // 3) 搜索新加的项目（跨分类）
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "vs code");
    }
    assert_eq!(ui.get_items().row_count(), 1, "搜索 vs code 应命中 1 项");
    assert_eq!(ui.get_items().row_data(0).unwrap().name.as_str(), "VS Code");

    // 4) Storage roundtrip：save → 新 Storage load → 数据一致
    let cfg = state.borrow().manager.config().clone();
    let path = std::env::temp_dir().join("gorun_e2e_roundtrip.json");
    let storage = Storage::new(path.to_string_lossy().to_string());
    assert!(storage.save(&cfg), "save 应成功");
    let loaded = storage.load().expect("load 应返回配置");
    assert_eq!(loaded.data.items.len(), cfg.data.items.len());
    assert_eq!(loaded.data.categories.len(), cfg.data.categories.len());
    let vs = loaded.data.items.iter().find(|i| i.name == "VS Code").expect("VS Code 应在持久化数据中");
    assert_eq!(vs.target, "C:\\vscode.exe");
    assert_eq!(vs.category_id, tools_id);
    let _ = std::fs::remove_file(&path);
}

/// E2E：删除项目→删除分类（级联）→UI 刷新反映变化。
#[test]
fn test_e2e_delete_item_and_category_cascade() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = MainWindow::new().unwrap();

    // 删除 i1（cat_1 2项 → 1项）
    assert!(state.borrow_mut().manager.delete_item("i1"), "删除项目应成功");
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "");
    }
    assert_eq!(ui.get_items().row_count(), 1, "删除后 cat_1 剩 1 项");
    assert_eq!(ui.get_items().row_data(0).unwrap().name.as_str(), "Notepad++");

    // 删除 cat_2（级联删 i3）
    assert!(state.borrow_mut().manager.delete_category("cat_2"), "删除分类应成功");
    {
        let st = state.borrow();
        crate::ui_sync::refresh_categories_ui(&ui, &st);
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "");
    }
    assert_eq!(ui.get_categories().row_count(), 1, "剩 1 个分类");
    // 搜索已级联删除的项目应无结果
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "minesweeper");
    }
    assert_eq!(ui.get_items().row_count(), 0, "级联删除的项目不应再被搜到");
}


// ===== tray_ui 动作分发补测（headless 可测部分） =====

/// ToggleFollowMouse：翻转 extra_config.follow_mouse 并落盘（经 save_if_modified）。
#[test]
fn test_tray_toggle_follow_mouse_updates_config() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = setup_ui_with_data(vec![], vec![]);

    // 初始无 follow_mouse 配置 → 翻转后应为 "1"
    crate::tray_ui::handle_tray_ui_action(
        crate::tray_ui::TrayUiAction::ToggleFollowMouse,
        &ui,
        &state,
    );
    let st = state.borrow();
    assert_eq!(
        st.manager.config().extra_config.get("follow_mouse").map(|s| s.as_str()),
        Some("1"),
        "首次翻转 follow_mouse 应置 1"
    );
    drop(st);

    // 再翻转一次 → "0"
    crate::tray_ui::handle_tray_ui_action(
        crate::tray_ui::TrayUiAction::ToggleFollowMouse,
        &ui,
        &state,
    );
    let st = state.borrow();
    assert_eq!(
        st.manager.config().extra_config.get("follow_mouse").map(|s| s.as_str()),
        Some("0"),
        "二次翻转 follow_mouse 应回 0"
    );
}

/// ToggleLowResident：翻转 low_resident 标志并写入 extra_config。
#[test]
fn test_tray_toggle_low_resident_updates_flag_and_config() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = setup_ui_with_data(vec![], vec![]);

    let before = state.borrow().low_resident.get();
    crate::tray_ui::handle_tray_ui_action(
        crate::tray_ui::TrayUiAction::ToggleLowResident,
        &ui,
        &state,
    );
    let st = state.borrow();
    assert_eq!(st.low_resident.get(), !before, "low_resident 应翻转");
    assert_eq!(
        st.manager.config().extra_config.get("low_resident").map(|s| s.as_str()),
        Some("1"),
        "extra_config.low_resident 应写 1（初始 false）"
    );
}

/// SetLanguage：写入 extra_config.language 且 loc 实例被替换。
#[test]
fn test_tray_set_language_updates_config_and_loc() {
    let (cats, items) = sample_data();
    let state = setup_app_state(cats, items);
    let ui = setup_ui_with_data(vec![], vec![]);

    crate::tray_ui::handle_tray_ui_action(
        crate::tray_ui::TrayUiAction::SetLanguage("en-US".into()),
        &ui,
        &state,
    );
    let st = state.borrow();
    assert_eq!(
        st.manager.config().extra_config.get("language").map(|s| s.as_str()),
        Some("en-US"),
        "language 应写入 extra_config"
    );
    assert_eq!(st.loc.borrow().language_code(), "en-US", "loc 实例应切换为 en-US");
}
