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
    CreateWindowExW, DefWindowProcW, DestroyMenu, LoadCursorW, LoadIconW,
    RegisterClassExW, RegisterWindowMessageW, AppendMenuW, CreatePopupMenu, GetCursorPos,
    SetForegroundWindow, TrackPopupMenu, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    HICON, IDC_ARROW, IDI_APPLICATION, MF_CHECKED, MF_UNCHECKED, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSEXW,
};

#[cfg(windows)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE,
    REG_SZ,
};

#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Mutex;

#[cfg(windows)]
const WM_TRAYICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[cfg(not(windows))]
const WM_TRAYICON: u32 = 0x8001;

const TRAY_ICON_ID: u32 = 1;
const MENU_ID_SHOW: u32 = 1001;
const MENU_ID_EXIT: u32 = 1002;
const MENU_ID_AUTOSTART: u32 = 1003;
const MENU_ID_FOLLOW_MOUSE: u32 = 1004;
const MENU_ID_LANG_ZHCN: u32 = 1005;
const MENU_ID_LANG_ZHTW: u32 = 1006;
const MENU_ID_LANG_ENUS: u32 = 1007;

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Show,
    Hide,
    Toggle,
    Quit,
    ToggleAutoStart,
    ToggleFollowMouse,
    SetLanguage(String),
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

/// 托盘菜单本地化文案（主线程随语言切换写入，弹出菜单时读取）。
#[derive(Debug, Clone)]
pub struct TrayTexts {
    pub show: String,
    pub autostart: String,
    pub follow_mouse: String,
    pub exit: String,
}

/// 托盘全局状态（P2-5：聚合 8 个 OnceLock 为单一结构体，统一经 tray() 访问，
/// 减少散落的隐式全局耦合；wndproc 回调仍经静态读取，不改动窗口机制）。
#[cfg(windows)]
struct TrayGlobals {
    sender: OnceLock<Sender<TrayMessage>>,
    hwnd: OnceLock<HWND>,
    icon_handle: OnceLock<HICON>,
    tip: OnceLock<[u16; 128]>,
    taskbar_created_msg: OnceLock<u32>,
    follow_mouse: OnceLock<AtomicBool>,
    lang: OnceLock<Mutex<String>>,
    texts: OnceLock<Mutex<Option<TrayTexts>>>,
}

#[cfg(windows)]
impl TrayGlobals {
    const fn new() -> Self {
        Self {
            sender: OnceLock::new(),
            hwnd: OnceLock::new(),
            icon_handle: OnceLock::new(),
            tip: OnceLock::new(),
            taskbar_created_msg: OnceLock::new(),
            follow_mouse: OnceLock::new(),
            lang: OnceLock::new(),
            texts: OnceLock::new(),
        }
    }
}

#[cfg(windows)]
static TRAY: OnceLock<TrayGlobals> = OnceLock::new();

/// 统一访问托盘全局状态（首次调用初始化）。
#[cfg(windows)]
fn tray() -> &'static TrayGlobals {
    TRAY.get_or_init(TrayGlobals::new)
}

#[cfg(windows)]
pub fn set_ui_texts(texts: TrayTexts) {
    if let Some(m) = tray().texts.get() {
        if let Ok(mut g) = m.lock() {
            *g = Some(texts);
        }
    } else {
        let _ = tray().texts.set(Mutex::new(Some(texts)));
    }
}

#[cfg(not(windows))]
pub fn set_ui_texts(_texts: TrayTexts) {}

/// 托盘菜单勾选状态同步（主线程写入，托盘弹出菜单读取）。
#[cfg(windows)]
pub fn set_ui_state(follow_mouse: bool, lang: &str) {
    if let Some(a) = tray().follow_mouse.get() {
        a.store(follow_mouse, Ordering::Relaxed);
    } else {
        let _ = tray().follow_mouse.set(AtomicBool::new(follow_mouse));
    }
    if let Some(m) = tray().lang.get() {
        if let Ok(mut g) = m.lock() {
            *g = lang.to_string();
        }
    } else {
        let _ = tray().lang.set(Mutex::new(lang.to_string()));
    }
}

#[cfg(not(windows))]
pub fn set_ui_state(_follow_mouse: bool, _lang: &str) {}

/// 读取"跟随鼠标"状态（主线程显示窗口时用于定位，无需访问 AppState）。
#[cfg(windows)]
pub fn is_follow_mouse() -> bool {
    tray().follow_mouse
        .get()
        .map(|a| a.load(Ordering::Relaxed))
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn is_follow_mouse() -> bool {
    false
}

#[cfg(windows)]
impl TrayIcon {
    pub fn create(_hwnd: isize, tooltip: &str) -> (Self, Receiver<TrayMessage>) {
        let (tx, rx) = channel();

        unsafe {
            let _ = tray().sender.set(tx.clone());

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

            // 监听 TaskbarCreated：Explorer 重启后系统托盘重建，需重新 NIM_ADD
            let taskbar_msg = RegisterWindowMessageW(windows::core::w!("TaskbarCreated"));
            if taskbar_msg != 0 {
                let _ = tray().taskbar_created_msg.set(taskbar_msg);
            }

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

            let _ = tray().hwnd.set(hwnd);

            let h_icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
            let _ = tray().icon_handle.set(h_icon);

            let mut tip = [0u16; 128];
            let tooltip_wide: Vec<u16> = tooltip.encode_utf16().collect();
            let len = tooltip_wide.len().min(127);
            tip[..len].copy_from_slice(&tooltip_wide[..len]);
            let _ = tray().tip.set(tip);

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
        popup_tray_menu(self.hwnd, &self.sender);
    }
}

#[cfg(windows)]
fn popup_tray_menu(hwnd: HWND, sender: &Sender<TrayMessage>) {
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

        // 本地化文案（默认英文兜底；主线程 set_ui_texts 注入）
        let texts = tray().texts
            .get()
            .and_then(|m| m.lock().ok().and_then(|g| g.clone()))
            .unwrap_or(TrayTexts {
                show: "Show GoRun".to_string(),
                autostart: "Start with Windows".to_string(),
                follow_mouse: "Follow Mouse".to_string(),
                exit: "Exit".to_string(),
            });
        let show_text: Vec<u16> = format!("{}\0", texts.show).encode_utf16().collect();
        let exit_text: Vec<u16> = format!("{}\0", texts.exit).encode_utf16().collect();
        let autostart_text: Vec<u16> = format!("{}\0", texts.autostart).encode_utf16().collect();
        let follow_text: Vec<u16> = format!("{}\0", texts.follow_mouse).encode_utf16().collect();
        let lang_zhcn: Vec<u16> = "中文 (简体)\0".encode_utf16().collect();
        let lang_zhtw: Vec<u16> = "中文 (繁體)\0".encode_utf16().collect();
        let lang_enus: Vec<u16> = "English (US)\0".encode_utf16().collect();

        let _ = AppendMenuW(h_menu, Default::default(), MENU_ID_SHOW as usize, windows::core::PCWSTR(show_text.as_ptr()));

        let autostart = is_autostart_enabled();
        let flags_auto = if autostart { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(h_menu, flags_auto, MENU_ID_AUTOSTART as usize, windows::core::PCWSTR(autostart_text.as_ptr()));

        let follow = tray().follow_mouse
            .get()
            .map(|a| a.load(Ordering::Relaxed))
            .unwrap_or(false);
        let flags_follow = if follow { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(h_menu, flags_follow, MENU_ID_FOLLOW_MOUSE as usize, windows::core::PCWSTR(follow_text.as_ptr()));

        let current_lang = tray().lang
            .get()
            .map(|m| m.lock().map(|s| s.clone()).unwrap_or_else(|_| "zh-CN".to_string()))
            .unwrap_or_else(|| "zh-CN".to_string());
        let flag_zhcn = if current_lang == "zh-CN" { MF_CHECKED } else { MF_UNCHECKED };
        let flag_zhtw = if current_lang == "zh-TW" { MF_CHECKED } else { MF_UNCHECKED };
        let flag_enus = if current_lang == "en-US" { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(h_menu, flag_zhcn, MENU_ID_LANG_ZHCN as usize, windows::core::PCWSTR(lang_zhcn.as_ptr()));
        let _ = AppendMenuW(h_menu, flag_zhtw, MENU_ID_LANG_ZHTW as usize, windows::core::PCWSTR(lang_zhtw.as_ptr()));
        let _ = AppendMenuW(h_menu, flag_enus, MENU_ID_LANG_ENUS as usize, windows::core::PCWSTR(lang_enus.as_ptr()));

        let _ = AppendMenuW(h_menu, Default::default(), MENU_ID_EXIT as usize, windows::core::PCWSTR(exit_text.as_ptr()));

        let mut pos = POINT::default();
        let _ = GetCursorPos(&mut pos);
        let _ = SetForegroundWindow(hwnd);

        let cmd = TrackPopupMenu(
            h_menu,
            TPM_NONOTIFY | TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pos.x,
            pos.y,
            0,
            hwnd,
            None,
        );

        match cmd.0 {
            0 => {}
            id if id == MENU_ID_SHOW as i32 => {
                let _ = sender.send(TrayMessage::Show);
            }
            id if id == MENU_ID_AUTOSTART as i32 => {
                let _ = sender.send(TrayMessage::ToggleAutoStart);
            }
            id if id == MENU_ID_FOLLOW_MOUSE as i32 => {
                let _ = sender.send(TrayMessage::ToggleFollowMouse);
            }
            id if id == MENU_ID_LANG_ZHCN as i32 => {
                let _ = sender.send(TrayMessage::SetLanguage("zh-CN".to_string()));
            }
            id if id == MENU_ID_LANG_ZHTW as i32 => {
                let _ = sender.send(TrayMessage::SetLanguage("zh-TW".to_string()));
            }
            id if id == MENU_ID_LANG_ENUS as i32 => {
                let _ = sender.send(TrayMessage::SetLanguage("en-US".to_string()));
            }
            id if id == MENU_ID_EXIT as i32 => {
                let _ = sender.send(TrayMessage::Quit);
            }
            _ => {}
        }

        let _ = DestroyMenu(h_menu);
    }
}

/// 开机启动注册表路径（HKCU\...\Run）。
#[cfg(windows)]
const AUTOSTART_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const AUTOSTART_VALUE: &str = "GoRun";

/// 读取开机启动是否启用（读注册表，独立于 config 状态）。
#[cfg(windows)]
fn is_autostart_enabled() -> bool {
    unsafe {
        let mut key = HKEY::default();
        let key_w: Vec<u16> = AUTOSTART_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(key_w.as_ptr()), 0, KEY_READ, &mut key).is_err() {
            return false;
        }
        let value_w: Vec<u16> = AUTOSTART_VALUE.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data = [0u8; 1024];
        let mut data_len = data.len() as u32;
        let result = RegQueryValueExW(
            key,
            windows::core::PCWSTR(value_w.as_ptr()),
            None,
            None,
            Some(data.as_mut_ptr()),
            Some(&mut data_len),
        );
        let _ = RegCloseKey(key);
        result.is_ok() && data_len > 0
    }
}

/// 切换开机启动（写入/删除 HKCU Run 值），返回新状态。
#[cfg(windows)]
pub fn toggle_autostart() -> bool {
    unsafe {
        let enabled = is_autostart_enabled();
        let mut key = HKEY::default();
        let key_w: Vec<u16> = AUTOSTART_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(key_w.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .is_err()
        {
            return enabled;
        }
        let value_w: Vec<u16> = AUTOSTART_VALUE.encode_utf16().chain(std::iter::once(0)).collect();
        if enabled {
            let _ = RegDeleteValueW(key, windows::core::PCWSTR(value_w.as_ptr()));
        } else {
            let mut exe_buf = [0u16; 1024];
            let len = GetModuleFileNameW(None, &mut exe_buf);
            if len == 0 || (len as usize) >= exe_buf.len() {
                // 获取 exe 路径失败：注册表未写入，返回原状态避免误报
                let _ = RegCloseKey(key);
                return enabled;
            }
            let exe = String::from_utf16_lossy(&exe_buf[..len as usize]);
            // 含空格路径加引号包裹，防同名影子二进制劫持（对齐 C++ 修正项）
            let exe_quoted = format!("\"{}\"", exe);
            let data: Vec<u8> = exe_quoted.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect();
            let _ = RegSetValueExW(
                key,
                windows::core::PCWSTR(value_w.as_ptr()),
                0,
                REG_SZ,
                Some(&data),
            );
        }
        let _ = RegCloseKey(key);
        !enabled
    }
}

#[cfg(not(windows))]
pub fn toggle_autostart() -> bool {
    log::warn!("Autostart is only supported on Windows");
    false
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

    pub fn show_menu(&self) {}
}

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // TaskbarCreated：Explorer 重启后托盘重建，重新添加图标
    if tray().taskbar_created_msg.get().is_some_and(|m| *m == msg) {
        if let (Some(hwnd), Some(h_icon), Some(tip)) =
            (tray().hwnd.get(), tray().icon_handle.get(), tray().tip.get())
        {
            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: *hwnd,
                uID: TRAY_ICON_ID,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: *h_icon,
                szTip: *tip,
                ..Default::default()
            };
            if Shell_NotifyIconW(NIM_ADD, &mut nid).as_bool() {
                info!("TrayIcon: re-added after taskbar recreation");
            }
        }
        return LRESULT(0);
    }
    match msg {
        WM_TRAYICON => {
            if let Some(sender) = tray().sender.get() {
                match lparam.0 as u32 {
                    WM_LBUTTONDBLCLK => {
                        let _ = sender.send(TrayMessage::Toggle);
                    }
                    WM_RBUTTONUP => {
                        if let Some(hwnd) = tray().hwnd.get() {
                            popup_tray_menu(*hwnd, sender);
                        }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
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
