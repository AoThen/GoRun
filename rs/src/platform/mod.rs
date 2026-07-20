use std::path::PathBuf;

pub enum DropMessage {
    Enter,
    Over(f64, f64),
    Leave,
    Drop(Vec<PathBuf>),
}

pub fn setup_dragdrop(_hwnd: isize) -> std::sync::mpsc::Receiver<DropMessage> {
    let (_tx, rx) = std::sync::mpsc::channel();
    rx
}
