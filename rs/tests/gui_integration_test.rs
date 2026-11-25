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
    assert!(!ui.get_is_icon_view());
    ui.invoke_toggle_view();
    assert!(ui.get_is_icon_view());
    ui.invoke_toggle_view();
    assert!(!ui.get_is_icon_view());
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

/// 列表模式多列（用户反馈修复）：800×600 窗口 + 160px 固定列宽应得 ≥2 列横向排列。
#[test]
fn test_list_mode_multi_columns() {
    let categories = vec![Category {
        id: "cat_1".into(),
        name: "Default".into(),
        ..Default::default()
    }];
    let items: Vec<Item> = (0..12)
        .map(|i| Item {
            id: format!("item_{}", i),
            name: format!("Item {}", i),
            target: format!("C:\\app{}.exe", i),
            category_id: "cat_1".into(),
            ..Default::default()
        })
        .collect();
    let ui = setup_ui_with_data(categories, items);
    // 列表模式为默认视图
    assert!(!ui.get_is_icon_view());
    // 800×600：扣除侧栏 180px 与内边距后，(可用宽-16)/164 应 ≥2 列
    ui.window().set_size(slint::LogicalSize::new(800.0, 600.0));
    let cols = ui.get_item_columns();
    assert!(
        cols >= 2,
        "列表模式 800x600 下应横向 ≥2 列，实际 {} 列",
        cols
    );
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
    ui.set_context_menu_items(
        Rc::new(slint::VecModel::from(vec![
            slint::SharedString::from("Edit"),
            slint::SharedString::from("Delete"),
        ]))
        .into(),
    );
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
    // 必须走 wire 后的 UI：菜单填充逻辑在 wire_handler 的 on_hamburger_clicked
    let (ui, _state) = setup_wired_ui(vec![], vec![]);
    ui.invoke_hamburger_clicked();
    // 菜单文本与危险项数组必须平行构建（增删菜单项时二者同步变化）
    let items = ui.get_menu_popup_items();
    let danger = ui.get_menu_popup_danger();
    assert!(items.row_count() > 0, "汉堡菜单应至少有一项");
    assert_eq!(
        items.row_count(),
        danger.row_count(),
        "菜单文本与 danger 数组行数必须一致"
    );
    ui.invoke_menu_popup_selected(0);
    // 越界索引必须安全忽略（不 panic、不触发任何动作）
    ui.invoke_menu_popup_selected(23);
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
        storage: Storage::new(
            std::env::temp_dir()
                .join("gorun_gui_test_config.json")
                .to_string_lossy()
                .to_string(),
        ),
        icon_cache: IconCache::new(),
        current_category_id: String::new(),
        search_query: String::new(),
        edit_dialog: None,
        loc: Rc::new(RefCell::new(Localization::new("en"))),
        save_fail_notified: false,
        pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
        icon_extract_attempted: std::sync::Mutex::new(std::collections::HashSet::new()),
        drop_handler: None,
        geometry_dirty: false,
        last_geometry_change: None,
        soft_dirty: false,
        last_soft_change: None,
        pending_run_results: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
        item_hotkey_tx: ihk_tx,
        low_resident: Rc::new(std::cell::Cell::new(false)),
        auto_hide_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            target: "C:\\notepad.exe".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "i2".into(),
            name: "Notepad++".into(),
            target: "C:\\notepadpp.exe".into(),
            category_id: "cat_1".into(),
            ..Default::default()
        },
        Item {
            id: "i3".into(),
            name: "Minesweeper".into(),
            target: "C:\\mine.exe".into(),
            category_id: "cat_2".into(),
            ..Default::default()
        },
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
    assert!(
        !model.row_data(0).unwrap().search_active,
        "非搜索态 search_active=false"
    );

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
    assert!(
        model.row_data(0).unwrap().search_active,
        "搜索态 search_active=true"
    );

    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "zzz_no_match");
    }
    assert_eq!(
        ui.get_items().row_count(),
        0,
        "无命中应为空列表（空态场景）"
    );
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
    assert_eq!(
        ui.get_confirm_dialog_message().as_str(),
        "Delete \"Notepad\"?"
    );
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
    let tools_id = state
        .borrow()
        .manager
        .categories()
        .last()
        .unwrap()
        .id
        .clone();
    let vscode = Item {
        name: "VS Code".into(),
        target: "C:\\vscode.exe".into(),
        category_id: tools_id.clone(),
        ..Default::default()
    };
    assert!(
        state.borrow_mut().manager.add_item(vscode),
        "添加项目应成功"
    );

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
    let vs = loaded
        .data
        .items
        .iter()
        .find(|i| i.name == "VS Code")
        .expect("VS Code 应在持久化数据中");
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
    assert!(
        state.borrow_mut().manager.delete_item("i1"),
        "删除项目应成功"
    );
    {
        let st = state.borrow();
        crate::ui_sync::refresh_items_ui(&ui, &st, "cat_1", "");
    }
    assert_eq!(ui.get_items().row_count(), 1, "删除后 cat_1 剩 1 项");
    assert_eq!(
        ui.get_items().row_data(0).unwrap().name.as_str(),
        "Notepad++"
    );

    // 删除 cat_2（级联删 i3）
    assert!(
        state.borrow_mut().manager.delete_category("cat_2"),
        "删除分类应成功"
    );
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
        st.manager
            .config()
            .extra_config
            .get("follow_mouse")
            .map(|s| s.as_str()),
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
        st.manager
            .config()
            .extra_config
            .get("follow_mouse")
            .map(|s| s.as_str()),
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
        st.manager
            .config()
            .extra_config
            .get("low_resident")
            .map(|s| s.as_str()),
        Some("1"),
        "extra_config.low_resident 应写 1（初始 false）"
    );
}

// ===== P2-5 基线行为测试：锁定新建/编辑条目对话框行为（对话框装配重构前建立） =====

/// 构造已 wire_handler 的主窗口与状态（条目菜单/新建对话框回调可用）。
fn setup_wired_ui(
    categories: Vec<Category>,
    items: Vec<Item>,
) -> (MainWindow, Rc<RefCell<AppState>>) {
    let (ui, state, _, _) = setup_wired_ui_full(categories, items);
    (ui, state)
}

/// wire 后的完整句柄组：主窗口 + 状态 + 全局热键管理器 + 热键通知通道。
type WiredUiFull = (
    MainWindow,
    Rc<RefCell<AppState>>,
    Rc<RefCell<GoRun::hotkey::HotkeyManager>>,
    Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
);

/// wire 全量回调并额外返回热键句柄（导入配置等需直接操作热键管理器的测试用）。
fn setup_wired_ui_full(categories: Vec<Category>, items: Vec<Item>) -> WiredUiFull {
    let state = setup_app_state(categories.clone(), items.clone());
    let ui = setup_ui_with_data(categories, items);
    let loc = state.borrow().loc.clone();
    let hotkey_mgr = Rc::new(RefCell::new(GoRun::hotkey::HotkeyManager::new()));
    let (hk_tx, _rx) = mpsc::channel::<()>();
    let hk_shared = Rc::new(RefCell::new(Some(hk_tx)));
    GoRun::handlers::wire_handler(
        &ui,
        state.clone(),
        &loc,
        hotkey_mgr.clone(),
        hk_shared.clone(),
    );
    (ui, state, hotkey_mgr, hk_shared)
}

/// 取当前打开的编辑对话框（wire 后由对话框逻辑存入 state.edit_dialog，
/// Rc 共享句柄；方法调用经自动解引用）。
fn open_dialog(state: &Rc<RefCell<AppState>>) -> Rc<EditDialog> {
    state.borrow().edit_dialog.clone().expect("对话框应已打开")
}

/// 新建路径：save 成功后新增条目（进首个分类）、对话框关闭、状态清空。
#[test]
fn test_edit_dialog_new_save_adds_item() {
    let (cats, items) = sample_data();
    let ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items);
    let before = state.borrow().manager.all_items().len();

    GoRun::handlers::edit_dialog::show_edit_dialog(&ui.as_weak(), &state);
    let dlg = open_dialog(&state);
    dlg.set_item_name("New Entry".into());
    dlg.set_item_target("C:\\newapp.exe".into());
    dlg.invoke_save();

    let st = state.borrow();
    let after = st.manager.all_items();
    assert_eq!(after.len(), before + 1, "保存后应新增一条");
    let added = after.iter().find(|i| i.name == "New Entry").expect("新条目应在列表中");
    assert_eq!(added.target, "C:\\newapp.exe");
    assert!(
        st.manager.categories().iter().any(|c| c.id == added.category_id),
        "新条目应归属有效分类"
    );
    assert!(st.edit_dialog.is_none(), "保存后对话框状态应清空");
}

/// 新建路径：目标为空时校验拦截（错误提示、不新增、对话框保持打开）。
#[test]
fn test_edit_dialog_new_save_validation_blocks() {
    let (cats, items) = sample_data();
    let _ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items.clone());
    let before = state.borrow().manager.all_items().len();

    GoRun::handlers::edit_dialog::show_edit_dialog(
        &_ui.as_weak(),
        &state,
    );
    let dlg = open_dialog(&state);
    dlg.set_item_name("".into());
    dlg.invoke_save();
    assert!(!dlg.get_error_message().to_string().is_empty(), "空名称应有错误提示");
    let st = state.borrow();
    assert_eq!(st.manager.all_items().len(), before, "校验失败不应新增条目");
    assert!(st.edit_dialog.is_some(), "校验失败对话框应保持打开");
}

/// 新建路径：同分类重复 target 拦截（提示且不新增）。
#[test]
fn test_edit_dialog_new_duplicate_target_blocked() {
    let (cats, items) = sample_data();
    let _ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items.clone());
    let existing_target = items[0].target.clone();
    let before = state.borrow().manager.all_items().len();

    GoRun::handlers::edit_dialog::show_edit_dialog(
        &_ui.as_weak(),
        &state,
    );
    let dlg = open_dialog(&state);
    dlg.set_item_name("Dup".into());
    dlg.set_item_target(existing_target.clone().into());
    dlg.invoke_save();
    assert!(
        !dlg.get_error_message().to_string().is_empty(),
        "同分类重复 target 应提示"
    );
    let st = state.borrow();
    assert_eq!(st.manager.all_items().len(), before, "重复 target 不应新增条目");
}

/// 编辑路径：右键菜单 Edit（首项）打开对话框，条目字段预填，save 后更新。
#[test]
fn test_edit_dialog_edit_save_updates_item() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items.clone());
    ui.invoke_item_right_clicked(0, 0.0, 0.0);
    ui.invoke_menu_item_selected(0); // 条目菜单首项 = Edit
    let dlg = open_dialog(&state);
    let orig = &items[0];
    assert_eq!(dlg.get_item_name().to_string(), orig.name, "应预填条目前名称");
    assert_eq!(
        dlg.get_item_target().to_string(),
        orig.target,
        "应预填条目前目标"
    );
    dlg.set_item_name("Renamed".into());
    dlg.set_item_target("C:\\renamed.exe".into());
    dlg.invoke_save();

    let st = state.borrow();
    let all = st.manager.all_items();
    let updated = all
        .iter()
        .find(|i| i.id == orig.id)
        .expect("原条目应仍存在（编辑非新增）");
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.target, "C:\\renamed.exe");
    assert!(st.edit_dialog.is_none());
}

/// 编辑路径：改到同分类其他条目已有 target 时拦截（条目不变、对话框保持打开）。
#[test]
fn test_edit_dialog_edit_duplicate_target_blocked() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items.clone());
    // sample_data 两个条目（i1/i2）同在 cat_1 且 target 不同
    let target_of_second = items[1].target.clone();
    ui.invoke_item_right_clicked(0, 0.0, 0.0);
    ui.invoke_menu_item_selected(0);
    let dlg = open_dialog(&state);
    dlg.set_item_target(target_of_second.clone().into());
    dlg.invoke_save();
    assert!(
        !dlg.get_error_message().to_string().is_empty(),
        "重复 target 应提示"
    );
    let st = state.borrow();
    let all = st.manager.all_items();
    let first = all
        .iter()
        .find(|i| i.id == items[0].id)
        .unwrap();
    assert_eq!(first.target, items[0].target, "拦截后原 target 不应被改动");
}

/// 取消/关闭路径：cancel 清空对话框状态且不改动条目。
#[test]
fn test_edit_dialog_cancel_clears_state() {
    let (cats, items) = sample_data();
    let _ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items.clone());
    let before = state.borrow().manager.all_items();

    GoRun::handlers::edit_dialog::show_edit_dialog(
        &_ui.as_weak(),
        &state,
    );
    let dlg = open_dialog(&state);
    dlg.invoke_cancel();
    let st = state.borrow();
    assert!(st.edit_dialog.is_none(), "取消后对话框状态应清空");
    assert_eq!(st.manager.all_items(), before, "取消不应改动条目");
}

// ===== H-1/H-2 回归：分类切换必须保证"显示集合 ≡ 运行取项集合" =====

/// keep_search=false（导入配置/新建分类语义）：切换分类必须清空搜索态
/// （state.search_query 与搜索框文本同步清空），界面显示新分类全量条目。
/// 否则显示集合（新分类全量）与运行取项集合（旧搜索结果）索引错位，
/// 可能"点 A 却启动 B"。
#[test]
fn test_switch_category_reset_search_consistency() {
    let (cats, items) = sample_data();
    let ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items);
    // 进入搜索态（"Notepad" 命中 cat_1 的 i1/i2）
    state.borrow_mut().search_query = "Notepad".into();
    ui.set_search_text("Notepad".into());
    // 模拟导入配置/新建分类后的切换：keep_search=false
    GoRun::ui_sync::switch_category(&ui, &state, "cat_2", false);
    {
        let st = state.borrow();
        assert!(st.search_query.is_empty(), "切换后必须清空搜索态（H-1/H-2）");
        assert_eq!(st.current_category_id, "cat_2");
    }
    assert_eq!(ui.get_search_text(), "", "搜索框文本须与搜索态同步清空");
    // 一致性：界面显示集合 == run_item_at_index 将取的集合（cat_2 全量）
    let shown = ui.get_items().row_count();
    let runnable = state.borrow().manager.items("cat_2").len();
    assert_eq!(shown, runnable, "显示集合必须与运行取项集合一致");
    assert_eq!(ui.get_sidebar_selected_index(), 1, "侧栏选中态应同步到 cat_2");
}

/// keep_search=true（侧栏点击/滚轮切页语义）：保留搜索态，列表显示搜索结果，
/// 与 run_item_at_index 的搜索取项集合一致。
#[test]
fn test_switch_category_keep_search_consistency() {
    let (cats, items) = sample_data();
    let ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items);
    state.borrow_mut().search_query = "Notepad".into();
    ui.set_search_text("Notepad".into());
    GoRun::ui_sync::switch_category(&ui, &state, "cat_2", true);
    {
        let st = state.borrow();
        assert_eq!(st.search_query, "Notepad", "keep_search=true 不得清空搜索态");
        assert_eq!(st.current_category_id, "cat_2");
    }
    assert_eq!(ui.get_search_text(), "Notepad");
    let shown = ui.get_items().row_count();
    let runnable = state.borrow().manager.search_items("Notepad").len();
    assert_eq!(shown, runnable, "搜索态下显示集合必须与运行取项集合一致");
}

/// clear_search_and_refresh（Esc 语义）：清空搜索态并显示当前分类全量条目。
#[test]
fn test_clear_search_and_refresh() {
    let (cats, items) = sample_data();
    let ui = setup_ui_with_data(cats.clone(), items.clone());
    let state = setup_app_state(cats, items);
    state.borrow_mut().search_query = "Notepad".into();
    ui.set_search_text("Notepad".into());
    GoRun::ui_sync::clear_search_and_refresh(&ui, &state);
    assert!(state.borrow().search_query.is_empty(), "Esc 后搜索态应清空");
    assert_eq!(ui.get_search_text(), "", "Esc 后搜索框应同步清空");
    let shown = ui.get_items().row_count();
    let cat_id = state.borrow().current_category_id.clone();
    let runnable = state.borrow().manager.items(&cat_id).len();
    assert_eq!(shown, runnable, "清搜索后应显示当前分类全量条目");
}

// ===== 确认框分支回归（P0：防误操作核心路径） =====

/// 在当前弹出的菜单 danger 数组中定位首个危险项索引。
/// 菜单项随状态增删、文案随语言资源变化，禁止硬编码索引：
/// 项目菜单仅 Delete 为 danger；汉堡菜单 danger 含 DelCategory（前）与
/// Exit（末位），首个 danger 即"删除当前分类"。
fn find_first_danger_index(ui: &MainWindow) -> i32 {
    let danger = ui.get_menu_popup_danger();
    for i in 0..danger.row_count() {
        if danger.row_data(i).unwrap() {
            return i as i32;
        }
    }
    panic!("菜单中未找到 danger 项");
}

/// 项目右键菜单（context-menu 系列）的 danger 定位：仅 Delete 为 danger。
fn find_context_danger_index(ui: &MainWindow) -> i32 {
    let danger = ui.get_context_menu_danger();
    for i in 0..danger.row_count() {
        if danger.row_data(i).unwrap() {
            return i as i32;
        }
    }
    panic!("项目菜单中未找到 danger 项");
}

/// 删除条目：右键菜单 Delete → 确认框 ok → 条目删除。
#[test]
fn test_confirm_delete_item_removes_item() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;
    let before = state.borrow().manager.all_items().len();

    ui.invoke_item_right_clicked(0, 10.0, 10.0);
    let idx = find_context_danger_index(&ui);
    ui.invoke_menu_item_selected(idx);
    assert!(ui.get_confirm_dialog_visible(), "删除确认框应弹出");
    ui.invoke_confirm_dialog_ok();

    assert_eq!(
        state.borrow().manager.all_items().len(),
        before - 1,
        "确认后条目应被删除"
    );
}

/// 删除条目确认框：取消（cancel）不得改动数据。
#[test]
fn test_confirm_cancel_keeps_data() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;
    let before = state.borrow().manager.all_items().len();

    ui.invoke_item_right_clicked(0, 10.0, 10.0);
    let idx = find_context_danger_index(&ui);
    ui.invoke_menu_item_selected(idx);
    ui.invoke_confirm_dialog_cancel();

    assert_eq!(
        state.borrow().manager.all_items().len(),
        before,
        "取消后条目不得被删除"
    );
}

/// take-once 语义：ok 只消费一次 pending，重复 ok 无二次效果。
#[test]
fn test_confirm_ok_twice_has_single_effect() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;
    let before = state.borrow().manager.all_items().len();

    ui.invoke_item_right_clicked(0, 10.0, 10.0);
    let idx = find_context_danger_index(&ui);
    ui.invoke_menu_item_selected(idx);
    ui.invoke_confirm_dialog_ok();
    let after_first = state.borrow().manager.all_items().len();
    assert_eq!(after_first, before - 1, "首次 ok 应删除一条");

    ui.invoke_confirm_dialog_ok();
    assert_eq!(
        state.borrow().manager.all_items().len(),
        after_first,
        "pending 已 take，重复 ok 不得二次删除"
    );
}

// ===== "移动到..."折叠子菜单（下钻式）回归 =====

/// 在当前弹出的项目菜单（context-menu-items）中按文本定位索引。
fn find_context_item_index(ui: &MainWindow, text: &str) -> i32 {
    let items = ui.get_context_menu_items();
    for i in 0..items.row_count() {
        if items.row_data(i).unwrap().as_str() == text {
            return i as i32;
        }
    }
    panic!("项目菜单中未找到项: {}", text);
}

/// 下钻流程：右键 → 点"移动到..." → 菜单不关闭且换为 [返回, 目标分类]
/// → 点返回恢复一级 → 再下钻点目标分类 → 条目归属变更。
#[test]
fn test_move_to_submenu_drilldown_and_back() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items.clone());
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;

    // 右键 cat_1 的第一个条目（Notepad，属 cat_1）。
    // invoke 直达 Rust 回调；真实 UI 的 visible=true 由 .slint 侧 ItemGrid
    // handler 设置（invoke 不经过该链路），此处先模拟打开动作。
    ui.set_context_menu_visible(true);
    ui.invoke_item_right_clicked(0, 10.0, 10.0);
    // 一级菜单末位为折叠入口，不再随分类数平铺
    let entry_idx = find_context_item_index(&ui, "移动到...");
    assert!(ui.get_context_menu_visible(), "右键后菜单应可见");

    // 下钻：菜单保持打开，内容换为 [返回, Games]（排除条目当前分类 cat_1）
    ui.invoke_menu_item_selected(entry_idx);
    assert!(ui.get_context_menu_visible(), "下钻后菜单应保持可见");
    assert_eq!(
        find_context_item_index(&ui, "返回"),
        0,
        "二级菜单首项应为返回"
    );
    let games_idx = find_context_item_index(&ui, "Games");
    assert!(games_idx > 0, "二级菜单应含目标分类 Games");

    // 返回：恢复一级菜单（折叠入口仍在，二级菜单内容已不在）
    let back_idx = find_context_item_index(&ui, "返回");
    ui.invoke_menu_item_selected(back_idx);
    assert!(ui.get_context_menu_visible(), "返回后菜单应保持可见");
    let _ = find_context_item_index(&ui, "移动到...");
    let _ = find_context_item_index(&ui, "编辑");
    let items_model = ui.get_context_menu_items();
    for i in 0..items_model.row_count() {
        assert_ne!(
            items_model.row_data(i).unwrap().as_str(),
            "返回",
            "返回一级后不应再有返回项"
        );
    }

    // 再次下钻并选择目标分类：条目移动到 cat_2
    let entry_idx2 = find_context_item_index(&ui, "移动到...");
    ui.invoke_menu_item_selected(entry_idx2);
    let games_idx2 = find_context_item_index(&ui, "Games");
    ui.invoke_menu_item_selected(games_idx2);
    assert!(
        !ui.get_context_menu_visible(),
        "选择目标分类后菜单应关闭"
    );
    let moved = state
        .borrow()
        .manager
        .all_items()
        .into_iter()
        .find(|it| it.id == items[0].id)
        .expect("条目应仍存在");
    assert_eq!(moved.category_id, "cat_2", "条目应移入 Games 分类");
}

/// 单分类时一级菜单不显示"移动到..."（无可移动目标）。
#[test]
fn test_move_to_submenu_hidden_with_single_category() {
    let (mut cats, items) = sample_data();
    cats.truncate(1);
    let items: Vec<Item> = items
        .into_iter()
        .filter(|it| it.category_id == cats[0].id)
        .collect();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;

    ui.invoke_item_right_clicked(0, 10.0, 10.0);
    let items_model = ui.get_context_menu_items();
    for i in 0..items_model.row_count() {
        assert_ne!(
            items_model.row_data(i).unwrap().as_str(),
            "移动到...",
            "单分类不应出现移动到入口"
        );
    }
}

/// 至少保留一个分类：删除最后分类被拒，Toast 提示且数据不变。
#[test]
fn test_confirm_delete_last_category_rejected() {
    let (mut cats, items) = sample_data();
    cats.truncate(1);
    let items: Vec<Item> = items
        .into_iter()
        .filter(|it| it.category_id == cats[0].id)
        .collect();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;
    let cats_before = state.borrow().manager.categories().len();

    ui.invoke_hamburger_clicked();
    let idx = find_first_danger_index(&ui);
    ui.invoke_menu_popup_selected(idx);
    assert!(ui.get_confirm_dialog_visible(), "删除分类确认框应弹出");
    ui.invoke_confirm_dialog_ok();

    assert_eq!(
        state.borrow().manager.categories().len(),
        cats_before,
        "最后分类不可删除"
    );
    assert!(ui.get_toast_visible(), "拒绝时应弹出 Toast 提示");
    assert_eq!(ui.get_toast_message().as_str(), "至少需要保留一个分类");
}

// ===== 业务路径补测（P1：导入/拖入/滚轮/重命名） =====

/// 按文本在汉堡菜单中定位条目索引（非 danger 项无法用 danger 数组定位）。
fn find_menu_text_index(ui: &MainWindow, text: &str) -> i32 {
    let items = ui.get_menu_popup_items();
    for i in 0..items.row_count() {
        if items.row_data(i).unwrap().as_str() == text {
            return i as i32;
        }
    }
    panic!("汉堡菜单中未找到条目: {}", text);
}

/// import_config_apply：整体替换数据 + 孤儿归首分类 + 会话偏好保留 +
/// 搜索态清空 + 当前分类归一（H-1）。
#[test]
fn test_import_config_apply_replaces_and_preserves_prefs() {
    let (cats, items) = sample_data();
    let (ui, state, hk_mgr, hk_tx) = setup_wired_ui_full(cats, items);
    {
        let mut st = state.borrow_mut();
        st.current_category_id = "cat_2".into();
        st.search_query = "Notepad".into();
        st.manager
            .config_mut()
            .extra_config
            .insert("follow_mouse".into(), "1".into());
    }

    // 导入配置：1 个新分类 + 1 个指向不存在分类的孤儿条目 + 不同的全局热键
    let imported = AppConfig {
        version: "9.9.9".into(),
        data: AppData {
            categories: vec![Category {
                id: "c_new".into(),
                name: "Imported".into(),
                ..Default::default()
            }],
            items: vec![Item {
                id: "it_new".into(),
                name: "New".into(),
                target: "C:\\x.exe".into(),
                category_id: "c_missing".into(),
                ..Default::default()
            }],
        },
        config: Config {
            global_hotkey: "Ctrl+Shift+G".into(),
            ..Default::default()
        },
        extra_config: Default::default(),
    };
    GoRun::handlers::config_io::import_config_apply(
        &ui, &state, &hk_mgr, &hk_tx, "test.json", imported,
    );

    let st = state.borrow();
    assert_eq!(st.manager.all_items().len(), 1, "数据应整体替换");
    assert_eq!(st.manager.categories().len(), 1);
    assert_eq!(st.manager.categories()[0].id, "c_new");
    assert_eq!(
        st.manager.all_items()[0].category_id, "c_new",
        "孤儿条目应归入首分类"
    );
    assert_eq!(
        st.manager.config().extra_config.get("follow_mouse").map(String::as_str),
        Some("1"),
        "会话偏好（跟随鼠标）应保留"
    );
    assert!(st.search_query.is_empty(), "导入后搜索态应清空（H-1）");
    assert_eq!(st.current_category_id, "c_new", "当前分类应归一为首分类");
}

/// 拖入添加幂等：重复 target 第二次拖入不重复添加。
#[test]
fn test_file_dropped_duplicate_target_idempotent() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;
    let before = state.borrow().manager.all_items().len();

    ui.invoke_file_dropped("NewApp".into(), "C:\\newapp.exe".into());
    let after_first = state.borrow().manager.all_items().len();
    assert_eq!(after_first, before + 1, "首次拖入应新增条目");

    ui.invoke_file_dropped("NewApp".into(), "C:\\newapp.exe".into());
    assert_eq!(
        state.borrow().manager.all_items().len(),
        after_first,
        "重复 target 应幂等跳过"
    );
}

/// 滚轮切页首尾回绕：首分类向上滚到末分类，末分类向下滚回首分类。
#[test]
fn test_wheel_switch_wraps_around() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    state.borrow_mut().current_category_id = "cat_1".into();

    ui.invoke_wheel_switch(-1);
    assert_eq!(
        state.borrow().current_category_id, "cat_2",
        "首分类向上滚应回绕到末分类"
    );
    ui.invoke_wheel_switch(1);
    assert_eq!(
        state.borrow().current_category_id, "cat_1",
        "末分类向下滚应回绕回首分类"
    );
}

/// 分类重命名：空名（含纯空白）拒绝并保持对话框打开；正常名生效落盘。
#[test]
fn test_rename_category_empty_name_rejected() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    state.borrow_mut().current_category_id = "cat_1".into();

    ui.invoke_hamburger_clicked();
    let idx = find_menu_text_index(&ui, "重命名当前分类");
    ui.invoke_menu_popup_selected(idx);
    assert!(ui.get_input_dialog_visible(), "重命名对话框应打开");

    ui.set_input_dialog_text("   ".into());
    ui.invoke_input_dialog_ok();
    assert_eq!(
        state.borrow().manager.categories()[0].name,
        "Programs",
        "空名应被拒绝，名称不变"
    );
    assert!(ui.get_input_dialog_visible(), "空名拒绝后对话框应保持打开");

    ui.set_input_dialog_text("Renamed".into());
    ui.invoke_input_dialog_ok();
    assert_eq!(
        state.borrow().manager.categories()[0].name, "Renamed",
        "正常名应生效"
    );
}

/// 设置页：日志档位切换写 extra_config（0=warn 1=info 2=debug）。
#[test]
fn test_settings_log_changed_persists() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);

    ui.invoke_settings_log_changed(2);
    assert_eq!(
        state
            .borrow()
            .manager
            .config()
            .extra_config
            .get("log_level")
            .map(String::as_str),
        Some("debug"),
        "档位 2 应写入 debug"
    );

    ui.invoke_settings_log_changed(0);
    assert_eq!(
        state
            .borrow()
            .manager
            .config()
            .extra_config
            .get("log_level")
            .map(String::as_str),
        Some("warn"),
        "档位 0 应写入 warn"
    );
}

/// 设置页：自动隐藏开关同步写 extra_config 与运行时共享标志。
#[test]
fn test_settings_auto_hide_toggled_updates_flag() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);

    ui.invoke_settings_auto_hide_toggled(true);
    {
        let st = state.borrow();
        assert!(
            st.auto_hide_flag.load(std::sync::atomic::Ordering::Relaxed),
            "开启后共享标志应为 true"
        );
        assert_eq!(
            st.manager.config().extra_config.get("auto_hide").map(String::as_str),
            Some("1"),
            "开启后 extra_config 应记录 1"
        );
    }

    ui.invoke_settings_auto_hide_toggled(false);
    let st = state.borrow();
    assert!(!st.auto_hide_flag.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(
        st.manager.config().extra_config.get("auto_hide").map(String::as_str),
        Some("0")
    );
}

// ===== 运行全部：双重确认（P0 防误批量启动） =====

/// 运行全部双重确认：第一道确认 → ok → 第二道确认（含数量）；
/// 取消第二道 → 关闭且任何条目 run_count 不变（未执行）。
#[test]
fn test_run_all_double_confirm_and_cancel() {
    let (cats, items) = sample_data();
    let (ui, state) = setup_wired_ui(cats, items);
    let first_cat = state.borrow().manager.categories()[0].id.clone();
    state.borrow_mut().current_category_id = first_cat;

    // 背景右键弹出菜单，按文本定位"运行全部"（禁止硬编码索引）
    ui.invoke_background_right_clicked(10.0, 10.0);
    let idx = find_context_item_index(&ui, "运行全部");
    ui.invoke_menu_item_selected(idx);

    // 第一道确认：运行分类提示
    assert!(ui.get_confirm_dialog_visible(), "第一道确认应弹出");
    assert!(
        ui.get_confirm_dialog_message().as_str().contains("运行分类"),
        "第一道确认应提示运行分类"
    );
    ui.invoke_confirm_dialog_ok();

    // 第二道确认：含将运行的项目数量
    assert!(ui.get_confirm_dialog_visible(), "第二道确认应弹出");
    let msg2 = ui.get_confirm_dialog_message().as_str().to_string();
    assert!(
        msg2.contains("连续运行") && msg2.contains("2 个项目"),
        "第二道确认应含数量与连续运行提示，实际：{msg2}"
    );

    // 取消第二道（真实流程：组件先关弹窗再回调取消）→ 未执行任何条目
    ui.set_confirm_dialog_visible(false);
    ui.invoke_confirm_dialog_cancel();
    assert!(!ui.get_confirm_dialog_visible(), "取消后确认框应关闭");
    let ran: i32 = state
        .borrow()
        .manager
        .all_items()
        .iter()
        .map(|i| i.run_count)
        .sum();
    assert_eq!(ran, 0, "取消后不得执行任何条目");
}
