#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::mpsc::Sender;
#[cfg(windows)]
use super::DropMessage;

#[cfg(windows)]
pub fn setup_dragdrop(hwnd: isize, sender_ptr: usize) {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{
        DragAcceptFiles, DragFinish, DragQueryFileW, SetWindowSubclass, HDROP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WM_DROPFILES};

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
        } else {
            DefWindowProcW(hwnd, umsg, wparam, lparam)
        }
    }

    unsafe {
        let hwnd = HWND(hwnd);
        DragAcceptFiles(hwnd, true);
        SetWindowSubclass(hwnd, Some(subclass_proc), 0, sender_ptr);
    }
}

#[cfg(not(windows))]
pub fn setup_dragdrop(_hwnd: isize, _sender_ptr: usize) {}
