#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewType {
    Icon = 0,
    List = 1,
}

impl Default for ViewType {
    fn default() -> Self {
        ViewType::Icon
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}

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

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub version: String,
    pub data: AppData,
    pub config: Config,
    pub extra_config: std::collections::HashMap<String, String>,
}

pub fn generate_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}_{}", prefix, ms)
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
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct RunResult {
    pub success: bool,
    pub error: RunError,
    pub error_message: String,
}
