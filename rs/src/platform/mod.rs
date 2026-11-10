#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

mod dragdrop;

pub enum DropMessage {
    Drop(Vec<PathBuf>),
}

pub struct DropHandler {
    pub rx: Receiver<DropMessage>,
    _hwnd: isize,
    // P2-6：子类化 refdata 指向 Arc 内值稳定指针（Arc::into_raw 泄漏，refcount 保持 1），
    // Drop 时经 Arc::from_raw 重建并释放
    _leaked_sender: *const std::sync::mpsc::Sender<DropMessage>,
}

// P2-6：裸指针指向稳定的 Arc 堆对象；子类化回调（主线程消息循环）与
// Drop（主线程）顺序访问，无并发竞争，故 Send 标记安全。
unsafe impl Send for DropHandler {}

impl DropHandler {
    pub fn new(hwnd: isize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let arc = Arc::new(tx);
        let leaked = Arc::into_raw(arc);
        dragdrop::setup_dragdrop(hwnd, leaked as usize);
        DropHandler {
            rx,
            _hwnd: hwnd,
            _leaked_sender: leaked,
        }
    }
}

impl Drop for DropHandler {
    // P2-6：RAII 生命周期——先移除子类化（防止悬垂 refdata），再释放 Arc 引用
    fn drop(&mut self) {
        dragdrop::remove_dragdrop(self._hwnd);
        unsafe {
            drop(Arc::from_raw(self._leaked_sender));
        }
    }
}

pub fn setup_dragdrop(hwnd: isize) -> DropHandler {
    DropHandler::new(hwnd)
}
