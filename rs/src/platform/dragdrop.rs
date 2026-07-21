#[cfg(windows)]
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use super::DropMessage;

#[cfg(windows)]
pub fn setup_dragdrop(hwnd: isize, sender: &Sender<DropMessage>) {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, LRESULT};
    use windows::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::WM_DROPFILES;

    unsafe extern "system" fn subclass_proc(
        _hwnd: HWND,
        umsg: u32,
        wparam: WPARAM,
        _lparam: LPARAM,
        _uidsubclass: usize,
        dwrefdata: usize,
    ) -> LRESULT {
        if umsg == WM_DROPFILES {
            let hdrop = HDROP(wparam.0 as isize);
            let sender = &*(dwrefdata as *const Sender<DropMessage>);

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
            LRESULT(0)
        }
    }

    unsafe {
        let hwnd = HWND(hwnd);
        DragAcceptFiles(hwnd, true);
        let sender_ptr = sender as *const Sender<DropMessage> as usize;
        SetWindowSubclass(hwnd, Some(subclass_proc), 0, sender_ptr);
    }
}

#[cfg(not(windows))]
pub fn setup_dragdrop(_hwnd: isize, _sender: &Sender<DropMessage>) {
    // Drag-and-drop is only supported on Windows
}
