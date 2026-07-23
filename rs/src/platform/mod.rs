#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

mod dragdrop;

pub enum DropMessage {
    Drop(Vec<PathBuf>),
}

pub struct DropHandler {
    pub rx: Receiver<DropMessage>,
    _sender_ptr: *const std::sync::mpsc::Sender<DropMessage>,
}

unsafe impl Send for DropHandler {}

impl DropHandler {
    pub fn new(hwnd: isize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_ptr = Box::into_raw(Box::new(tx));
        dragdrop::setup_dragdrop(hwnd, tx_ptr as usize);
        DropHandler {
            rx,
            _sender_ptr: tx_ptr,
        }
    }
}

// 注意：故意不实现 Drop，让 Sender 存活到程序退出
// 这是安全的，因为 Sender 很小且窗口子类化需要它

pub fn setup_dragdrop(hwnd: isize) -> DropHandler {
    DropHandler::new(hwnd)
}
