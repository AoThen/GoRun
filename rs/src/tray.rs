#![allow(dead_code)]

#[cfg(windows)]
use std::sync::OnceLock;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::sync::mpsc::{channel, Receiver, Sender};

#[cfg(windows)]
use log::{info, warn};

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, HINSTANCE, POINT};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyMenu, LoadCursorW, LoadIconW, PostMessageW,
    RegisterClassExW, AppendMenuW, CreatePopupMenu, GetCursorPos, SetForegroundWindow,
    TrackPopupMenu, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HMENU, HCURSOR, IDC_ARROW,
    IDI_APPLICATION, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_CREATE, WM_DESTROY,
    WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSEXW,
};

#[cfg(windows)]
const WM_TRAYICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[cfg(not(windows))]
const WM_TRAYICON: u32 = 0x8001;

const TRAY_ICON_ID: u32 = 1;
const MENU_ID_SHOW: u32 = 1001;
const MENU_ID_EXIT: u32 = 1002;

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Show,
    Hide,
    Quit,
}

#[cfg(windows)]
pub struct TrayIcon {
    hwnd: HWND,
    sender: Sender<TrayMessage>,
}

#[cfg(not(windows))]
pub struct TrayIcon {
    _private: (),
}

#[cfg(windows)]
static TRAY_SENDER: OnceLock<Sender<TrayMessage>> = OnceLock::new();

#[cfg(windows)]
impl TrayIcon {
    pub fn create(_hwnd: isize, tooltip: &str) -> (Self, Receiver<TrayMessage>) {
        let (tx, rx) = channel();

        unsafe {
            let _ = TRAY_SENDER.set(tx.clone());

            let h_instance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap_or_default();

            let class_name = windows::core::w!("GoRunTrayWindow");

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(tray_wnd_proc),
                hInstance: HINSTANCE(h_instance.0),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: class_name,
                ..Default::default()
            };

            let _ = RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                Default::default(),
                class_name,
                windows::core::w!(""),
                Default::default(),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                h_instance,
                None,
            );

            if hwnd.0 == 0 {
                warn!("TrayIcon: failed to create message window");
                return (
                    TrayIcon {
                        hwnd,
                        sender: tx,
                    },
                    rx,
                );
            }

            let h_icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();

            let mut tip = [0u16; 128];
            let tooltip_wide: Vec<u16> = tooltip.encode_utf16().collect();
            let len = tooltip_wide.len().min(127);
            tip[..len].copy_from_slice(&tooltip_wide[..len]);

            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: TRAY_ICON_ID,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: h_icon,
                szTip: tip,
                ..Default::default()
            };

            if Shell_NotifyIconW(NIM_ADD, &mut nid).as_bool() {
                info!("TrayIcon: tray icon created successfully");
            } else {
                warn!("TrayIcon: Shell_NotifyIconW(NIM_ADD) failed");
            }

            let tray = TrayIcon {
                hwnd,
                sender: tx,
            };

            (tray, rx)
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: TRAY_ICON_ID,
                ..Default::default()
            };
            if Shell_NotifyIconW(NIM_DELETE, &nid).as_bool() {
                info!("TrayIcon: tray icon removed");
            }
        }
    }

    pub fn show_menu(&self) {
        unsafe {
            let h_menu = match CreatePopupMenu() {
                Ok(menu) => menu,
                Err(_) => {
                    warn!("TrayIcon: CreatePopupMenu failed");
                    return;
                }
            };
            if h_menu.is_invalid() {
                warn!("TrayIcon: CreatePopupMenu returned invalid handle");
                return;
            }

            let show_text: Vec<u16> = "Show GoRun\0".encode_utf16().collect();
            let exit_text: Vec<u16> = "Exit\0".encode_utf16().collect();

            AppendMenuW(h_menu, Default::default(), MENU_ID_SHOW as usize, windows::core::PCWSTR(show_text.as_ptr()));
            AppendMenuW(h_menu, Default::default(), MENU_ID_EXIT as usize, windows::core::PCWSTR(exit_text.as_ptr()));

            let mut pos = POINT::default();
            let _ = GetCursorPos(&mut pos);
            let _ = SetForegroundWindow(self.hwnd);

            let cmd = TrackPopupMenu(
                h_menu,
                TPM_NONOTIFY | TPM_RETURNCMD | TPM_RIGHTBUTTON,
                pos.x,
                pos.y,
                0,
                self.hwnd,
                None,
            );

            match cmd.0 {
                0 => {}
                id if id == MENU_ID_SHOW as i32 => {
                    let _ = self.sender.send(TrayMessage::Show);
                }
                id if id == MENU_ID_EXIT as i32 => {
                    let _ = self.sender.send(TrayMessage::Quit);
                }
                _ => {}
            }

            let _ = DestroyMenu(h_menu);
            let _ = PostMessageW(self.hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
        }
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
}

#[cfg(windows)]
impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(not(windows))]
impl TrayIcon {
    pub fn create(_hwnd: isize, _tooltip: &str) -> (Self, std::sync::mpsc::Receiver<TrayMessage>) {
        let (_, rx) = std::sync::mpsc::channel();
        (Self { _private: () }, rx)
    }

    pub fn destroy(&mut self) {}

    pub fn handle_message(&self, _lparam: u32) -> Option<TrayMessage> {
        None
    }

    pub fn show_menu(&self) {}
}

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            if let Some(sender) = TRAY_SENDER.get() {
                match lparam.0 as u32 {
                    WM_LBUTTONDBLCLK => {
                        let _ = sender.send(TrayMessage::Show);
                    }
                    WM_RBUTTONUP => {
                        let _ = sender.send(TrayMessage::Show);
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostMessageW(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
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
