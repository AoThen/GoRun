#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::mpsc::Sender;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use super::DropMessage;

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, RemoveWindowSubclass,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TRACKMOUSEEVENT, TME_LEAVE,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{WM_DROPFILES, WM_MOUSEMOVE};
#[cfg(windows)]
use windows::Win32::UI::Controls::WM_MOUSELEAVE;

/// 鼠标离开状态（F7 自动隐藏）：置位表示已发过 MouseLeft（光标在窗口外），
/// 下次 WM_MOUSEMOVE（回到窗口）时发 MouseEnter 取消隐藏。单窗口单实例。
#[cfg(windows)]
static LEFT_SENT: AtomicBool = AtomicBool::new(false);

/// 子类化回调：处理 WM_DROPFILES 并把拖放文件路径转发给接收端。
/// 模块级定义（非嵌套），供 setup/remove 成对引用（RemoveWindowSubclass 需同函数指针）。
#[cfg(windows)]
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    if umsg == WM_DROPFILES {
        let hdrop = HDROP(wparam.0 as isize);
        // P2-6：refdata 为 Arc::into_raw 泄漏的稳定内值指针（RAII 生命周期见 DropHandler）
        let sender = &*(_dwrefdata as *const Sender<DropMessage>);

        let file_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        let mut paths = Vec::new();

        for i in 0..file_count {
            let len = DragQueryFileW(hdrop, i, None);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                let copied = DragQueryFileW(hdrop, i, Some(&mut buf));
                if copied > 0 {
                    let path = String::from_utf16_lossy(&buf[..copied as usize]);
                    paths.push(PathBuf::from(path));
                }
            }
        }

        sender.send(DropMessage::Drop(paths)).ok();
        DragFinish(hdrop);
        LRESULT(0)
    } else if umsg == WM_MOUSEMOVE {
        // F7 自动隐藏：光标在窗口内移动时注册 leave 跟踪（每次 move 重置）
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        let _ = TrackMouseEvent(&mut tme);
        // 之前发过 MouseLeft（光标在窗口外）且现在又 move → 回到窗口，通知取消隐藏
        if LEFT_SENT.swap(false, Ordering::SeqCst) {
            let sender = &*(_dwrefdata as *const Sender<DropMessage>);
            sender.send(DropMessage::MouseEnter).ok();
        }
        DefSubclassProc(hwnd, umsg, wparam, lparam)
    } else if umsg == WM_MOUSELEAVE {
        // F7 自动隐藏：光标离开窗口 → 置位并通知（消费线程延迟后隐藏）
        LEFT_SENT.store(true, Ordering::SeqCst);
        let sender = &*(_dwrefdata as *const Sender<DropMessage>);
        sender.send(DropMessage::MouseLeft).ok();
        LRESULT(0)
    } else {
        DefSubclassProc(hwnd, umsg, wparam, lparam)
    }
}

#[cfg(windows)]
pub fn setup_dragdrop(hwnd: isize, sender_ptr: usize) {
    use windows::Win32::UI::Shell::SetWindowSubclass;

    unsafe {
        let hwnd = HWND(hwnd);
        // 已装过则先摘除，避免重复子类化叠加（幂等）。
        let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), 0);
        DragAcceptFiles(hwnd, true);
        let ok = SetWindowSubclass(hwnd, Some(subclass_proc), 0, sender_ptr);
        if ok.as_bool() {
            log::debug!("Drag-drop subclass installed on hwnd={}", hwnd.0);
        } else {
            log::warn!("SetWindowSubclass failed for hwnd={}", hwnd.0);
        }
    }
}

/// 移除拖放子类化（P2-6：RAII 生命周期配对，uidsubclass=0 须与 setup_dragdrop 一致）。
#[cfg(windows)]
pub fn remove_dragdrop(hwnd: isize) {
    use windows::Win32::UI::Shell::RemoveWindowSubclass;

    unsafe {
        let result = RemoveWindowSubclass(HWND(hwnd), Some(subclass_proc), 0);
        if !result.as_bool() {
            log::debug!("RemoveWindowSubclass failed for hwnd={}", hwnd);
        }
    }
}

#[cfg(not(windows))]
pub fn setup_dragdrop(_hwnd: isize, _sender_ptr: usize) {}

#[cfg(not(windows))]
pub fn remove_dragdrop(_hwnd: isize) {}
