//! OLE 拖放入口（P1-3 自 main.rs 拆出）

use crate::MainWindow;
use slint::ComponentHandle;

#[cfg(windows)]
pub fn setup_drop_handler(ui: &MainWindow) {
    use raw_window_handle::HasWindowHandle;

    let window_handle = ui.window().window_handle();
    let raw_handle = match HasWindowHandle::window_handle(&window_handle) {
        Ok(handle) => handle,
        Err(e) => {
            log::warn!("Failed to get window handle: {e:?}, drag-and-drop disabled");
            return;
        }
    };

    let hwnd = match raw_handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(win32) => win32.hwnd.get() as isize,
        _ => {
            log::warn!("Not running on Windows, drag-and-drop disabled");
            return;
        }
    };

    let handler = crate::platform::setup_dragdrop(hwnd);
    let weak_ui = ui.as_weak();

    std::thread::spawn(move || {
        while let Ok(msg) = handler.rx.recv() {
            match msg {
                crate::platform::DropMessage::Drop(paths) => {
                    let weak = weak_ui.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            for path in &paths {
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
            }
        }
    });
}

#[cfg(not(windows))]
pub fn setup_drop_handler(_ui: &MainWindow) {
    log::warn!("Drag-and-drop is only supported on Windows");
}
