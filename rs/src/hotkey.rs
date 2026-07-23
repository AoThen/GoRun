#![allow(dead_code)]

use std::sync::mpsc::{channel, Sender};
use std::thread;

const HOTKEY_ID: u32 = 0xC001;

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub modifiers: u32,
    pub vk: u32,
}

pub fn parse_hotkey_string(s: &str) -> Option<Hotkey> {
    let mut modifiers = 0u32;
    let mut vk = 0u32;
    let parts: Vec<&str> = s.split('+').collect();

    for part in &parts {
        let trimmed = part.trim();
        match trimmed.to_lowercase().as_str() {
            "ctrl" => modifiers |= 0x0002,  // MOD_CONTROL
            "alt" => modifiers |= 0x0001,   // MOD_ALT
            "shift" => modifiers |= 0x0004, // MOD_SHIFT
            "win" => modifiers |= 0x0008,   // MOD_WIN
            last => {
                vk = match last.to_uppercase().as_str() {
                    c if c.len() == 1 => {
                        let ch = c.chars().next().unwrap();
                        if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                            ch as u32
                        } else {
                            return None;
                        }
                    }
                    "SPACE" => 0x20,
                    "TAB" => 0x09,
                    "ESCAPE" | "ESC" => 0x1B,
                    "ENTER" => 0x0D,
                    "BACKSPACE" => 0x08,
                    "DELETE" | "DEL" => 0x2E,
                    "INSERT" | "INS" => 0x2D,
                    "HOME" => 0x24,
                    "END" => 0x23,
                    "PAGEUP" => 0x21,
                    "PAGEDOWN" => 0x22,
                    "PRINTSCREEN" => 0x2C,
                    "PAUSE" => 0x13,
                    "NUMLOCK" => 0x90,
                    "CAPSLOCK" => 0x14,
                    "SCROLLLOCK" => 0x91,
                    "F1" => 0x70,
                    "F2" => 0x71,
                    "F3" => 0x72,
                    "F4" => 0x73,
                    "F5" => 0x74,
                    "F6" => 0x75,
                    "F7" => 0x76,
                    "F8" => 0x77,
                    "F9" => 0x78,
                    "F10" => 0x79,
                    "F11" => 0x7A,
                    "F12" => 0x7B,
                    _ => return None,
                };
            }
        }
    }

    if vk == 0 {
        None
    } else {
        Some(Hotkey { modifiers, vk })
    }
}

pub struct HotkeyManager {
    id: u32,
    registered: bool,
    thread_handle: Option<thread::JoinHandle<()>>,
    kill_sender: Option<Sender<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            id: HOTKEY_ID,
            registered: false,
            thread_handle: None,
            kill_sender: None,
        }
    }

    pub fn register(&mut self, hotkey_str: &str) -> bool {
        let hk = match parse_hotkey_string(hotkey_str) {
            Some(h) => h,
            None => {
                log::error!(
                    "HotkeyManager: failed to parse hotkey string: {}",
                    hotkey_str
                );
                return false;
            }
        };

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::TRUE;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey,
            };

            if self.registered {
                let _ = UnregisterHotKey(std::ptr::null_mut(), self.id as i32);
            }

            let result = RegisterHotKey(std::ptr::null_mut(), self.id as i32, hk.modifiers, hk.vk);

            if result == TRUE {
                log::info!(
                    "HotkeyManager: registered hotkey '{}' (id=0x{:X})",
                    hotkey_str,
                    self.id
                );
                self.registered = true;
                true
            } else {
                log::error!("HotkeyManager: RegisterHotKey failed for '{}'", hotkey_str);
                self.registered = false;
                false
            }
        }

        #[cfg(not(windows))]
        {
            let _ = hk;
            log::warn!("HotkeyManager: hotkey registration not supported on this platform");
            false
        }
    }

    pub fn register_with_notifier(
        &mut self,
        hotkey_str: &str,
        sender: Sender<()>,
    ) -> bool {
        if !self.register(hotkey_str) {
            return false;
        }

        let (kill_tx, kill_rx) = channel::<()>();
        self.kill_sender = Some(kill_tx);

        let hotkey_id = self.id;
                     let handle = thread::Builder::new()
            .name("hotkey-pump".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        DispatchMessageW, PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY,
                    };
                    use windows_sys::Win32::Foundation::TRUE;
                    use windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

                     let mut msg: MSG = std::mem::zeroed();
                    loop {
                        if kill_rx.try_recv().is_ok() {
                            log::info!("HotkeyManager: message pump thread received kill signal");
                            break;
                        }

                        let result = PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE);
                        if result == TRUE {
                            if msg.message == WM_HOTKEY && msg.wParam as u32 == hotkey_id {
                                let _ = sender.send(());
                            }
                            let _ = DispatchMessageW(&msg);
                        } else {
                            // No message available, sleep briefly to avoid busy-wait
                            thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }

                    let _ = UnregisterHotKey(std::ptr::null_mut(), hotkey_id as i32);
                }

                #[cfg(not(windows))]
                {
                    let _ = &sender;
                    let _ = &kill_rx;
                    let _ = hotkey_id;
                    log::warn!("HotkeyManager: hotkey notifier thread not supported on this platform");
                }
            });

        match handle {
            Ok(h) => {
                self.thread_handle = Some(h);
                log::info!("HotkeyManager: message pump thread started");
                true
            }
            Err(e) => {
                log::error!("HotkeyManager: failed to start message pump thread: {}", e);
                false
            }
        }
    }

    pub fn unregister(&mut self) -> bool {
        self.kill_pump_thread();

        if !self.registered {
            return true;
        }

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::TRUE;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

            let result = UnregisterHotKey(std::ptr::null_mut(), self.id as i32);
            if result == TRUE {
                log::info!("HotkeyManager: unregistered hotkey (id=0x{:X})", self.id);
                self.registered = false;
                true
            } else {
                log::error!(
                    "HotkeyManager: UnregisterHotKey failed (id=0x{:X})",
                    self.id
                );
                false
            }
        }

        #[cfg(not(windows))]
        {
            self.registered = false;
            true
        }
    }

    pub fn process_message(&self, msg: u32, wparam: usize) -> bool {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::WM_HOTKEY;
            if msg == WM_HOTKEY && wparam as u32 == self.id {
                return true;
            }
        }
        let _ = msg;
        let _ = wparam;
        false
    }

    pub fn update(&mut self, hotkey_str: &str) -> bool {
        self.unregister();
        self.register(hotkey_str)
    }

    pub fn is_registered(&self) -> bool {
        self.registered
    }

    fn kill_pump_thread(&mut self) {
        if let Some(kill_tx) = self.kill_sender.take() {
            let _ = kill_tx.send(());
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            log::info!("HotkeyManager: message pump thread joined");
        }
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ctrl_alt_m() {
        let hk = parse_hotkey_string("Ctrl+Alt+M").unwrap();
        assert_eq!(hk.modifiers, 0x0002 | 0x0001);
        assert_eq!(hk.vk, 'M' as u32);
    }

    #[test]
    fn test_parse_f12() {
        let hk = parse_hotkey_string("Ctrl+F12").unwrap();
        assert_eq!(hk.modifiers, 0x0002);
        assert_eq!(hk.vk, 0x7B);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_hotkey_string("").is_none());
        assert!(parse_hotkey_string("Ctrl+").is_none());
    }

    #[test]
    fn test_hotkey_manager_new() {
        let mgr = HotkeyManager::new();
        assert!(!mgr.is_registered());
    }

    #[test]
    fn test_hotkey_manager_register_invalid() {
        let mut mgr = HotkeyManager::new();
        assert!(!mgr.register(""));
        assert!(!mgr.is_registered());
    }

    #[test]
    fn test_hotkey_manager_default() {
        let mgr: HotkeyManager = Default::default();
        assert!(!mgr.is_registered());
    }

    #[test]
    fn test_hotkey_manager_update_invalid() {
        let mut mgr = HotkeyManager::new();
        assert!(!mgr.update("InvalidKey"));
        assert!(!mgr.is_registered());
    }

    #[test]
    fn test_process_message_non_hotkey() {
        let mgr = HotkeyManager::new();
        assert!(!mgr.process_message(0x00, 0));
    }
}
