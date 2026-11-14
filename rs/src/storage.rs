use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::model::{AppConfig, AppData, Category, Config, Item, ViewType};

#[derive(Serialize, Deserialize)]
struct RawFile {
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    items: Vec<RawItem>,
    #[serde(default)]
    config: RawConfig,
}

#[derive(Serialize, Deserialize)]
struct RawConfig {
    #[serde(rename = "globalHotkey", default = "default_hotkey")]
    global_hotkey: String,
    #[serde(
        rename = "windowX",
        default = "default_window_x",
        deserialize_with = "de_i32_str_or_num",
        serialize_with = "ser_i32_as_str"
    )]
    window_x: i32,
    #[serde(
        rename = "windowY",
        default = "default_window_y",
        deserialize_with = "de_i32_str_or_num",
        serialize_with = "ser_i32_as_str"
    )]
    window_y: i32,
    #[serde(
        rename = "windowWidth",
        default = "default_window_width",
        deserialize_with = "de_i32_str_or_num",
        serialize_with = "ser_i32_as_str"
    )]
    window_width: i32,
    #[serde(
        rename = "windowHeight",
        default = "default_window_height",
        deserialize_with = "de_i32_str_or_num",
        serialize_with = "ser_i32_as_str"
    )]
    window_height: i32,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            global_hotkey: default_hotkey(),
            window_x: default_window_x(),
            window_y: default_window_y(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            extra: HashMap::new(),
        }
    }
}

fn default_version() -> String {
    crate::model::DEFAULT_VERSION.to_string()
}
fn default_hotkey() -> String {
    "Ctrl+Alt+M".to_string()
}
fn default_window_x() -> i32 {
    100
}
fn default_window_y() -> i32 {
    100
}
fn default_window_width() -> i32 {
    800
}
fn default_window_height() -> i32 {
    600
}

fn de_i32_str_or_num<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(i32),
        Str(String),
    }
    match IntOrString::deserialize(deserializer)? {
        IntOrString::Int(v) => Ok(v),
        IntOrString::Str(s) => s.parse::<i32>().map_err(serde::de::Error::custom),
    }
}

fn ser_i32_as_str<S>(value: &i32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Serialize, Deserialize)]
struct RawCategory {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "sortOrder", default)]
    sort_order: i32,
    #[serde(rename = "viewType", default)]
    view_type: i32,
    #[serde(rename = "iconSize", default = "default_icon_size")]
    icon_size: i32,
}

fn default_icon_size() -> i32 {
    48
}

#[derive(Serialize, Deserialize)]
struct RawItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    arguments: String,
    #[serde(rename = "workingDir", default)]
    working_dir: String,
    #[serde(rename = "iconPath", default)]
    icon_path: String,
    /// 自动提取图标的缓存路径（P0）：重启后据此恢复图标。
    #[serde(rename = "cachedIcon", default)]
    cached_icon: String,
    #[serde(rename = "iconIndex", default)]
    icon_index: i32,
    #[serde(rename = "runAsAdmin", default)]
    run_as_admin: bool,
    #[serde(rename = "runCount", default)]
    run_count: i32,
    #[serde(default)]
    keywords: String,
    #[serde(default)]
    remark: String,
    #[serde(rename = "categoryId", default)]
    category_id: String,
    #[serde(rename = "sortOrder", default)]
    sort_order: i32,
    #[serde(default)]
    hotkey: String,
}

/// P0-2 完整性值：对配置内容（version/categories/items/config/排序后的 extra）做
/// HMAC-SHA256。密钥编译进二进制（可被逆向提取，但比公开哈希门槛显著提高），
/// 防的是"读源码即可伪造"的普通攻击者；真正信任控制点是启动确认弹窗，
/// 此值用于把"外部/被篡改"从"仅缺标记"升级为可检测。
/// 前缀 `hmac:` 用于区分新旧格式：旧版 `fnv1a64:` 配置在升级后无法匹配，
/// 会触发隔离 + 确认弹窗，强制用户审阅目标后重新签名（预期安全行为）。
const INTEGRITY_KEY: &[u8] = b"GoRun-rs-HMAC-Key-v1";

pub fn compute_integrity(cfg: &AppConfig) -> String {
    let extra: std::collections::BTreeMap<&str, &str> = cfg
        .extra_config
        .iter()
        .filter(|(k, _)| k.as_str() != "config_integrity")
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let payload = serde_json::json!({
        "version": cfg.version,
        "categories": serde_json::to_value(&cfg.data.categories).unwrap_or_default(),
        "items": serde_json::to_value(&cfg.data.items).unwrap_or_default(),
        "config": {
            "globalHotkey": cfg.config.global_hotkey,
            "windowX": cfg.config.window_x.to_string(),
            "windowY": cfg.config.window_y.to_string(),
            "windowWidth": cfg.config.window_width.to_string(),
            "windowHeight": cfg.config.window_height.to_string(),
        },
        "extra": serde_json::to_value(&extra).unwrap_or_default(),
    });
    let mut mac =
        Hmac::<Sha256>::new_from_slice(INTEGRITY_KEY).expect("HMAC key length must be valid");
    mac.update(payload.to_string().as_bytes());
    let tag = mac.finalize().into_bytes();
    format!("hmac:{}", tag.iter().map(|b| format!("{:02x}", b)).collect::<String>())
}

/// P0-2 信任校验：标记值精确匹配 + 内容完整性匹配，两者缺一即视为未信任。
pub fn verify_integrity(cfg: &AppConfig) -> bool {
    cfg.extra_config
        .get("generated_by")
        .map(|v| v == "gorun-rs")
        .unwrap_or(false)
        && cfg
            .extra_config
            .get("config_integrity")
            .map(|v| v == &compute_integrity(cfg))
            .unwrap_or(false)
}

pub struct Storage {
    path: String,
}

impl Storage {
    pub fn new(path: String) -> Self {
        Storage { path }
    }

    pub fn load(&self) -> Option<AppConfig> {
        if !Path::new(&self.path).exists() {
            log::warn!("Config file not found: {}", self.path);
            return None;
        }
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => {
                log::error!("Failed to open config file: {}", self.path);
                return None;
            }
        };
        let reader = BufReader::new(file);
        let raw: RawFile = match serde_json::from_reader(reader) {
            Ok(r) => r,
            Err(_) => {
                log::error!("Failed to parse config JSON: {}", self.path);
                return None;
            }
        };

        let raw_config = raw.config;
        let extra_config: HashMap<String, String> = raw_config
            .extra
            .into_iter()
            .map(|(k, v)| {
                let s = v
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string());
                (k, s)
            })
            .collect();

        let config = Config {
            global_hotkey: raw_config.global_hotkey,
            window_x: raw_config.window_x,
            window_y: raw_config.window_y,
            window_width: raw_config.window_width,
            window_height: raw_config.window_height,
        };

        let mut categories: Vec<Category> = raw
            .categories
            .into_iter()
            .map(|c| Category {
                id: c.id,
                name: c.name,
                sort_order: c.sort_order,
                view_type: if c.view_type == 1 {
                    ViewType::List
                } else {
                    ViewType::Icon
                },
                icon_size: c.icon_size,
            })
            .collect();

        let mut items: Vec<Item> = raw
            .items
            .into_iter()
            .map(|i| Item {
                id: i.id,
                name: i.name,
                target: i.target,
                arguments: i.arguments,
                working_dir: i.working_dir,
                icon_path: i.icon_path,
                cached_icon: i.cached_icon,
                icon_index: i.icon_index.max(0),
                run_as_admin: i.run_as_admin,
                run_count: i.run_count.max(0),
                keywords: i.keywords,
                remark: i.remark,
                category_id: i.category_id,
                sort_order: i.sort_order,
                hotkey: i.hotkey,
            })
            .collect();

        // P2-03：加载时按 sortOrder 稳定排序（分类按 sort_order；项目按
        // (category_id, sort_order)），一次性修复 C++ 生成配置中"数组序≠sortOrder 序"
        // 的历史数据，保证迁移后展示顺序与 C++ 渲染端一致。
        categories.sort_by_key(|c| c.sort_order);
        items.sort_by_key(|i| (i.category_id.clone(), i.sort_order));

        log::info!(
            "Config loaded: {} categories, {} items",
            categories.len(),
            items.len()
        );

        Some(AppConfig {
            version: raw.version,
            data: AppData { categories, items },
            config,
            extra_config,
        })
    }

/// 把内存配置序列化为导出/落盘用的 RawFile（重编号 sortOrder，写完整性值）。
fn build_raw(app_config: &AppConfig) -> RawFile {
    let raw_categories: Vec<RawCategory> = app_config
        .data
        .categories
        .iter()
        .enumerate()
        .map(|(idx, c)| RawCategory {
            id: c.id.clone(),
            name: c.name.clone(),
            sort_order: idx as i32,
            view_type: c.view_type.clone() as i32,
            icon_size: c.icon_size,
        })
        .collect();

    let mut cat_counters: HashMap<&str, i32> = HashMap::new();
    let raw_items: Vec<RawItem> = app_config
        .data
        .items
        .iter()
        .map(|i| {
            let so = cat_counters.entry(i.category_id.as_str()).or_insert(0);
            let cur = *so;
            *so += 1;
            RawItem {
                id: i.id.clone(),
                name: i.name.clone(),
                target: i.target.clone(),
                arguments: i.arguments.clone(),
                working_dir: i.working_dir.clone(),
                icon_path: i.icon_path.clone(),
                cached_icon: i.cached_icon.clone(),
                icon_index: i.icon_index,
                run_as_admin: i.run_as_admin,
                run_count: i.run_count,
                keywords: i.keywords.clone(),
                remark: i.remark.clone(),
                category_id: i.category_id.clone(),
                sort_order: cur,
                hotkey: i.hotkey.clone(),
            }
        })
        .collect();

    let mut extra = HashMap::new();
    // 此剔除列表必须与 RawConfig 显式字段集合保持同步，否则 serde flatten 会输出重复键
    for (k, v) in &app_config.extra_config {
        if matches!(
            k.as_str(),
            "globalHotkey"
                | "windowX"
                | "windowY"
                | "windowWidth"
                | "windowHeight"
                | "config_integrity"
        ) {
            continue;
        }
        extra.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    // P0-2：落盘前写入当前配置内容的完整性值，旧值/外部篡改在启动校验时被发现
    extra.insert(
        "config_integrity".to_string(),
        serde_json::Value::String(compute_integrity(app_config)),
    );

    let raw_config = RawConfig {
        global_hotkey: app_config.config.global_hotkey.clone(),
        window_x: app_config.config.window_x,
        window_y: app_config.config.window_y,
        window_width: app_config.config.window_width,
        window_height: app_config.config.window_height,
        extra,
    };

    RawFile {
        version: app_config.version.clone(),
        categories: raw_categories,
        items: raw_items,
        config: raw_config,
    }
}

    pub fn save(&self, app_config: &AppConfig) -> bool {
        // P2-03：保存前按数组顺序重编号 sortOrder（0..n-1 / 分类内 0..m-1），
        // 保证写回的 JSON 数组序 ≡ sortOrder 序，与 C++ 渲染端排序语义恒一致。
        let raw = Self::build_raw(app_config);

        let json = match serde_json::to_string(&raw) {
            Ok(s) => s,
            Err(_) => {
                log::error!("Failed to serialize config to JSON");
                return false;
            }
        };

        let path = Path::new(&self.path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let tmp_path = format!("{}.tmp.{}", self.path, std::process::id());
        let write_result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&tmp_path)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(e) = write_result {
            let _ = fs::remove_file(&tmp_path);
            log::error!("Failed to write temp file: {} ({})", tmp_path, e);
            return false;
        }
        if fs::rename(&tmp_path, &self.path).is_err() {
            let _ = fs::remove_file(&tmp_path);
            log::error!("Failed to rename temp to config: {}", self.path);
            return false;
        }
        log::info!("Config saved to: {}", self.path);
        true
    }

    /// F8：导出当前配置为 JSON 到指定路径（不自持 self.path，供用户选目标文件）。
    pub fn export_to(&self, path: &str, app_config: &AppConfig) -> bool {
        let raw = Self::build_raw(app_config);
        let json = match serde_json::to_string_pretty(&raw) {
            Ok(s) => s,
            Err(_) => {
                log::error!("Failed to serialize config for export");
                return false;
            }
        };
        let target = Path::new(path);
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(target, json.as_bytes()).is_err() {
            log::error!("Failed to write export file: {}", path);
            return false;
        }
        log::info!("Config exported to: {}", path);
        true
    }

    pub fn rotate_backups(&self) -> bool {
        let path = Path::new(&self.path);
        if !path.exists() {
            return true;
        }

        let bak3 = format!("{}.bak3", self.path);
        if Path::new(&bak3).exists() && fs::remove_file(&bak3).is_err() {
            log::error!("Failed to delete old backup: {}", bak3);
            return false;
        }

        let bak2 = format!("{}.bak2", self.path);
        if Path::new(&bak2).exists() && fs::rename(&bak2, &bak3).is_err() {
            log::error!("Failed to rotate backup .bak2 to .bak3");
            return false;
        }

        let bak1 = format!("{}.bak1", self.path);
        if Path::new(&bak1).exists() && fs::rename(&bak1, &bak2).is_err() {
            log::error!("Failed to rotate backup .bak1 to .bak2");
            return false;
        }

        if fs::copy(&self.path, &bak1).is_err() {
            log::error!("Failed to create backup .bak1");
            return false;
        }

        log::info!("Backup rotation completed");
        true
    }

    pub fn load_from_backup(&self) -> Option<AppConfig> {
        for suffix in &[".bak1", ".bak2", ".bak3"] {
            let backup_path = format!("{}{}", self.path, suffix);
            if Path::new(&backup_path).exists() {
                log::info!("Trying to load from backup: {}", backup_path);
                let backup_storage = Storage::new(backup_path.clone());
                if let Some(config) = backup_storage.load() {
                    // P1-05：仅当备份"声称是 gorun-rs 签名配置"（带 generated_by 标记）
                    // 但完整性校验不通过时才视为被篡改并跳过；未签名的旧版/迁移配置
                    // 仍按最佳恢复处理（恢复时由 save 重新签名），避免破坏既有恢复路径。
                    let is_signed = config
                        .extra_config
                        .get("generated_by")
                        .map(|v| v == "gorun-rs")
                        .unwrap_or(false);
                    if is_signed && !verify_integrity(&config) {
                        log::warn!("Backup {} failed integrity check, trying next", backup_path);
                        continue;
                    }
                    log::info!("Successfully loaded from backup: {}", backup_path);
                    if self.save(&config) {
                        log::info!("Restored backup to main config file");
                    }
                    return Some(config);
                }
            }
        }
        log::warn!("No valid backup found");
        None
    }

    pub fn save_with_backup(&self, app_config: &AppConfig) -> bool {
        if !self.rotate_backups() {
            log::warn!("Backup rotation failed, continuing save without backup");
        }
        self.save(app_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::generate_id;

    #[test]
    fn test_save_and_load_roundtrip() {
        let mut config = AppConfig::default();
        config.version = "1.0.0".to_string();
        config.data.categories.push(Category {
            id: generate_id("cat"),
            name: "Test Category".to_string(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        });
        config.data.items.push(Item {
            id: generate_id("item"),
            name: "Test Item".to_string(),
            target: "C:\\Windows\\notepad.exe".to_string(),
            ..Default::default()
        });
        config.config.global_hotkey = "Ctrl+Alt+M".to_string();

        let path = std::env::temp_dir()
            .join("gorun_test_config.json")
            .to_str()
            .unwrap()
            .to_string();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.data.categories.len(), 1);
        assert_eq!(loaded.data.categories[0].name, "Test Category");
        assert_eq!(loaded.data.items.len(), 1);
        assert_eq!(loaded.data.items[0].target, "C:\\Windows\\notepad.exe");
        assert_eq!(loaded.config.global_hotkey, "Ctrl+Alt+M");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_integrity_roundtrip_and_tamper_detection() {
        let mut config = AppConfig::default();
        config.extra_config.insert("generated_by".to_string(), "gorun-rs".to_string());
        config.data.items.push(Item {
            id: "item_x".into(),
            name: "Foo".into(),
            target: "C:\\foo.exe".into(),
            ..Default::default()
        });

        let path = std::env::temp_dir()
            .join(format!("gorun_integrity_test_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));

        let loaded = storage.load().unwrap();
        // 正常往返：标记 + 完整性均通过
        assert!(verify_integrity(&loaded));

        // 篡改 target 后校验失败
        let mut tampered = loaded.clone();
        tampered.data.items[0].target = "C:\\evil.exe".to_string();
        assert!(!verify_integrity(&tampered));

        // 重新保存后重签，再次通过
        assert!(storage.save(&tampered));
        let reloaded = storage.load().unwrap();
        assert!(verify_integrity(&reloaded));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_verify_rejects_forged_generated_by() {
        // 仅伪造 generated_by 标记（无完整性值）不能通过校验（P0-2）
        let mut config = AppConfig::default();
        config.extra_config.insert("generated_by".to_string(), "gorun-rs".to_string());
        config.data.items.push(Item {
            id: "item_x".into(),
            name: "Foo".into(),
            target: "C:\\foo.exe".into(),
            ..Default::default()
        });
        assert!(!verify_integrity(&config));
    }

    #[test]
    fn test_compute_integrity_deterministic() {
        let mut config = AppConfig::default();
        config.data.items.push(Item {
            id: "item_x".into(),
            name: "Foo".into(),
            target: "C:\\foo.exe".into(),
            ..Default::default()
        });
        let a = compute_integrity(&config);
        let b = compute_integrity(&config);
        assert_eq!(a, b);
        assert!(a.starts_with("hmac:"));
    }

    #[test]
    fn test_legacy_fnv_integrity_rejected() {
        // 旧版 `fnv1a64:` 格式配置在 HMAC 校验下必须失败，以触发隔离 + 确认弹窗（迁移路径）。
        // 同时证明"仅重算公开哈希（旧 FNV 攻击）"无法伪造 HMAC——产出值仍以 `fnv1a64:` 开头，
        // 与 `hmac:` 前缀不匹配，校验必失败。
        let mut config = AppConfig::default();
        config.extra_config.insert("generated_by".to_string(), "gorun-rs".to_string());
        config.extra_config.insert("config_integrity".to_string(), "fnv1a64:0123456789abcdef".to_string());
        config.data.items.push(Item {
            id: "item_x".into(),
            name: "Foo".into(),
            target: "C:\\foo.exe".into(),
            ..Default::default()
        });
        assert!(!verify_integrity(&config));
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let storage = Storage::new("/tmp/nonexistent_gorun_config_12345.json".to_string());
        assert!(storage.load().is_none());
    }

    #[test]
    fn test_load_cpp_string_config_format() {
        let path = std::env::temp_dir().join("gorun_cpp_format_test.json");
        let path_str = path.to_str().unwrap().to_string();
        let json = r#"{
            "version": "1.0.0",
            "categories": [{"id":"cat_1","name":"Test","sortOrder":0,"viewType":0,"iconSize":48}],
            "items": [{"id":"item_1","name":"Note","target":"C:\\Windows\\notepad.exe","arguments":"","workingDir":"","iconPath":"","iconIndex":0,"runAsAdmin":false,"runCount":0,"keywords":"","remark":"","categoryId":"cat_1","sortOrder":0}],
            "config": {"globalHotkey":"Ctrl+Alt+M","windowX":"100","windowY":"100","windowWidth":"800","windowHeight":"600","language":"zh-CN","theme":"dark"}
        }"#;
        fs::write(&path, json).unwrap();
        let storage = Storage::new(path_str.clone());
        let loaded = storage.load().expect("should load C++ string config");
        assert_eq!(loaded.config.window_x, 100);
        assert_eq!(loaded.config.window_width, 800);
        assert_eq!(
            loaded.extra_config.get("language").map(|s| s.as_str()),
            Some("zh-CN")
        );
        assert_eq!(
            loaded.extra_config.get("theme").map(|s| s.as_str()),
            Some("dark")
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_writes_string_config_and_preserves_extra() {
        let path = std::env::temp_dir().join("gorun_rs_write_test.json");
        let path_str = path.to_str().unwrap().to_string();
        let mut config = AppConfig::default();
        config.version = "1.0.0".to_string();
        config
            .extra_config
            .insert("language".to_string(), "en-US".to_string());
        config
            .extra_config
            .insert("windowWidth".to_string(), "999".to_string());
        let storage = Storage::new(path_str.clone());
        assert!(storage.save(&config));
        let content = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["config"]["windowWidth"], "800");
        assert_eq!(v["config"]["language"], "en-US");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_cached_icon_roundtrip() {
        // P0：自动提取的图标缓存路径必须落盘（cachedIcon），否则重启后图标丢失
        let path = std::env::temp_dir().join(format!(
            "gorun_cached_icon_{}.json",
            std::process::id()
        ));
        let path_str = path.to_str().unwrap().to_string();
        let mut config = AppConfig::default();
        config.data.items.push(Item {
            id: "item_a".into(),
            name: "A".into(),
            target: "C:\\a.exe".into(),
            cached_icon: "C:\\GoRun\\icons\\deadbeef.png".into(),
            ..Default::default()
        });

        let storage = Storage::new(path_str.clone());
        assert!(storage.save(&config));

        let content = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(
            v["items"][0]["cachedIcon"], "C:\\GoRun\\icons\\deadbeef.png",
            "cachedIcon 应写入 JSON"
        );

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.data.items[0].cached_icon, "C:\\GoRun\\icons\\deadbeef.png");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_missing_cached_icon_defaults_empty() {
        // 旧配置（无 cachedIcon）应能加载，字段回落为空串
        let path = std::env::temp_dir().join(format!(
            "gorun_legacy_no_cached_{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{"version":"1.0.0","categories":[],"items":[{"id":"i","name":"n","target":"t"}],"config":{}}"#,
        )
        .unwrap();
        let loaded = Storage::new(path.to_str().unwrap().to_string()).load().unwrap();
        assert!(loaded.data.items[0].cached_icon.is_empty());
        let _ = fs::remove_file(&path);
    }
}
