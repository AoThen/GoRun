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
        loc
    }

    fn lang_file_path(code: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}.json", LANG_DIR, code))
    }

    fn load(&mut self) {
        let path = Self::lang_file_path(&self.lang_code);
        if !path.exists() {
            return;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<HashMap<String, String>>(&content) {
                self.strings = json;
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
