//! GoRun 库根：全部实现从此处导出，bin（`main.rs`）只做转导出与 `main`。

pub mod app_state;
pub mod clipboard;
pub mod config;
pub mod drop_handler;
pub mod fileops;
pub mod geometry;
pub mod handlers;
pub mod hotkey;
pub mod icon_cache;
pub mod item_manager;
pub mod localization;
pub mod logger;
pub mod model;
pub mod platform;
pub mod runner;
pub mod storage;
pub mod tray;
pub mod tray_ui;
pub mod ui_sync;
pub mod util;

include!(env!("SLINT_INCLUDE_GENERATED"));
include!(env!("SLINT_INCLUDE_GENERATED_EDIT_DIALOG"));
