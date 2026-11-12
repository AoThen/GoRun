//! 拖放入口：安装子类化处理器，把落下的文件转成 `file-dropped` UI 回调。

#[cfg(windows)]
use crate::MainWindow;
#[cfg(windows)]
use slint::ComponentHandle;

/// 安装拖放处理器。
///
/// 返回的 `(guard, rx)`：
/// - `guard` **必须随应用存活**（存入 `AppState.drop_handler`），
///   它持有子类化与 refdata，Drop 时按序清理；
/// - `rx` 交给消费线程，线程退出（channel 断开）即意味着 guard 已释放。
#[cfg(windows)]
pub fn setup_drop_handler(
    ui: &MainWindow,
) -> Option<(
    crate::platform::DropHandler,
    std::sync::mpsc::Receiver<crate::platform::DropMessage>,
)> {
    use raw_window_handle::HasWindowHandle;

    let window_handle = ui.window().window_handle();
    let raw_handle = match HasWindowHandle::window_handle(&window_handle) {
        Ok(handle) => handle,
        Err(e) => {
            log::warn!("Failed to get window handle: {e:?}, drag-and-drop disabled");
            return None;
        }
    };

    let hwnd = match raw_handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(win32) => win32.hwnd.get() as isize,
        _ => {
            log::warn!("Raw window handle is not Win32; drag-and-drop disabled");
            return None;
        }
    };

    Some(crate::platform::setup_dragdrop(hwnd))
}

#[cfg(not(windows))]
pub fn setup_drop_handler(
    _ui: &crate::MainWindow,
) -> Option<(
    crate::platform::DropHandler,
    std::sync::mpsc::Receiver<crate::platform::DropMessage>,
)> {
    log::warn!("Drag-and-drop is only supported on Windows");
    None
}

/// 启动消费线程：把落下的路径转成 UI 的 `file-dropped` 回调。
#[cfg(windows)]
pub fn spawn_drop_consumer(
    ui_weak: slint::Weak<MainWindow>,
    rx: std::sync::mpsc::Receiver<crate::platform::DropMessage>,
) {
    std::thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            match msg {
                crate::platform::DropMessage::Drop(paths) => {
                    let weak = ui_weak.clone();
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
        log::debug!("Drop consumer thread exited");
    });
}

#[cfg(not(windows))]
pub fn spawn_drop_consumer(
    _ui_weak: slint::Weak<crate::MainWindow>,
    _rx: std::sync::mpsc::Receiver<crate::platform::DropMessage>,
) {
}
