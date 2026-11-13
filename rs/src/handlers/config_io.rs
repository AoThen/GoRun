//! 配置导出/导入（F8）：Settings 菜单 → 导出配置 / 导入配置。
//! 导出用存储的 RawFile 序列化（与落盘同格式，含完整性值）；
//! 导入整体替换数据与偏好设置，但保留语言/跟随鼠标/自动隐藏等会话偏好。

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::MainWindow;

/// Settings 菜单 → 导出配置：用户选目标文件，写当前配置 JSON。
pub fn export_config(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    let Some(path) = crate::fileops::save_file_dialog(
        "Export Config",
        "Config (*.json)\0*.json\0",
    ) else {
        return;
    };
    let st = state.borrow();
    if st.storage.export_to(&path, st.manager.config()) {
        log::info!("Exported config to {}", path);
        ui.set_status_text(format!("Exported: {}", path).into());
    } else {
        log::error!("Failed to export config to {}", path);
        ui.set_status_text("Export failed".into());
    }
}

/// Settings 菜单 → 导入配置：用户选配置文件，整体替换数据与行为设置，
/// 保留语言/跟随鼠标/自动隐藏等会话偏好；成功后刷新侧栏与条目。
pub fn import_config(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    let Some(path) = crate::fileops::browse_file_dialog(
        "Import Config",
        "Config (*.json)\0*.json\0",
    ) else {
        return;
    };
    let imported = match crate::storage::Storage::new(path.clone()).load() {
        Some(c) => c,
        None => {
            log::error!("Failed to parse import config: {}", path);
            ui.set_status_text("Import failed: invalid config file".into());
            return;
        }
    };
    let new_current = imported
        .data
        .categories
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let mut st = state.borrow_mut();
    {
        let cfg = st.manager.config_mut();
        // 保留会话偏好（语言/跟随鼠标/自动隐藏/信任标记），其余整体替换
        let keep: std::collections::HashMap<String, String> = cfg
            .extra_config
            .iter()
            .filter(|(k, _)| {
                matches!(
                    k.as_str(),
                    "language" | "follow_mouse" | "auto_hide" | "generated_by"
                )
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        cfg.version = imported.version;
        cfg.data = imported.data;
        cfg.config = imported.config;
        cfg.extra_config = keep;
    }
    st.current_category_id = new_current;
    let saved = st.save_if_modified();
    let cur = st.current_category_id.clone();
    drop(st);

    if !saved {
        log::error!("Failed to save imported config");
        ui.set_status_text("Import failed: save error".into());
        return;
    }
    crate::ui_sync::refresh_categories_ui(ui, &state.borrow());
    crate::ui_sync::refresh_items_ui(ui, &state.borrow(), &cur, "");
    ui.set_status_text("Imported config".into());
    log::info!("Imported config from {}", path);
}
