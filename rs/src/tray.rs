use std::sync::mpsc;
use std::thread;

pub enum TrayMessage {
    Show,
    Hide,
    Quit,
}

pub struct TrayIcon {
    _inner: Option<tray_icon::TrayIcon>,
    rx: mpsc::Receiver<TrayMessage>,
}

impl TrayIcon {
    pub fn new() -> (Self, mpsc::Sender<TrayMessage>) {
        let (tx, rx) = mpsc::channel();
        let builder = tray_icon::TrayIconBuilder::new()
            .with_tooltip("GoRun")
            .with_title("GoRun");

        let inner = builder.build().ok();

        (
            TrayIcon {
                _inner: inner,
                rx,
            },
            tx,
        )
    }

    pub fn try_recv(&self) -> Option<TrayMessage> {
        self.rx.try_recv().ok()
    }
}
