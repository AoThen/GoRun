#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const LANG_DIR: &str = "lang";
const DEFAULT_LANG: &str = "zh-CN";

pub struct Localization {
    strings: HashMap<String, String>,
    lang_code: String,
}

impl Localization {
    pub fn new(lang_code: &str) -> Self {
        let mut loc = Localization {
            strings: HashMap::new(),
            lang_code: lang_code.to_string(),
        };
        loc.load();
        log::info!("Localization created: lang={}", lang_code);
        loc
    }

    fn lang_file_path(code: &str) -> PathBuf {
        let relative = format!("{}/{}.json", LANG_DIR, code);

        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let exe_lang = exe_dir.join(&relative);
                if exe_lang.exists() {
                    return exe_lang;
                }
            }
        }

        PathBuf::from(relative)
    }

    fn load(&mut self) {
        let path = Self::lang_file_path(&self.lang_code);
        if !path.exists() {
            log::warn!("Language file not found: {:?}", path);
            return;
        }
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<HashMap<String, String>>(&content) {
                    Ok(json) => {
                        log::info!("Language file loaded: {:?}, {} strings", path, json.len());
                        self.strings = json;
                    }
                    Err(_) => {
                        log::error!("Failed to parse language JSON: {:?}", path);
                    }
                }
            }
            Err(_) => {
                log::error!("Failed to read language file: {:?}", path);
            }
        }
    }

    pub fn tr<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    pub fn language_code(&self) -> &str {
        &self.lang_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_key_returns_key() {
        let loc = Localization::new("nonexistent_lang");
        assert_eq!(loc.tr("SomeKey"), "SomeKey");
    }
}
