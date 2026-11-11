// 本地化：语言文件编译期嵌入（resources/lang/*.json），tr() 取键值；
// 语言切换时构造新实例重新解析。UI 文本接线：菜单栏 + 空状态（最小版）。

use std::collections::HashMap;

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

    fn load(&mut self) {
        // 绿色软件：语言文件编译期嵌入，无需运行时外部文件
        let content = match self.lang_code.as_str() {
            "zh-TW" => include_str!("../../resources/lang/zh-TW.json"),
            "en-US" => include_str!("../../resources/lang/en-US.json"),
            _ => include_str!("../../resources/lang/zh-CN.json"),
        };
        match serde_json::from_str::<HashMap<String, String>>(content) {
            Ok(json) => {
                log::info!(
                    "Language file loaded: {}, {} strings",
                    self.lang_code,
                    json.len()
                );
                self.strings = json;
            }
            Err(_) => {
                log::error!("Failed to parse language JSON: {}", self.lang_code);
            }
        }
    }

    pub fn tr<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    #[allow(dead_code)]
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
