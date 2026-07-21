#![allow(dead_code)]

use std::sync::mpsc;

pub enum TrayMessage {
    Show,
    Hide,
    Quit,
}

pub struct TrayIcon {
    rx: mpsc::Receiver<TrayMessage>,
}

impl TrayIcon {
    pub fn new() -> (Self, mpsc::Sender<TrayMessage>) {
        let (tx, rx) = mpsc::channel();
        (TrayIcon { rx }, tx)
    }

    pub fn try_recv(&self) -> Option<TrayMessage> {
        self.rx.try_recv().ok()
    }
}
