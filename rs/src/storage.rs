use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{AppConfig, AppData, Category, Config, Item, ViewType};

/// 临时文件命名计数器：与进程 id 组合成唯一后缀，避免同进程多次保存
/// （或未来重入/多线程）相互覆盖 `.tmp.<pid>` 文件。
static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
    "Alt+Z".to_string()
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
    #[serde(rename = "viewType", default = "default_view_type")]
    view_type: i32,
    #[serde(rename = "iconSize", default = "default_icon_size")]
    icon_size: i32,
}

/// P3-9：JSON 缺失 viewType 时默认列表视图（1），与 Rust `ViewType::default()`
/// 及"默认列表模式"决策一致（此前 serde 无参 default 取 i32::default()=0=图标视图）。
fn default_view_type() -> i32 {
    1
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

pub struct Storage {
    path: String,
    /// load 时是否发生了旧默认热键迁移（L-5）：main 据此顺手 save 一次落盘，
    /// 避免未改配置时每次启动都重复迁移（幂等但属无谓工作）。
    migrated_hotkey: std::cell::Cell<bool>,
}

impl Storage {
    pub fn new(path: String) -> Self {
        Storage {
            path,
            migrated_hotkey: std::cell::Cell::new(false),
        }
    }

    /// L-5：load 时是否迁移了旧默认热键（供 main 决定是否顺手落盘一次，
    /// 避免每次启动重复迁移）。
    pub fn migrated_hotkey(&self) -> bool {
        self.migrated_hotkey.get()
    }

    pub fn load(&self) -> Option<AppConfig> {
        if !Path::new(&self.path).exists() {
            log::warn!("Config file not found: {}", self.path);
            return None;
        }
        // L-4：文件大小上限——正常配置远小于此；外部误写/恶意超大 JSON 直接按
        // 损坏处理（走隔离/备份恢复路径），避免拖慢启动甚至 OOM。
        const MAX_CONFIG_SIZE: u64 = 64 * 1024 * 1024;
        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() > MAX_CONFIG_SIZE {
                log::error!(
                    "Config file too large ({} bytes), treating as corrupt: {}",
                    meta.len(),
                    self.path
                );
                return None;
            }
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
            // 历史默认迁移链：Ctrl+Alt+M（全局注册易被其它程序占用）→ Ctrl+M → Ctrl+/
            // → Alt+Z（更冷门、冲突概率更低）。加载时把仍为任一旧默认的值逐级升级到
            // 当前默认，用户自定义的其它键不受影响；当前默认值不在迁移源集合中，幂等。
            global_hotkey: match raw_config.global_hotkey.as_str() {
                "Ctrl+Alt+M" | "Ctrl+M" | "Ctrl+/" => {
                    self.migrated_hotkey.set(true);
                    "Alt+Z".to_string()
                }
                other => other.to_string(),
            },
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
        // (category_id, sort_order)），一次性修复旧版生成配置中"数组序≠sortOrder 序"
        // 的历史数据，保证迁移后展示顺序一致。
        categories.sort_by_key(|c| c.sort_order);
        // L-9：sort_by 借用比较替代 sort_by_key 的逐次 String 克隆
        items.sort_by(|a, b| {
            a.category_id
                .cmp(&b.category_id)
                .then(a.sort_order.cmp(&b.sort_order))
        });

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

    /// 把内存配置序列化为导出/落盘用的 RawFile（重编号 sortOrder）。
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
        // 此剔除列表必须与 RawConfig 显式字段集合保持同步，否则 serde flatten 会输出重复键。
        // 另含历史完整性机制残留键（config_integrity/generated_by）：旧配置携带的
        // 这些键不再回写落盘，随后续保存自然清除（内存中保留无害）。
        for (k, v) in &app_config.extra_config {
            if matches!(
                k.as_str(),
                "globalHotkey"
                    | "windowX"
                    | "windowY"
                    | "windowWidth"
                    | "windowHeight"
                    | "config_integrity"
                    | "generated_by"
            ) {
                continue;
            }
            extra.insert(k.clone(), serde_json::Value::String(v.clone()));
        }

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
        // 保证写回的 JSON 数组序 ≡ sortOrder 序。
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

        let tmp_path = format!(
            "{}.tmp.{}.{}",
            self.path,
            std::process::id(),
            TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
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
        if Self::rename_with_retry(Path::new(&tmp_path), Path::new(&self.path)).is_err() {
            let _ = fs::remove_file(&tmp_path);
            log::error!("Failed to rename temp to config: {}", self.path);
            return false;
        }
        log::info!("Config saved to: {}", self.path);
        true
    }

    /// rename 原子替换，瞬时冲突重试（M-3）：Windows 上目标被外部进程短暂占用
    /// （杀毒扫描/云同步/资源管理器）时返回 ERROR_SHARING_VIOLATION(32)/
    /// ACCESS_DENIED(5)，单次失败即丢一次落盘时机；对这类冲突做有限次退避重试。
    fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
        const MAX_ATTEMPTS: u32 = 3;
        let mut attempt = 1;
        loop {
            match fs::rename(from, to) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let retryable = e.kind() == std::io::ErrorKind::PermissionDenied
                        || (cfg!(windows) && matches!(e.raw_os_error(), Some(32) | Some(5)));
                    if !retryable || attempt >= MAX_ATTEMPTS {
                        return Err(e);
                    }
                    log::warn!(
                        "Rename attempt {} failed ({}), retrying: {} -> {}",
                        attempt,
                        e,
                        from.display(),
                        to.display()
                    );
                    std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
                    attempt += 1;
                }
            }
        }
    }

    /// 启动时清理原子写残留的临时文件（M-3）：进程在 tmp 写入后、rename 前
    /// 崩溃会残留 `config.json.tmp.<pid>.<seq>`；仅清理修改时间早于本进程
    /// 启动时刻的文件，避免误删并发实例正在写的 tmp。返回清理数量。
    pub fn cleanup_stale_tmp(config_path: &Path) -> usize {
        let file_name = match config_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => return 0,
        };
        let dir = match config_path.parent() {
            Some(p) => p,
            None => return 0,
        };
        let prefix = format!("{}.tmp.", file_name);
        let started = std::time::SystemTime::now();
        let mut removed = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let name = match fname.to_str() {
                    Some(n) => n,
                    None => continue,
                };
                if !name.starts_with(&prefix) {
                    continue;
                }
                // 并发实例正在写的 tmp 修改时间晚于本进程启动，不会被误删
                let stale = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .map(|t| t < started)
                    .unwrap_or(false);
                if stale && fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            log::info!("Cleaned {} stale config tmp file(s) in {:?}", removed, dir);
        }
        removed
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

        // 精简为单份备份：仅保留 config.json.bak1（覆盖式）；
        // 历史遗留的 bak2/bak3 顺手清理，主目录不再堆积多个备份文件。
        for legacy in [".bak2", ".bak3"] {
            let legacy_path = format!("{}{}", self.path, legacy);
            if Path::new(&legacy_path).exists() && fs::remove_file(&legacy_path).is_err() {
                log::error!("Failed to delete legacy backup: {}", legacy_path);
            }
        }

        let bak1 = format!("{}.bak1", self.path);
        if fs::copy(&self.path, &bak1).is_err() {
            log::error!("Failed to create backup .bak1");
            return false;
        }

        log::info!("Backup rotation completed");
        true
    }

    pub fn load_from_backup(&self) -> Option<AppConfig> {
        let backup_path = format!("{}{}", self.path, ".bak1");
        if Path::new(&backup_path).exists() {
            log::info!("Trying to load from backup: {}", backup_path);
            let backup_storage = Storage::new(backup_path.clone());
            if let Some(config) = backup_storage.load() {
                log::info!("Successfully loaded from backup: {}", backup_path);
                if self.save(&config) {
                    log::info!("Restored backup to main config file");
                }
                return Some(config);
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

/// P2-05 损坏配置隔离（留证）：损坏且无可用备份时把坏文件改名为
/// `<原路径>.corrupt.<ts>`，避免后续保存覆盖坏数据/污染备份链。
/// 返回隔离后的路径；重命名失败（如文件已被外部删除）返回 None，
/// 调用方继续以内存默认配置运行。
pub fn quarantine_corrupt_file(config_path: &std::path::Path) -> Option<String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let corrupt_path = format!("{}.corrupt.{}", config_path.display(), ts);
    if std::fs::rename(config_path, &corrupt_path).is_ok() {
        log::warn!("Corrupt config quarantined to: {}", corrupt_path);
        Some(corrupt_path)
    } else {
        log::error!(
            "Failed to quarantine corrupt config {}; will run with default in memory",
            config_path.display()
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::generate_id;

    #[test]
    fn test_save_and_load_roundtrip() {
        let mut config = AppConfig {
            version: "1.0.0".to_string(),
            ..Default::default()
        };
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
        config.config.global_hotkey = "Ctrl+Shift+P".to_string();

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
        assert_eq!(loaded.config.global_hotkey, "Ctrl+Shift+P");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_disordered_sort_order_survives_roundtrip() {
        // 回归：迁移数据常见"数组序≠sortOrder 序"。save 按落盘形态重编号后
        // 往返稳定（幂等），展示顺序与数组序一致。
        let path = std::env::temp_dir()
            .join(format!("gorun_disorder_test_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut config = AppConfig::default();
        // 分类 sortOrder 故意逆序（数组序 0,1,2 vs sortOrder 2,1,0）
        for (i, name) in ["C", "B", "A"].iter().enumerate() {
            config.data.categories.push(Category {
                id: format!("cat_{}", name),
                name: name.to_string(),
                sort_order: (2 - i) as i32,
                view_type: ViewType::Icon,
                icon_size: 48,
            });
        }
        // 项目 sortOrder 在分类内也乱序
        for (i, name) in ["i3", "i1", "i2"].iter().enumerate() {
            config.data.items.push(Item {
                id: format!("item_{}", name),
                name: name.to_string(),
                target: format!("C:\\{}.exe", name),
                category_id: "cat_A".into(),
                sort_order: (2 - i) as i32,
                ..Default::default()
            });
        }

        let storage = Storage::new(path.clone());
        assert!(storage.save(&config));
        let loaded = storage.load().expect("disordered config should roundtrip");
        // 落盘 JSON 的 sortOrder 已按数组序重编号
        let content = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["categories"][0]["sortOrder"], 0);
        assert_eq!(v["categories"][1]["sortOrder"], 1);
        assert_eq!(v["items"][0]["sortOrder"], 0);
        // 二次 save→load 仍稳定（幂等）
        assert!(storage.save(&loaded));
        let reloaded = storage.load().unwrap();
        assert_eq!(reloaded.data.items.len(), 3);
        assert_eq!(reloaded.data.categories.len(), 3);
        let _ = fs::remove_file(&path);
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
        let loaded = storage.load().expect("should load string config");
        // 输入为最老默认 Ctrl+Alt+M，加载时应逐级升级到当前默认 Alt+Z（迁移链锁定）
        assert_eq!(loaded.config.global_hotkey, "Alt+Z");
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
        let mut config = AppConfig {
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        config
            .extra_config
            .insert("custom_pref".to_string(), "keep-me".to_string());
        config
            .extra_config
            .insert("windowWidth".to_string(), "999".to_string());
        let storage = Storage::new(path_str.clone());
        assert!(storage.save(&config));
        let content = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["config"]["windowWidth"], "800");
        assert_eq!(v["config"]["custom_pref"], "keep-me");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_cached_icon_roundtrip() {
        // P0：自动提取的图标缓存路径必须落盘（cachedIcon），否则重启后图标丢失
        let path =
            std::env::temp_dir().join(format!("gorun_cached_icon_{}.json", std::process::id()));
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
        assert_eq!(
            loaded.data.items[0].cached_icon,
            "C:\\GoRun\\icons\\deadbeef.png"
        );
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
        let loaded = Storage::new(path.to_str().unwrap().to_string())
            .load()
            .unwrap();
        assert!(loaded.data.items[0].cached_icon.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_missing_view_type_defaults_list() {
        // P3-9：旧配置缺省 viewType 应按"默认列表模式"解析为 List（=1）
        let path = std::env::temp_dir().join(format!(
            "gorun_viewtype_default_{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{"version":"1.0.0","categories":[{"id":"c1","name":"C"}],"items":[],"config":{}}"#,
        )
        .unwrap();
        let loaded = Storage::new(path.to_str().unwrap().to_string())
            .load()
            .unwrap();
        assert_eq!(
            loaded.data.categories[0].view_type,
            ViewType::List,
            "缺失 viewType 应解析为 List"
        );
        // 显式 0 仍应解析为 Icon（存量配置保持原视图）
        fs::write(
            &path,
            r#"{"version":"1.0.0","categories":[{"id":"c1","name":"C","viewType":0}],"items":[],"config":{}}"#,
        )
        .unwrap();
        let loaded = Storage::new(path.to_str().unwrap().to_string())
            .load()
            .unwrap();
        assert_eq!(loaded.data.categories[0].view_type, ViewType::Icon);
        let _ = fs::remove_file(&path);
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;
    use crate::model::{generate_id, Category, Item, ViewType};

    fn sample() -> AppConfig {
        let mut config = AppConfig::default();
        config.data.categories.push(Category {
            id: generate_id("cat"),
            name: "边度分类".into(),
            sort_order: 0,
            view_type: ViewType::Icon,
            icon_size: 48,
        });
        config.data.items.push(Item {
            id: generate_id("item"),
            name: "emoji🎯\"引号\"'单引'\\反斜\n换行".into(),
            target: "C:\\Program Files\\测试 目录\\app.exe".into(),
            ..Default::default()
        });
        config
    }

    /// export_to：无效目录（父路径是文件）应失败返回 false，不 panic。
    #[test]
    fn test_export_to_invalid_path_fails_gracefully() {
        let dir = std::env::temp_dir().join("gorun_export_edge");
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("blocker_file");
        std::fs::write(&blocker, b"x").unwrap();
        // 父路径是文件 → 创建目录/写文件失败
        let bad = blocker.join("sub").join("out.json");
        let storage = Storage::new(dir.join("cfg.json").to_string_lossy().to_string());
        assert!(!storage.export_to(&bad.to_string_lossy(), &sample()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// export_to → load_from(外部文件) 语义：导出的文件是合法 JSON，可被重新解析。
    #[test]
    fn test_export_then_parse_roundtrip() {
        let dir = std::env::temp_dir().join("gorun_export_ok");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("exported.json");
        let cfg = sample();
        let storage = Storage::new(dir.join("cfg.json").to_string_lossy().to_string());
        assert!(storage.export_to(&out.to_string_lossy(), &cfg));
        let text = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("导出文件应是合法 JSON");
        assert!(
            parsed.get("categories").is_some(),
            "导出格式为 categories 顶层键"
        );
        assert!(parsed.get("items").is_some(), "应含 items 字段");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 特殊字符（emoji/引号/反斜杠/换行/中文）save→load 往返不丢真。
    #[test]
    fn test_special_characters_roundtrip() {
        let dir = std::env::temp_dir().join("gorun_special_chars");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cfg.json");
        let storage = Storage::new(path.to_string_lossy().to_string());
        let cfg = sample();
        assert!(storage.save(&cfg));
        let loaded = storage.load().expect("应能加载");
        assert_eq!(loaded.data.items[0].name, cfg.data.items[0].name);
        assert_eq!(loaded.data.items[0].target, cfg.data.items[0].target);
        assert_eq!(loaded.data.categories[0].name, "边度分类");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// load：文件被截断/损坏（非法 JSON）时返回 None，不 panic。
    #[test]
    fn test_load_corrupt_json_returns_none() {
        let dir = std::env::temp_dir().join("gorun_corrupt_json");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cfg.json");
        std::fs::write(&path, "{ this is not json !!!").unwrap();
        let storage = Storage::new(path.to_string_lossy().to_string());
        assert!(storage.load().is_none(), "损坏 JSON 应返回 None");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_with_backup_creates_bak1() {
        let dir = std::env::temp_dir().join(format!("gorun_backup_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let path_str = path.to_string_lossy().to_string();
        let storage = Storage::new(path_str.clone());

        let config = AppConfig::default();
        // 先建立主配置，再次保存时才轮转备份（首次保存无旧配置可备份）
        assert!(storage.save(&config));
        assert!(storage.save_with_backup(&config));

        let bak1 = format!("{}.bak1", path_str);
        assert!(Path::new(&bak1).exists(), "save_with_backup 应创建 .bak1");
        // 主配置仍可加载
        assert!(storage.load().is_some());
        // 备份本身也应是合法配置
        let bak_storage = Storage::new(bak1.clone());
        assert!(bak_storage.load().is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_from_backup_restores_config() {
        let dir = std::env::temp_dir().join(format!("gorun_backup_restore_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let path_str = path.to_string_lossy().to_string();
        let storage = Storage::new(path_str.clone());

        let config = AppConfig::default();
        // 先建立主配置，再保存以触发备份轮转
        assert!(storage.save(&config));
        assert!(storage.save_with_backup(&config));

        // 模拟主配置损坏：删除主文件，仅剩 .bak1
        std::fs::remove_file(&path).unwrap();
        let restored = storage.load_from_backup();
        assert!(restored.is_some(), "应从 .bak1 恢复配置");
        // 恢复后主配置被重新写入（save 在 load_from_backup 内调用）
        assert!(path.exists(), "恢复后主配置应重建");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_from_backup_no_backup_returns_none() {
        let dir = std::env::temp_dir().join(format!("gorun_backup_none_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let storage = Storage::new(path.to_string_lossy().to_string());

        assert!(
            storage.load_from_backup().is_none(),
            "无备份文件应返回 None"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_with_backup_cleans_legacy_bak_files() {
        let dir = std::env::temp_dir().join(format!("gorun_backup_legacy_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let path_str = path.to_string_lossy().to_string();

        // 预置历史 bak2/bak3 垃圾
        std::fs::write(format!("{}.bak2", path_str), b"old").unwrap();
        std::fs::write(format!("{}.bak3", path_str), b"old").unwrap();

        let storage = Storage::new(path_str.clone());
        let config = AppConfig::default();
        // 先建立主配置，再保存以触发备份轮转与 bak2/bak3 清理
        assert!(storage.save(&config));
        assert!(storage.save_with_backup(&config));

        assert!(
            !Path::new(&format!("{}.bak2", path_str)).exists(),
            "历史 bak2 应被清理"
        );
        assert!(
            !Path::new(&format!("{}.bak3", path_str)).exists(),
            "历史 bak3 应被清理"
        );
        assert!(Path::new(&format!("{}.bak1", path_str)).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cleanup_stale_tmp_removes_only_stale() {
        use std::time::{Duration, SystemTime};
        let dir = std::env::temp_dir().join(format!("gorun_cleanup_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("config.json");

        // 上次运行残留的 tmp（mtime 一小时前）→ 应清理
        let stale = dir.join("config.json.tmp.123");
        std::fs::write(&stale, b"{}").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&stale)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();

        // 并发实例正在写的 tmp（mtime 晚于本次清理启动）→ 保留
        let fresh = dir.join("config.json.tmp.456");
        std::fs::write(&fresh, b"{}").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&fresh)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(3600))
            .unwrap();

        // 其他前缀的 tmp（非本配置文件）→ 保留
        let other = dir.join("other.json.tmp.789");
        std::fs::write(&other, b"{}").unwrap();

        let removed = Storage::cleanup_stale_tmp(&config_path);
        assert_eq!(removed, 1, "只应清理 stale tmp");
        assert!(!stale.exists());
        assert!(fresh.exists(), "并发实例的 tmp 不得误删");
        assert!(other.exists(), "无关前缀不得误删");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_quarantine_corrupt_file_renames_and_returns_path() {
        let dir = std::env::temp_dir().join(format!(
            "gorun_quarantine_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.json");
        std::fs::write(&cfg, b"{not json").unwrap();

        let q = quarantine_corrupt_file(&cfg).expect("隔离应成功");
        assert!(!cfg.exists(), "坏文件应被移出原位");
        let qpath = std::path::Path::new(&q);
        assert!(qpath.exists(), "隔离后文件应存在（留证）");
        let name = qpath.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            name.starts_with("config.json.corrupt."),
            "应带 .corrupt.<ts> 后缀: {name}"
        );
        assert_eq!(
            std::fs::read(qpath).unwrap(),
            b"{not json",
            "隔离内容应保持原样"
        );

        let _ = std::fs::remove_dir_all(&dir);
        // 原文件不存在（已被移走/外部删除）→ 重命名失败，返回 None 不 panic
        assert!(quarantine_corrupt_file(&cfg).is_none());
    }
}
