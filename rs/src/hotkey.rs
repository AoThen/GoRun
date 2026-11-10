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

    for (idx, part) in parts.iter().enumerate() {
        let is_last = idx == parts.len() - 1;
        let trimmed = part.trim();
        match trimmed.to_lowercase().as_str() {
            "ctrl" => modifiers |= 0x0002,  // MOD_CONTROL
            "alt" => modifiers |= 0x0001,   // MOD_ALT
            "shift" => modifiers |= 0x0004, // MOD_SHIFT
            "win" => modifiers |= 0x0008,   // MOD_WIN
            last => {
                if !is_last {
                    return None;
                }
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
    pump_active: bool,
    thread_handle: Option<thread::JoinHandle<()>>,
    kill_sender: Option<Sender<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            id: HOTKEY_ID,
            registered: false,
            pump_active: false,
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
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
            };

            if self.registered {
                let _ = UnregisterHotKey(None, self.id as i32);
            }

            let result =
                RegisterHotKey(None, self.id as i32, HOT_KEY_MODIFIERS(hk.modifiers), hk.vk);

            if result.is_ok() {
                log::info!(
                    "HotkeyManager: registered hotkey '{}' (id=0x{:X})",
                    hotkey_str,
                    self.id
                );
                self.registered = true;
                true
            } else {
                log::error!(
                    "HotkeyManager: RegisterHotKey failed for '{}': {:?}",
                    hotkey_str,
                    result.err()
                );
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

        let (kill_tx, kill_rx) = channel::<()>();
        self.kill_sender = Some(kill_tx);
        let (result_tx, result_rx) = channel::<bool>();
        let hotkey_id = self.id;

        let handle = thread::Builder::new()
            .name("hotkey-pump".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use windows::Win32::Foundation::{FALSE, TRUE};
                    use windows::Win32::UI::Input::KeyboardAndMouse::{
                        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
                    };
                    use windows::Win32::UI::WindowsAndMessaging::{
                        DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, QS_ALLINPUT,
                        MSG, PM_REMOVE, WM_HOTKEY,
                    };

                    let ok = RegisterHotKey(
                        None,
                        hotkey_id as i32,
                        HOT_KEY_MODIFIERS(hk.modifiers),
                        hk.vk,
                    )
                    .is_ok();
                    let _ = result_tx.send(ok);
                    if !ok {
                        return;
                    }

                    let mut msg: MSG = std::mem::zeroed();
                    loop {
                        if kill_rx.try_recv().is_ok() {
                            log::info!("HotkeyManager: message pump thread received kill signal");
                            break;
                        }
                        // P2-8：MsgWaitForMultipleObjects + 500ms 超时阻塞等待，
                        // 替代 50ms 忙轮询 sleep；空闲时 CPU 占用近零。
                        // 不传事件句柄（None），唤醒掩码 QS_ALLINPUT；有消息时返回
                        // 值含 MWMO_MESSAGE (0x80000000)，超时返回 WAIT_TIMEOUT (258)
                        let waited = MsgWaitForMultipleObjects(None, FALSE, 500, QS_ALLINPUT);
                        if waited.0 == 0x80000000 {
                            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) == TRUE {
                                if msg.message == WM_HOTKEY && msg.wParam.0 as u32 == hotkey_id {
                                    let _ = sender.send(());
                                }
                                let _ = DispatchMessageW(&msg);
                            }
                        }
                    }

                    let _ = UnregisterHotKey(None, hotkey_id as i32);
                }

                #[cfg(not(windows))]
                {
                    let _ = &sender;
                    let _ = &kill_rx;
                    let _ = &hk;
                    let _ = hotkey_id;
                    // 必须回送结果，否则主线程 result_rx.recv() 永久阻塞
                    let _ = result_tx.send(false);
                    log::warn!("HotkeyManager: hotkey notifier thread not supported on this platform");
                }
            });

        match handle {
            Ok(h) => {
                self.thread_handle = Some(h);
                // P3-10：recv_timeout 兜底，pump 线程异常未回送结果时避免主线程永久挂起
                let ok = result_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap_or(false);
                self.registered = ok;
                self.pump_active = ok;
                if ok {
                    log::info!(
                        "HotkeyManager: registered hotkey '{}' (id=0x{:X})",
                        hotkey_str,
                        self.id
                    );
                } else {
                    log::error!("HotkeyManager: RegisterHotKey failed for '{}'", hotkey_str);
                }
                ok
            }
            Err(e) => {
                log::error!("HotkeyManager: failed to start message pump thread: {}", e);
                false
            }
        }
    }

    pub fn unregister(&mut self) -> bool {
        let had_pump = self.pump_active;
        self.kill_pump_thread();
        self.pump_active = false;

        if !self.registered {
            return true;
        }

        #[cfg(windows)]
        unsafe {
            if had_pump {
                self.registered = false;
                return true;
            }
            use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

            let result = UnregisterHotKey(None, self.id as i32);
            if result.is_ok() {
                log::info!("HotkeyManager: unregistered hotkey (id=0x{:X})", self.id);
                self.registered = false;
                true
            } else {
                log::error!(
                    "HotkeyManager: UnregisterHotKey failed (id=0x{:X}): {:?}",
                    self.id,
                    result.err()
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

}
