pub mod config;
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

include!(env!("SLINT_INCLUDE_GENERATED"));
include!(env!("SLINT_INCLUDE_GENERATED_EDIT_DIALOG"));
