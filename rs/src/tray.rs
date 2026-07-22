#![allow(dead_code)]

#[cfg(windows)]
use log::{info, warn};

#[cfg(windows)]
use std::mem::size_of;

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, PostMessageW,
    SetForegroundWindow, TrackPopupMenu, IDC_ICON, IDI_APPLICATION, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP,
};

#[cfg(windows)]
const WM_TRAYICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[cfg(not(windows))]
const WM_TRAYICON: u32 = 0x8001;

const TRAY_ID_SHOW: u32 = 1;
const TRAY_ID_EXIT: u32 = 2;
const TRAY_ICON_ID: u32 = 1;

pub enum TrayMessage {
    Show,
    Hide,
    Quit,
}

#[cfg(windows)]
pub struct TrayIcon {
    hwnd: HWND,
    id: u32,
    created: bool,
}

#[cfg(not(windows))]
pub struct TrayIcon {
    _private: (),
}

#[cfg(windows)]
impl TrayIcon {
    pub fn create(hwnd: isize, tooltip: &str) -> Self {
        let hwnd = HWND(hwnd as *mut std::ffi::c_void);

        let h_icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default();

        let mut sz_tip = [0u16; 128];
        let truncated: Vec<u16> = tooltip.encode_utf16().take(127).collect();
        for (i, &ch) in truncated.iter().enumerate() {
            sz_tip[i] = ch;
        }

        let mut nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: h_icon,
            szTip: sz_tip,
            ..Default::default()
        };

        let created = unsafe { Shell_NotifyIconW(NIM_ADD, &mut nid) }.as_bool();

        if created {
            info!("Tray icon created successfully");
        } else {
            warn!("Failed to create tray icon");
        }

        Self {
            hwnd,
            id: TRAY_ICON_ID,
            created,
        }
    }

    pub fn destroy(&mut self) {
        if !self.created {
            return;
        }

        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id,
            ..Default::default()
        };

        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }

        self.created = false;
        info!("Tray icon destroyed");
    }

    pub fn handle_message(&self, lparam: u32) -> Option<TrayMessage> {
        match lparam {
            WM_LBUTTONDBLCLK => Some(TrayMessage::Show),
            WM_RBUTTONUP => {
                self.show_menu();
                None
            }
            _ => None,
        }
    }

    pub fn show_menu(&self) {
        let mut cursor_pos = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut cursor_pos);
        }

        let h_menu = unsafe { CreatePopupMenu() };
        if h_menu.is_err() {
            warn!("Failed to create popup menu");
            return;
        }
        let h_menu = h_menu.unwrap();

        unsafe {
            let _ = AppendMenuW(h_menu, TPM_NONOTIFY, TRAY_ID_SHOW as usize, "Show/Hide");
            let _ = AppendMenuW(h_menu, TPM_NONOTIFY, 0, None);
            let _ = AppendMenuW(h_menu, TPM_NONOTIFY, TRAY_ID_EXIT as usize, "Exit");

            let _ = SetForegroundWindow(self.hwnd);

            let cmd = TrackPopupMenu(
                h_menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                cursor_pos.x,
                cursor_pos.y,
                0,
                self.hwnd,
                None,
            );

            let _ = DestroyMenu(h_menu);

            match cmd.0 as u32 {
                TRAY_ID_SHOW => {
                    let _ = PostMessageW(
                        self.hwnd,
                        WM_TRAYICON,
                        None,
                        windows::Win32::Foundation::LPARAM(WM_LBUTTONDBLCLK as isize),
                    );
                }
                TRAY_ID_EXIT => {
                    let _ = PostMessageW(
                        self.hwnd,
                        WM_DESTROY,
                        None,
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(not(windows))]
impl TrayIcon {
    pub fn create(_hwnd: isize, _tooltip: &str) -> Self {
        Self { _private: () }
    }

    pub fn destroy(&mut self) {}

    pub fn handle_message(&self, _lparam: u32) -> Option<TrayMessage> {
        None
    }

    pub fn show_menu(&self) {}
}

#[cfg(windows)]
impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_message_enum() {
        let _show = TrayMessage::Show;
        let _hide = TrayMessage::Hide;
        let _quit = TrayMessage::Quit;
    }
}
