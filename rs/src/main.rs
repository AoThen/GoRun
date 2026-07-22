#![allow(non_snake_case)]

mod config;
mod hotkey;
mod icon_cache;
mod item_manager;
mod localization;
mod logger;
mod model;
mod platform;
mod runner;
mod storage;
mod tray;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use item_manager::ItemManager;
use model::{Category, Item, ViewType};
use storage::Storage;

include!(env!("SLINT_INCLUDE_GENERATED"));
include!(env!("SLINT_INCLUDE_GENERATED_EDIT_DIALOG"));

struct AppState {
    manager: ItemManager,
    current_category_id: String,
    current_item_index: Option<i32>,
    search_query: String,
}

fn sanitize_for_log(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

fn main() {
    logger::init().ok();
    logger::set_panic_hook();
    log::info!("GoRun starting...");

    let storage = Storage::new(
        config::config_path()
            .to_str()
            .unwrap_or("config.json")
            .to_string(),
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
    let icon_cache = icon_cache::IconCache::new();
    let _loc = localization::Localization::new("zh-CN");

    let _tray = tray::TrayIcon::create(0, "GoRun");

    let current_category_id = manager
        .categories()
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let state = Rc::new(RefCell::new(AppState {
        manager,
        current_category_id: current_category_id.clone(),
        current_item_index: None,
        search_query: String::new(),
    }));

    let ui = MainWindow::new().unwrap();
    log::info!("MainWindow created successfully");

    let categories: Vec<CategoryModel> = state
        .borrow()
        .manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
        })
        .collect();
    let category_model = Rc::new(slint::VecModel::from(categories));
    ui.set_categories(category_model.into());

    refresh_items_ui(&ui, &state.borrow(), &current_category_id, "");

    wire_handler(&ui, state.clone(), icon_cache);

    setup_drop_handler(&ui);

    log::info!("Entering main event loop");
    ui.run().unwrap();
    log::info!("Main event loop ended");

    let state = state.borrow();
    if state.manager.is_modified() {
        let _ = storage.save(state.manager.config());
    }
}

fn wire_handler(ui: &MainWindow, state: Rc<RefCell<AppState>>, icon_cache: icon_cache::IconCache) {
    let right_clicked_index: Rc<std::cell::Cell<Option<i32>>> = Rc::new(std::cell::Cell::new(None));

    let state_file = state.clone();
    let weak_ui_file = ui.as_weak();
    ui.on_file_dropped(move |name, target| {
        log::info!(
            "File dropped: name={}, target={}",
            sanitize_for_log(&name),
            sanitize_for_log(&target)
        );
        let path = PathBuf::from(target.to_string());
        let mut state = state_file.borrow_mut();
        let cat_id = state.current_category_id.clone();
        let count = state.manager.handle_drop(&[path], &cat_id);
        let query = state.search_query.clone();
        drop(state);
        if let Some(ui) = weak_ui_file.upgrade() {
            refresh_items_ui(&ui, &state_file.borrow(), &cat_id, &query);
        }
    });

    let state_search = state.clone();
    let weak_ui_search = ui.as_weak();
    ui.on_search_changed(move |text| {
        let text = text.to_string();
        log::debug!("Search changed: {}", text);
        let mut state = state_search.borrow_mut();
        state.search_query = text.clone();
        let cat_id = state.current_category_id.clone();
        drop(state);
        if let Some(ui) = weak_ui_search.upgrade() {
            refresh_items_ui(&ui, &state_search.borrow(), &cat_id, &text);
        }
    });

    let state_cat = state.clone();
    let weak_ui_cat = ui.as_weak();
    ui.on_category_selected(move |index| {
        log::debug!("Category selected: index={}", index);
        let mut state = state_cat.borrow_mut();
        if let Some(cat) = state.manager.categories().get(index as usize) {
            let cat_id = cat.id.clone();
            state.current_category_id = cat_id.clone();
            let query = state.search_query.clone();
            drop(state);
            if let Some(ui) = weak_ui_cat.upgrade() {
                refresh_items_ui(&ui, &state_cat.borrow(), &cat_id, &query);
            }
        }
    });

    let state_view = state.clone();
    ui.on_toggle_view(move || {
        log::debug!("View toggled");
        let is_icon = state_view
            .borrow()
            .manager
            .config()
            .data
            .categories
            .first()
            .map(|c| matches!(c.view_type, ViewType::Icon))
            .unwrap_or(true);
        let _ = is_icon;
    });

    ui.on_toggle_theme(move || {
        log::debug!("Theme toggled");
    });

    let state_new = state.clone();
    let weak_ui_new = ui.as_weak();
    ui.on_new_item_clicked(move || {
        log::debug!("New item clicked");
        let mut state = state_new.borrow_mut();
        let stub_item = Item {
            id: model::generate_id("item"),
            name: "New Item".to_string(),
            target: String::new(),
            category_id: state.current_category_id.clone(),
            ..Default::default()
        };
        if state.manager.add_item(stub_item) {
            let cat_id = state.current_category_id.clone();
            let query = state.search_query.clone();
            drop(state);
            if let Some(ui) = weak_ui_new.upgrade() {
                refresh_items_ui(&ui, &state_new.borrow(), &cat_id, &query);
            }
        }
    });

    let state_refresh = state.clone();
    ui.on_refresh_icons(move || {
        log::debug!("Refresh icons");
        let state = state_refresh.borrow();
        let items = state.manager.items(&state.current_category_id);
        for item in &items {
            if let Some(path) = icon_cache.get_icon_path(&item.id) {
                log::debug!("Icon found for {}: {:?}", item.name, path);
            }
        }
    });

    let state_dbl = state.clone();
    ui.on_item_double_clicked(move |index| {
        log::debug!("Item double-clicked: index={}", index);
        let state = state_dbl.borrow();
        let items = if state.search_query.is_empty() {
            state.manager.items(&state.current_category_id)
        } else {
            state.manager.search_items(&state.search_query)
        };
        if let Some(item) = items.get(index as usize) {
            let item = item.clone();
            drop(state);
            let result = crate::runner::run(&item);
            if result.success {
                log::info!("Launched: {}", item.name);
                let mut state = state_dbl.borrow_mut();
                state.manager.increment_run_count(&item.id);
            } else {
                log::error!("Failed to launch {}: {}", item.name, result.error_message);
            }
        }
    });

    let rci = right_clicked_index.clone();
    let weak_ui_right = ui.as_weak();
    ui.on_item_right_clicked(move |index, _x, _y| {
        log::debug!("Item right-clicked: index={}", index);
        rci.set(Some(index));
        let menu_items = Rc::new(slint::VecModel::from(vec![
            slint::SharedString::from("Edit"),
            slint::SharedString::from("Run as Admin"),
            slint::SharedString::from("Delete"),
            slint::SharedString::from("Copy Path"),
            slint::SharedString::from("Open Location"),
        ]));
        if let Some(ui) = weak_ui_right.upgrade() {
            ui.set_context_menu_items(menu_items.into());
        }
    });

    let state_menu = state.clone();
    let weak_ui_menu = ui.as_weak();
    ui.on_menu_item_selected(move |index| {
        log::debug!("Menu item selected: {}", index);
        if let Some(item_idx) = right_clicked_index.get() {
            let mut state = state_menu.borrow_mut();
            let items = if state.search_query.is_empty() {
                state.manager.items(&state.current_category_id)
            } else {
                state.manager.search_items(&state.search_query)
            };
            if let Some(item) = items.get(item_idx as usize) {
                let item = item.clone();
                match index {
                    0 => {
                        log::info!("Edit item: {} ({})", item.name, item.id);
                        let mut state = state_menu.borrow_mut();
                        let categories: Vec<slint::SharedString> = state
                            .manager
                            .categories()
                            .iter()
                            .map(|c| c.name.clone().into())
                            .collect();
                        let selected_cat_idx = state
                            .manager
                            .categories()
                            .iter()
                            .position(|c| c.id == item.category_id)
                            .unwrap_or(0);
                        drop(state);

                        if let Some(ui) = weak_ui_menu.upgrade() {
                            let dialog = EditDialog::new().unwrap();
                            dialog.set_item_name(item.name.clone().into());
                            dialog.set_item_target(item.target.clone().into());
                            dialog.set_item_arguments(item.arguments.clone().into());
                            dialog.set_item_working_dir(item.working_dir.clone().into());
                            dialog.set_item_icon_path(item.icon_path.clone().into());
                            dialog.set_item_icon_index(item.icon_index);
                            dialog.set_item_keywords(item.keywords.clone().into());
                            dialog.set_item_remark(item.remark.clone().into());
                            dialog.set_run_as_admin(item.run_as_admin);
                            dialog.set_category_names(
                                Rc::new(slint::VecModel::from(categories)).into(),
                            );
                            dialog.set_selected_category_index(selected_cat_idx as i32);

                            let dialog_weak = dialog.as_weak();
                            let state_edit = state_menu.clone();
                            let weak_ui_edit = weak_ui_menu.clone();
                            dialog.on_save(move || {
                                let dialog = dialog_weak.upgrade().unwrap();
                                let mut state = state_edit.borrow_mut();
                                let mut updated_item = item.clone();
                                updated_item.name = dialog.get_item_name().to_string();
                                updated_item.target = dialog.get_item_target().to_string();
                                updated_item.arguments = dialog.get_item_arguments().to_string();
                                updated_item.working_dir =
                                    dialog.get_item_working_dir().to_string();
                                updated_item.icon_path = dialog.get_item_icon_path().to_string();
                                updated_item.icon_index = dialog.get_item_icon_index();
                                updated_item.keywords = dialog.get_item_keywords().to_string();
                                updated_item.remark = dialog.get_item_remark().to_string();
                                updated_item.run_as_admin = dialog.get_run_as_admin();
                                let cat_idx = dialog.get_selected_category_index() as usize;
                                if let Some(cat) = state.manager.categories().get(cat_idx) {
                                    updated_item.category_id = cat.id.clone();
                                }

                                if state.manager.update_item(&updated_item) {
                                    let cat_id = state.current_category_id.clone();
                                    let query = state.search_query.clone();
                                    drop(state);
                                    if let Some(ui) = weak_ui_edit.upgrade() {
                                        refresh_items_ui(
                                            &ui,
                                            &state_edit.borrow(),
                                            &cat_id,
                                            &query,
                                        );
                                    }
                                }
                            });

                            dialog.show().unwrap();
                        }
                    }
                    1 => {
                        log::info!("Run as admin: {}", item.name);
                        drop(state);
                        let result = crate::runner::run_as_admin(&item);
                        if result.success {
                            let mut state = state_menu.borrow_mut();
                            state.manager.increment_run_count(&item.id);
                        } else {
                            log::error!(
                                "Failed to launch as admin {}: {}",
                                item.name,
                                result.error_message
                            );
                        }
                        return;
                    }
                    2 => {
                        log::info!("Delete item: {} ({})", item.name, item.id);
                        if state.manager.delete_item(&item.id) {
                            let cat_id = state.current_category_id.clone();
                            let query = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &query);
                            }
                            return;
                        }
                    }
                    3 => {
                        copy_to_clipboard(&item.target);
                    }
                    4 => {
                        open_file_location(&item.target);
                    }
                    _ => {}
                }
            }
        }
    });

    ui.on_background_clicked(move || {
        log::debug!("Background clicked");
    });
}

fn refresh_items_ui(ui: &MainWindow, state: &AppState, category_id: &str, query: &str) {
    let items = if query.is_empty() {
        state.manager.items(category_id)
    } else {
        state.manager.search_items(query)
    };

    let ui_items: Vec<ItemModel> = items
        .iter()
        .map(|item| ItemModel {
            id: item.id.clone().into(),
            name: item.name.clone().into(),
            target: item.target.clone().into(),
            icon_path: item.icon_path.clone().into(),
        })
        .collect();

    let model = Rc::new(slint::VecModel::from(ui_items));
    ui.set_items(model.into());

    let status = format!("{} items", items.len());
    ui.set_status_text(status.into());
}

fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Foundation::{HANDLE, HWND};
        use windows::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
        };
        use windows::Win32::System::Ole::CF_UNICODETEXT;

        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let size = wide.len() * std::mem::size_of::<u16>();

        unsafe {
            if OpenClipboard(HWND::default()).is_err() {
                log::error!("Failed to open clipboard");
                return false;
            }
            if EmptyClipboard().is_err() {
                log::error!("Failed to empty clipboard");
                let _ = CloseClipboard();
                return false;
            }

            let h_mem = GlobalAlloc(GMEM_MOVEABLE, size).unwrap_or_default();
            if h_mem.is_invalid() {
                log::error!("Failed to allocate global memory for clipboard");
                let _ = CloseClipboard();
                return false;
            }

            let ptr = GlobalLock(h_mem) as *mut u16;
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            let _ = GlobalUnlock(h_mem);

            if SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(h_mem.0 as isize)).is_err() {
                log::error!("Failed to set clipboard data");
                let _ = CloseClipboard();
                return false;
            }

            let _ = CloseClipboard();
        }

        log::info!("Copied to clipboard: {}", text);
        return true;
    }

    #[cfg(not(windows))]
    log::warn!("Clipboard operation not supported on non-Windows platform");
    return false;
}

#[cfg(windows)]
fn open_file_location(path: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let explorer = OsStr::new("explorer")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let params = OsStr::new(&format!("/select,{}", path))
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe {
        let _ = ShellExecuteW(
            HWND::default(),
            windows::core::w!("open"),
            windows::core::w!("explorer"),
            windows::core::PCWSTR(params.as_ptr()),
            windows::core::PCWSTR(std::ptr::null()),
            SW_SHOW,
        );
    }
    log::info!("Opened file location: {}", path);
}

#[cfg(not(windows))]
fn open_file_location(path: &str) {
    log::warn!(
        "Open file location not supported on non-Windows platform: {}",
        path
    );
}

#[cfg(windows)]
fn setup_drop_handler(ui: &MainWindow) {
    use raw_window_handle::HasWindowHandle;

    let window_handle = ui.window().window_handle();
    let raw_handle = HasWindowHandle::window_handle(&window_handle).unwrap();
    let hwnd = match raw_handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(win32) => win32.hwnd.get() as isize,
        _ => {
            log::warn!("Not running on Windows, drag-and-drop disabled");
            return;
        }
    };

    let handler = platform::setup_dragdrop(hwnd);
    let weak_ui = ui.as_weak();

    std::thread::spawn(move || {
        while let Ok(msg) = handler.rx.recv() {
            match msg {
                platform::DropMessage::Enter => {
                    let weak = weak_ui.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_drop_active(true);
                        }
                    })
                    .ok();
                }
                platform::DropMessage::Leave => {
                    let weak = weak_ui.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_drop_active(false);
                        }
                    })
                    .ok();
                }
                platform::DropMessage::Drop(paths) => {
                    let weak = weak_ui.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_drop_active(false);
                            if let Some(path) = paths.first() {
                                let target = path.to_string_lossy().to_string();
                                let name = path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| target.clone());
                                ui.invoke_file_dropped(
                                    slint::SharedString::from(name),
                                    slint::SharedString::from(target),
                                );
                            }
                        }
                    })
                    .ok();
                }
                _ => {}
            }
        }
    });
}

#[cfg(not(windows))]
fn setup_drop_handler(_ui: &MainWindow) {
    log::warn!("Drag-and-drop is only supported on Windows");
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
