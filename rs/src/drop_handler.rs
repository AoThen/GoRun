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

/// 把落下的路径转发到 UI 的 `file-dropped` 回调（Drop 消费共用，L-12）。
fn forward_drop_paths(
    ui_weak: &slint::Weak<crate::MainWindow>,
    paths: &[std::path::PathBuf],
) {
    let weak = ui_weak.clone();
    let paths = paths.to_vec();
    let _ = slint::invoke_from_event_loop(move || {
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
    });
}

/// 延迟隐藏窗口决策（纯 channel 状态机，供 MouseLeft 延迟窗口与测试复用）：
/// 在 deadline 前消费消息——MouseEnter 取消隐藏；窗口内到达的 Drop 照常
/// 收集不丢（L-12：原实现 `Ok(_) => continue` 会丢弃延迟窗口内的 Drop，
/// 依赖"OLE DragEnter 必先于 Drop"的隐式时序）；MouseLeft 无效果。
/// 返回 (是否仍需隐藏, 窗口内收到的待转发路径)。
#[cfg_attr(not(windows), allow(dead_code))]
fn drain_hide_window(
    rx: &std::sync::mpsc::Receiver<crate::platform::DropMessage>,
    deadline: std::time::Instant,
) -> (bool, Vec<std::path::PathBuf>) {
    let mut hide = true;
    let mut dropped: Vec<std::path::PathBuf> = Vec::new();
    while hide {
        let now = std::time::Instant::now();
        if now >= deadline {
            break;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(crate::platform::DropMessage::MouseEnter) => hide = false,
            Ok(crate::platform::DropMessage::Drop(paths)) => dropped.extend(paths),
            Ok(crate::platform::DropMessage::MouseLeft) => {}
            Err(_) => break,
        }
    }
    (hide, dropped)
}

/// 启动消费线程：把落下的路径转成 UI 的 `file-dropped` 回调；
/// 光标离开（F7 自动隐藏）延迟 1s 后隐藏，期间回到窗口则取消。
#[cfg(windows)]
pub fn spawn_drop_consumer(
    ui_weak: slint::Weak<MainWindow>,
    rx: std::sync::mpsc::Receiver<crate::platform::DropMessage>,
    auto_hide: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    std::thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            match msg {
                crate::platform::DropMessage::Drop(paths) => {
                    forward_drop_paths(&ui_weak, &paths);
                }
                crate::platform::DropMessage::MouseLeft => {
                    // P3-11：按事件时刻读共享标志（设置页开关可运行时切换，
                    // 即时生效；此前为启动时 bool 快照，切换需重启才生效）
                    if !auto_hide.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    // 延迟 1s 窗口：期间回到窗口（MouseEnter）则取消隐藏；
                    // 窗口内到达的 Drop 不丢（L-12），统一在窗口结束后转发
                    let deadline =
                        std::time::Instant::now() + std::time::Duration::from_secs(1);
                    let (hide, dropped) = drain_hide_window(&rx, deadline);
                    if !dropped.is_empty() {
                        forward_drop_paths(&ui_weak, &dropped);
                    }
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
    _auto_hide: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::DropMessage;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    #[test]
    fn test_drain_hide_window_mouse_enter_cancels_hide() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(DropMessage::MouseEnter).unwrap();
        let (hide, dropped) = drain_hide_window(&rx, Instant::now() + Duration::from_millis(50));
        assert!(!hide, "窗口内 MouseEnter 应取消隐藏");
        assert!(dropped.is_empty());
    }

    #[test]
    fn test_drain_hide_window_drop_not_lost() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(DropMessage::Drop(vec![PathBuf::from("C:\\a.exe")]))
            .unwrap();
        // 无 MouseEnter：超时后仍隐藏，但窗口内 Drop 必须带出（不丢消息）
        let (hide, dropped) = drain_hide_window(&rx, Instant::now() + Duration::from_millis(50));
        assert!(hide);
        assert_eq!(dropped, vec![PathBuf::from("C:\\a.exe")]);
    }

    #[test]
    fn test_drain_hide_window_timeout_hides() {
        let (_tx, rx) = std::sync::mpsc::channel::<DropMessage>();
        let (hide, dropped) = drain_hide_window(&rx, Instant::now() + Duration::from_millis(30));
        assert!(hide, "超时无 MouseEnter 应保持隐藏");
        assert!(dropped.is_empty());
    }
}
