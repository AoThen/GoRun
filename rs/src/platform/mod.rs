#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

mod dragdrop;

pub enum DropMessage {
    Enter,
    Over(f64, f64),
    Leave,
    Drop(Vec<PathBuf>),
}

pub struct DropHandler {
    pub rx: Receiver<DropMessage>,
    _sender_ptr: *const std::sync::mpsc::Sender<DropMessage>,
}

impl DropHandler {
    pub fn new(hwnd: isize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_ptr = Arc::into_raw(Arc::new(tx));
        unsafe {
            dragdrop::setup_dragdrop(hwnd, tx_ptr as usize);
        }
        DropHandler {
            rx,
            _sender_ptr: tx_ptr,
        }
    }
}

impl Drop for DropHandler {
    fn drop(&mut self) {
        unsafe {
            let _ = Arc::from_raw(self._sender_ptr as *mut std::sync::mpsc::Sender<DropMessage>);
        }
    }
}

pub fn setup_dragdrop(hwnd: isize) -> DropHandler {
    DropHandler::new(hwnd)
}
