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
            "ctrl" => modifiers |= 0x0002, // MOD_CONTROL
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
}
