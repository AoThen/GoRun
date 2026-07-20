#[cfg(windows)]
pub mod dragdrop;

use std::path::PathBuf;

pub enum DropMessage {
    Enter,
    Over(f64, f64),
    Leave,
    Drop(Vec<PathBuf>),
}

#[cfg(windows)]
pub fn setup_dragdrop(hwnd: isize) -> std::sync::mpsc::Receiver<DropMessage> {
    self::dragdrop::setup(hwnd)
}

#[cfg(not(windows))]
pub fn setup_dragdrop(_hwnd: isize) -> std::sync::mpsc::Receiver<DropMessage> {
    let (_tx, rx) = std::sync::mpsc::channel();
    rx
}
