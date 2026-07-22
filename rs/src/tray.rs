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
    pub fn create(_hwnd: isize, _tooltip: &str) -> Self {
        Self {
            hwnd: HWND(0),
            id: TRAY_ICON_ID,
            created: false,
        }
    }

    pub fn destroy(&mut self) {
        self.created = false;
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

    pub fn show_menu(&self) {}
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
