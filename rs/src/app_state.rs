//! 应用状态容器（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::icon_cache::IconCache;
use crate::item_manager::ItemManager;
use crate::localization::Localization;
use crate::platform::DropHandler;
use crate::storage::Storage;
use crate::EditDialog;

/// 高频非关键写（runCount 等软保存）去抖窗口（M-6）：标记后 2s 内的多次运行
/// 合并为一次落盘（与几何去抖同策略）；退出路径 force 刷盘兜底。
pub const SOFT_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(2000);

/// 异步启动结果（M-5/M-4）：工作线程产出并推入 `pending_run_results` 队列，
/// UI 线程房管 Timer 经 `ui_sync::drain_run_results` 消费——成功回写运行计数，
/// 失败在 UI 线程用 loc 组装本地化文案提示（loc 非 Send，不能在工作线程组装）。
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub item_id: String,
    pub item_name: String,
    pub success: bool,
    pub error: crate::model::RunError,
}

pub struct AppState {
    pub manager: ItemManager,
    pub storage: Storage,
    pub icon_cache: IconCache,
    pub current_category_id: String,
    pub search_query: String,
    /// 当前打开的编辑对话框（新建/编辑共用）；Rc 持有供测试/外部克隆句柄
    /// （slint 组件不实现 Clone，经 Rc 共享引用传递）
    pub edit_dialog: Option<Rc<EditDialog>>,
    pub loc: Rc<RefCell<Localization>>,
    /// 保存失败标志：置位后由周期 Timer 一次性写状态栏提示，避免静默丢数据。
    pub save_fail_notified: bool,
    /// 后台提取出的图标缓存路径（item_id -> png 路径），由 Timer 回写模型并落盘（P0）。
    pub pending_icon_paths:
        std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    /// P1-2 提取去重：本会话已入过异步提取队列的 item_id（防止提取失败/无源时
    /// 每次刷新重复 spawn 提取线程）。提取成功回写后清除（persist_cached_icons），
    /// 编辑条目后清除（新键值得再试一次）。仅 UI 线程访问，用 Mutex 满足
    /// 不可变借用下的内部可变性（refresh_items_ui 收 &AppState）。
    pub icon_extract_attempted: std::sync::Mutex<std::collections::HashSet<String>>,
    /// OLE/子类化拖放处理器：必须随应用存活，Drop 时移除子类化并释放 refdata。
    pub drop_handler: Option<DropHandler>,
    /// 窗口几何已变化但尚未落盘（P1-5 去抖）。
    pub geometry_dirty: bool,
    /// 最近一次几何变化时刻，用于判断去抖窗口是否已过。
    pub last_geometry_change: Option<std::time::Instant>,
    /// 软保存（runCount 等高频非关键写）已标脏但未落盘（M-6 去抖）。
    pub soft_dirty: bool,
    /// 最近一次软保存标脏时刻，用于判断去抖窗口是否已过。
    pub last_soft_change: Option<std::time::Instant>,
    /// 异步启动结果队列（M-5/M-4）：工作线程推入，UI 线程房管 Timer 消费。
    pub pending_run_results:
        std::sync::Arc<std::sync::Mutex<Vec<RunOutcome>>>,
    /// 条目级全局热键管理器（F8/4.1）：多热键注册/分发。
    pub item_hotkey_mgr: Rc<RefCell<crate::hotkey::ItemHotkeyManager>>,
    /// 条目热键触发通道发送端（item_id），由消费线程收。
    pub item_hotkey_tx: std::sync::mpsc::Sender<String>,
    /// P3/P5 后台低驻留：true 时窗口隐藏暂停 500ms Timer 非必要工作（可运行时经托盘切换）。
    pub low_resident: Rc<std::cell::Cell<bool>>,
    /// P3-11 自动隐藏（F7）运行时标志：拖放消费线程在事件时刻读取
    /// （设置页开关同时写 extra_config 与本标志，即时生效无需重启）。
    pub auto_hide_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AppState {
    /// 几何变化是否已到可落盘时机（force=true 表示强制立即写）。
    pub fn geometry_should_flush(&self, force: bool) -> bool {
        if !self.geometry_dirty {
            return false;
        }
        if force {
            return true;
        }
        self.last_geometry_change
            .map(|t| t.elapsed() >= crate::geometry::GEOMETRY_SAVE_DEBOUNCE)
            .unwrap_or(true)
    }

    /// 若满足时机则落盘几何；返回是否已落盘成功。
    pub fn flush_geometry_if_due(&mut self, force: bool) -> bool {
        if !self.geometry_should_flush(force) {
            return true;
        }
        self.flush_geometry()
    }

    /// 立即落盘几何（几何单独走 storage.save，不轮转备份）。
    ///
    /// 注意：`storage.save` 写入的是**完整配置**（含数据变更），因此这里
    /// **不能** `reset_modified()` —— 否则数据变更的"已修改"标记会被几何写盘抹掉，
    /// 导致本该走 `save_with_backup`（轮转备份）的数据变更被当成普通写丢弃备份。
    /// 仅清除几何自身的 dirty 标记。
    pub fn flush_geometry(&mut self) -> bool {
        if !self.geometry_dirty {
            return true;
        }
        if self.storage.save(self.manager.config()) {
            self.geometry_dirty = false;
            self.last_geometry_change = None;
            true
        } else {
            log::warn!("Failed to save window geometry (will retry)");
            false
        }
    }

    /// 保存修改。返回是否成功（无修改视为成功）。
    /// 失败时置位 `save_fail_notified`，供 UI 层轮询提示。
    pub fn save_if_modified(&mut self) -> bool {
        if self.manager.is_modified() {
            if self.storage.save_with_backup(self.manager.config()) {
                self.manager.reset_modified();
                self.save_fail_notified = false;
                true
            } else {
                log::error!("Failed to save config after modification");
                self.save_fail_notified = true;
                false
            }
        } else {
            true
        }
    }

    /// 高频非关键写（使用计数）标脏（M-6）：仅记录 dirty + 时刻，实际落盘由
    /// `flush_soft_if_due` 在去抖窗口（`SOFT_SAVE_DEBOUNCE`）到期或退出 force 时执行。
    /// 旧实现每次运行都全量序列化+fsync+rename，连续点击/"运行全部"时 I/O 放大明显。
    ///
    /// 与几何 flush 一致，**不**检查/清除 `modified`：结构性变更的"已修改"标记
    /// 仍留给 `save_if_modified` 触发备份轮转；软保存写入的是完整配置，未保存的
    /// 结构性变更会顺带落盘（不丢数据，仅推迟轮转）。
    pub fn save_soft(&mut self) -> bool {
        self.soft_dirty = true;
        self.last_soft_change = Some(std::time::Instant::now());
        true
    }

    /// 软保存是否到期可落盘（force=true 表示强制立即写，退出路径兜底）。
    pub fn soft_should_flush(&self, force: bool) -> bool {
        if !self.soft_dirty {
            return false;
        }
        if force {
            return true;
        }
        self.last_soft_change
            .map(|t| t.elapsed() >= SOFT_SAVE_DEBOUNCE)
            .unwrap_or(true)
    }

    /// 若满足时机则落盘软保存（使用计数等高频非关键写）；返回是否成功/无事可做。
    /// 失败置位 `save_fail_notified`（磁盘异常往往同时威胁结构性变更），供 UI 提示。
    pub fn flush_soft_if_due(&mut self, force: bool) -> bool {
        if !self.soft_should_flush(force) {
            return true;
        }
        if self.storage.save(self.manager.config()) {
            self.soft_dirty = false;
            self.last_soft_change = None;
            true
        } else {
            log::error!("Failed to save config (soft save)");
            self.save_fail_notified = true;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppConfig;

    fn make_state(path: &str) -> AppState {
        let (ihk_tx, _ihk_rx) = std::sync::mpsc::channel::<String>();
        AppState {
            manager: ItemManager::new(AppConfig::default()),
            storage: Storage::new(path.to_string()),
            icon_cache: IconCache::new(),
            current_category_id: String::new(),
            search_query: String::new(),
            edit_dialog: None,
            loc: Rc::new(RefCell::new(Localization::new("zh-CN"))),
            save_fail_notified: false,
            pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            icon_extract_attempted: std::sync::Mutex::new(std::collections::HashSet::new()),
            drop_handler: None,
            geometry_dirty: false,
            last_geometry_change: None,
            soft_dirty: false,
            last_soft_change: None,
            pending_run_results: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
            item_hotkey_tx: ihk_tx,
            low_resident: Rc::new(std::cell::Cell::new(false)),
            auto_hide_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    #[test]
    fn test_save_if_modified_unmodified_returns_true() {
        let ok = std::env::temp_dir()
            .join(format!("gorun_appstate_unmod_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&ok);
        assert!(!state.manager.is_modified());
        assert!(state.save_if_modified());
        assert!(!state.save_fail_notified);
        let _ = std::fs::remove_file(&ok);
    }

    #[test]
    fn test_save_if_modified_success_clears_flag() {
        let ok = std::env::temp_dir()
            .join(format!("gorun_appstate_ok_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&ok);
        state.save_fail_notified = true;
        let _ = state.manager.config_mut();
        assert!(state.manager.is_modified());
        assert!(state.save_if_modified());
        assert!(!state.manager.is_modified());
        assert!(!state.save_fail_notified);
        let _ = std::fs::remove_file(&ok);
    }

    #[test]
    fn test_save_if_modified_failure_sets_flag() {
        // 父路径为文件 → save 写盘失败，验证失败路径置位 save_fail_notified 且保留 modified
        let blocker = std::env::temp_dir().join(format!("gorun_block_{}.txt", std::process::id()));
        std::fs::write(&blocker, b"x").unwrap();
        let fail_path = format!("{}/config.json", blocker.to_str().unwrap());
        let mut state = make_state(&fail_path);
        let _ = state.manager.config_mut();
        assert!(state.manager.is_modified());
        assert!(!state.save_if_modified());
        assert!(state.manager.is_modified());
        assert!(state.save_fail_notified);
        let _ = std::fs::remove_file(&blocker);
    }

    #[test]
    fn test_save_soft_saves_without_clearing_modified() {
        let ok = std::env::temp_dir()
            .join(format!("gorun_appstate_soft_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&ok);
        let _ = state.manager.config_mut();
        assert!(state.manager.is_modified());
        // P1-1/M-6：save_soft 标脏（去抖），force flush 落盘但不消费数据变更标记
        assert!(state.save_soft());
        assert!(state.flush_soft_if_due(true));
        assert!(std::path::Path::new(&ok).exists());
        assert!(state.manager.is_modified(), "save_soft 不得清除数据变更标记");
        assert!(!state.soft_dirty, "flush 后软保存脏标记应清除");
        assert!(!state.save_fail_notified, "save_soft 成功不应置位失败提示");
        // 标记仍在：后续 save_if_modified 照常轮转落盘并复位
        assert!(state.save_if_modified());
        assert!(!state.manager.is_modified());
        let _ = std::fs::remove_file(&ok);
    }

    #[test]
    fn test_save_soft_failure_sets_flag() {
        let blocker = std::env::temp_dir().join(format!("gorun_block_soft_{}.txt", std::process::id()));
        std::fs::write(&blocker, b"x").unwrap();
        let fail_path = format!("{}/config.json", blocker.to_str().unwrap());
        let mut state = make_state(&fail_path);
        assert!(state.save_soft());
        assert!(!state.flush_soft_if_due(true));
        assert!(state.save_fail_notified, "软保存失败应置位提示（磁盘异常威胁结构性变更）");
        let _ = std::fs::remove_file(&blocker);
    }

    #[test]
    fn test_save_soft_debounce_window() {
        let path = std::env::temp_dir()
            .join(format!("gorun_soft_debounce_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let _ = std::fs::remove_file(&path);
        let mut state = make_state(&path);
        assert!(state.save_soft());
        // 去抖窗口内：非强制不落盘
        assert!(!state.soft_should_flush(false));
        assert!(state.flush_soft_if_due(false));
        assert!(
            !std::path::Path::new(&path).exists(),
            "去抖窗口内不应写盘"
        );
        // 窗口已过（模拟 3s 前标脏）：非强制即落盘并清脏标记
        state.last_soft_change =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(3000));
        assert!(state.soft_should_flush(false));
        assert!(state.flush_soft_if_due(false));
        assert!(std::path::Path::new(&path).exists());
        assert!(!state.soft_dirty);
        assert!(state.last_soft_change.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_geometry_debounce_not_dirty_by_default() {
        let mut state = make_state("/tmp/gorun_geom_default.json");
        assert!(!state.geometry_dirty);
        assert!(!state.geometry_should_flush(false));
        // 未 dirty 时强制落盘也视为成功（无事可做）
        assert!(state.flush_geometry_if_due(true));
    }

    #[test]
    fn test_geometry_debounce_waits_window_then_flushes() {
        let path = std::env::temp_dir()
            .join(format!("gorun_geom_{}.json", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let mut state = make_state(&path);
        // 模拟"刚记录一次几何变化"
        let _ = state.manager.config_mut();
        state.geometry_dirty = true;
        state.last_geometry_change = Some(std::time::Instant::now());

        // 刚变化：去抖窗口未到，非强制不落盘
        assert!(!state.geometry_should_flush(false));
        assert!(state.flush_geometry_if_due(false));

        // 强制（退出路径）立即落盘
        assert!(state.flush_geometry_if_due(true));
        assert!(!state.geometry_dirty);
        assert!(std::path::Path::new(&path).exists());
        // 几何写盘不得清除数据变更标记（否则数据变更会绕过备份轮转）
        assert!(state.manager.is_modified());
        let _ = std::fs::remove_file(&path);
    }
}
