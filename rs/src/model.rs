use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewType {
    #[default]
    Icon = 0,
    List = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Category {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub view_type: ViewType,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
}

fn default_icon_size() -> i32 {
    48
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Item {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub arguments: String,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub icon_path: String,
    /// 自动提取图标的缓存 PNG 绝对路径。与 `icon_path`（用户自定义图标源）区分：
    /// `icon_path` 为空时图标由 target 提取，缓存路径必须落盘才能跨进程恢复，
    /// 否则重启后自动提取的图标全部空白。
    #[serde(default)]
    pub cached_icon: String,
    #[serde(default)]
    pub icon_index: i32,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub run_count: i32,
    #[serde(default)]
    pub keywords: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub category_id: String,
    #[serde(default)]
    pub sort_order: i32,
    /// 条目级全局热键（F8/4.1）：如 "Ctrl+Shift+F1"；空=未设置。
    #[serde(default)]
    pub hotkey: String,
}

pub const DEFAULT_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Default)]
pub struct AppData {
    pub categories: Vec<Category>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub global_hotkey: String,
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            global_hotkey: "Ctrl+Alt+M".to_string(),
            window_x: 100,
            window_y: 100,
            window_width: 800,
            window_height: 600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub version: String,
    pub data: AppData,
    pub config: Config,
    pub extra_config: std::collections::HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            version: DEFAULT_VERSION.to_string(),
            data: AppData::default(),
            config: Config::default(),
            extra_config: std::collections::HashMap::new(),
        }
    }
}

pub fn generate_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{}_{}", prefix, ms, seq)
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum RunError {
    #[default]
    None,
    FileNotFound,
    PathNotFound,
    AccessDenied,
    OutOfMemory,
    DllNotFound,
    /// 用户取消了 UAC 提权请求（ShellExecuteEx ERROR_CANCELLED）
    Cancelled,
    /// 搜索参数含 shell 元字符，拒绝注入到 shell 型目标（P0-1）
    UnsafeSearchParam,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct RunResult {
    pub success: bool,
    pub error: RunError,
    pub error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_config_default_values() {
        let c = Config::default();
        assert_eq!(c.global_hotkey, "Ctrl+Alt+M");
        assert_eq!(c.window_x, 100);
        assert_eq!(c.window_y, 100);
        assert_eq!(c.window_width, 800);
        assert_eq!(c.window_height, 600);
    }

    #[test]
    fn test_app_config_default() {
        let a = AppConfig::default();
        assert_eq!(a.version, DEFAULT_VERSION);
        assert_eq!(a.version, "1.0.0");
        assert!(a.data.categories.is_empty());
        assert!(a.data.items.is_empty());
        assert_eq!(a.config.window_width, 800);
        assert!(a.extra_config.is_empty());
    }

    #[test]
    fn test_view_type_default_is_icon() {
        assert_eq!(ViewType::default(), ViewType::Icon);
    }

    #[test]
    fn test_run_error_default_is_none() {
        assert_eq!(RunError::default(), RunError::None);
    }

    #[test]
    fn test_run_result_default() {
        let r = RunResult::default();
        assert!(!r.success);
        assert_eq!(r.error, RunError::None);
        assert!(r.error_message.is_empty());
    }

    #[test]
    fn test_generate_id_format() {
        let id = generate_id("cat");
        assert!(id.starts_with("cat_"), "id 应以前缀开头: {}", id);
        // 格式 prefix_ms_seq
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 3, "id 应含三段: {}", id);
        assert_eq!(parts[0], "cat");
        assert!(parts[1].parse::<u128>().is_ok(), "中段应为毫秒时间戳: {}", id);
        assert!(parts[2].parse::<u64>().is_ok(), "末段应为序号: {}", id);
    }

    #[test]
    fn test_generate_id_unique_within_same_ms() {
        // 同一毫秒内连续生成应因序号递增而唯一（storage_test 已有，此处补前缀差异）
        let a = generate_id("item");
        let b = generate_id("item");
        assert_ne!(a, b);
        let c = generate_id("cat");
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn test_item_serde_roundtrip() {
        let item = Item {
            id: "i1".into(),
            name: "Notepad".into(),
            target: "C:\\Windows\\notepad.exe".into(),
            arguments: "--foo".into(),
            working_dir: "C:\\".into(),
            icon_path: "C:\\icon.ico".into(),
            cached_icon: "C:\\cache.png".into(),
            icon_index: 2,
            run_as_admin: true,
            run_count: 7,
            keywords: "foo bar".into(),
            remark: "note".into(),
            category_id: "cat_x".into(),
            sort_order: 3,
            hotkey: "Ctrl+Shift+F1".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, back);
    }

    #[test]
    fn test_item_serde_defaults_missing_fields() {
        // 缺失字段（serde(default)）应填默认：id 空串、icon_index 0、hotkey 空串
        let json = r#"{"name":"x","target":"y"}"#;
        let item: Item = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "");
        assert_eq!(item.icon_index, 0);
        assert_eq!(item.hotkey, "");
        assert_eq!(item.run_as_admin, false);
    }

    #[test]
    fn test_category_serde_roundtrip_fields() {
        let cat = Category {
            id: "cat_x".into(),
            name: "Apps".into(),
            sort_order: 2,
            view_type: ViewType::List,
            icon_size: 64,
        };
        let json = serde_json::to_string(&cat).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cat.id);
        assert_eq!(back.name, cat.name);
        assert_eq!(back.sort_order, cat.sort_order);
        assert_eq!(back.view_type, cat.view_type);
        assert_eq!(back.icon_size, cat.icon_size);
    }

    #[test]
    fn test_category_default_icon_size() {
        // Rust Default derive → icon_size=0（不是 48；serde 的 default 只影响反序列化）
        let cat = Category::default();
        assert_eq!(cat.icon_size, 0, "Rust Default 的 icon_size 应为 0");
        assert_eq!(cat.view_type, ViewType::Icon);
        // serde 缺失 icon_size 字段 → default_icon_size()=48
        let json = r#"{"id":"c","name":"Apps"}"#;
        let cat: Category = serde_json::from_str(json).unwrap();
        assert_eq!(cat.icon_size, 48, "serde 缺失字段应回退到 default_icon_size()=48");
    }

    #[test]
    fn test_app_data_default() {
        let d = AppData::default();
        assert!(d.categories.is_empty());
        assert!(d.items.is_empty());
    }
}
