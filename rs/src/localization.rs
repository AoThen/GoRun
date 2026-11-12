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

    /// 取键对应文案；缺失时回退为键本身（便于发现漏翻译）。
    ///
    /// 注意：返回值同时借用 `self` 与 `key`，因此 `key` 为临时值时需先绑定局部变量
    /// 或直接 `.to_string()`。若要放开该限制，应改为返回 `Cow<'a, str>`，
    /// 但那会波及全部调用点，暂不采用。
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
