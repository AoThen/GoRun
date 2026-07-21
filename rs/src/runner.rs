#![allow(dead_code)]

use crate::model::{Item, RunError, RunResult};

pub fn run(item: &Item) -> RunResult {
    RunResult {
        success: false,
        error: RunError::Unknown,
        error_message: format!("Not implemented yet: {}", item.target),
    }
}

pub fn is_url(target: &str) -> bool {
    let t = target.to_lowercase();
    t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("ftp://")
        || t.starts_with("ftps://")
        || t.starts_with("steam://")
        || t.starts_with("mailto:")
}
