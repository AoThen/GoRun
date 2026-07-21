#![allow(dead_code)]

use std::path::PathBuf;

mod dragdrop;

pub enum DropMessage {
    Enter,
    Over(f64, f64),
    Leave,
    Drop(Vec<PathBuf>),
}

pub fn setup_dragdrop(hwnd: isize) -> std::sync::mpsc::Receiver<DropMessage> {
    let (tx, rx) = std::sync::mpsc::channel();
    let tx = Box::leak(Box::new(tx));
    dragdrop::setup_dragdrop(hwnd, tx);
    rx
}
