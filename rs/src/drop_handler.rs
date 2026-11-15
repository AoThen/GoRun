//! 拖放入口：安装子类化处理器，把落下的文件转成 `file-dropped` UI 回调。

#[cfg(windows)]
use crate::MainWindow;
#[cfg(windows)]
use slint::ComponentHandle;

/// 尝试获取主窗口的 Win32 句柄；winit 窗口尚未创建时返回 None（静默，供重试探测复用）。
#[cfg(windows)]
pub fn try_get_hwnd(ui: &MainWindow) -> Option<isize> {
    use raw_window_handle::HasWindowHandle;

    let window_handle = ui.window().window_handle();
    let raw_handle = HasWindowHandle::window_handle(&window_handle).ok()?;

    match raw_handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(win32) => Some(win32.hwnd.get() as isize),
        _ => None,
    }
}

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
    let hwnd = match try_get_hwnd(ui) {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("Window handle not ready; drag-and-drop disabled");
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

/// 非 Windows 平台无 Win32 句柄概念，恒为 None（仅为 when_window_ready 提供统一判据）。
#[cfg(not(windows))]
pub fn try_get_hwnd(_ui: &crate::MainWindow) -> Option<isize> {
    None
}

/// 启动消费线程：把落下的路径转成 UI 的 `file-dropped` 回调；
/// 光标离开（F7 自动隐藏）延迟 1s 后隐藏，期间回到窗口则取消。
#[cfg(windows)]
pub fn spawn_drop_consumer(
    ui_weak: slint::Weak<MainWindow>,
    rx: std::sync::mpsc::Receiver<crate::platform::DropMessage>,
    auto_hide: bool,
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
                crate::platform::DropMessage::MouseLeft => {
                    if !auto_hide {
                        continue;
                    }
                    // 延迟 1s：期间收到 MouseEnter（回到窗口）则取消隐藏
                    let hide = match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                        Ok(crate::platform::DropMessage::MouseEnter) => false,
                        Ok(_) => continue,
                        Err(_) => true,
                    };
                    if hide {
                        let weak = ui_weak.clone();
                        slint::invoke_from_event_loop(move || {
                            if let Some(ui) = weak.upgrade() {
                                if ui.window().is_visible() {
                                    ui.hide();
                                }
                            }
                        })
                        .ok();
                    }
                }
                crate::platform::DropMessage::MouseEnter => {}
            }
        }
        log::debug!("Drop consumer thread exited");
    });
}

#[cfg(not(windows))]
pub fn spawn_drop_consumer(
    _ui_weak: slint::Weak<crate::MainWindow>,
    _rx: std::sync::mpsc::Receiver<crate::platform::DropMessage>,
    _auto_hide: bool,
) {
}
