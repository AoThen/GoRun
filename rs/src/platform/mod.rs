//! 平台抽象：窗口拖放（Windows 用窗口子类化 + WM_DROPFILES）。

use std::path::PathBuf;
use std::sync::mpsc::Sender;

mod dragdrop;

pub enum DropMessage {
    Drop(Vec<PathBuf>),
    /// 光标离开窗口（F7 自动隐藏）：WM_MOUSELEAVE。
    MouseLeft,
    /// 光标回到窗口（F7 自动隐藏）：WM_MOUSEMOVE（发过 MouseLeft 之后）。
    MouseEnter,
}

/// 拖放子类化的生命周期守卫。
///
/// 只负责"子类化 + refdata"这对资源：必须与窗口同寿，Drop 时先
/// `RemoveWindowSubclass` 再释放 refdata，避免回调读到已释放内存。
/// 消息接收端（Receiver）在构造时一并返回，由消费线程独占。
pub struct DropHandler {
    hwnd: isize,
    /// 子类化 refdata：指向堆上的 `Sender`，与 `_tx` 的所有权绑定。
    refdata: *mut Sender<DropMessage>,
    _tx: Box<Sender<DropMessage>>,
}

impl DropHandler {
    /// 子类化窗口并把 `WM_DROPFILES` 路径转发到返回的 `Receiver`。
    pub fn new(hwnd: isize) -> (Self, std::sync::mpsc::Receiver<DropMessage>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut tx = Box::new(tx);
        let refdata: *mut Sender<DropMessage> = &mut *tx;
        dragdrop::setup_dragdrop(hwnd, refdata as usize);
        (
            DropHandler {
                hwnd,
                refdata,
                _tx: tx,
            },
            rx,
        )
    }
}

impl Drop for DropHandler {
    fn drop(&mut self) {
        // P0：顺序不可颠倒 —— 先摘子类化，回调不再可能读到 refdata
        dragdrop::remove_dragdrop(self.hwnd);
        debug_assert_eq!(
            self.refdata,
            &*self._tx as *const Sender<DropMessage> as *mut Sender<DropMessage>
        );
    }
}

// refdata 指向堆分配，且只在主线程（窗口消息循环）读写；
// 标记 Send 以便经 AppState 传递。
unsafe impl Send for DropHandler {}

/// 安装拖放，返回 (生命周期守卫, 消息接收端)。
pub fn setup_dragdrop(
    hwnd: isize,
) -> (DropHandler, std::sync::mpsc::Receiver<DropMessage>) {
    DropHandler::new(hwnd)
}
