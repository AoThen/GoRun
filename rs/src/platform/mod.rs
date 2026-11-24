//! 平台抽象：窗口拖放（Windows：OLE IDropTarget + 鼠标跟踪子类化）与主窗口图标设置。

use std::path::PathBuf;
use std::sync::mpsc::Sender;

mod dragdrop;

pub mod window_icon;

pub enum DropMessage {
    Drop(Vec<PathBuf>),
    /// 光标离开窗口（F7 自动隐藏）：WM_MOUSELEAVE / OLE DragLeave。
    MouseLeft,
    /// 光标回到窗口（F7 自动隐藏）：WM_MOUSEMOVE / OLE DragEnter。
    MouseEnter,
}

/// 拖放装配的生命周期守卫。
///
/// Windows 持有必须与窗口同寿的资源：
/// - 鼠标跟踪子类化 + refdata（F7 自动隐藏）；
/// - OLE `IDropTarget` COM 对象（`RegisterDragDrop` 不增引用，必须保持存活到 `RevokeDragDrop`）。
///
/// Drop 时先撤销 OLE 注册、再摘子类化，最后才释放 refdata/Sender，避免回调踩空。
///
/// 消息接收端（Receiver）在构造时一并返回，由消费线程独占。
pub struct DropHandler {
    hwnd: isize,
    /// 子类化 refdata：指向堆上的 `Sender`，与 `_tx` 的所有权绑定。
    refdata: *mut Sender<DropMessage>,
    _tx: Box<Sender<DropMessage>>,
    /// OLE 拖放目标；None 表示注册失败（拖放禁用，鼠标跟踪仍有效）。
    #[cfg(windows)]
    _ole_target: Option<windows::Win32::System::Ole::IDropTarget>,
}

impl DropHandler {
    /// 装配拖放与鼠标跟踪，把文件路径/进出事件转发到返回的 `Receiver`。
    #[cfg(windows)]
    pub fn new(hwnd: isize) -> (Self, std::sync::mpsc::Receiver<DropMessage>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut tx = Box::new(tx);
        let refdata: *mut Sender<DropMessage> = &mut *tx;
        dragdrop::setup_mouse_tracking(hwnd, refdata as usize);
        let ole_target = dragdrop::register_ole_drop(hwnd, std::sync::Arc::new((*tx).clone()));
        (
            DropHandler {
                hwnd,
                refdata,
                _tx: tx,
                _ole_target: ole_target,
            },
            rx,
        )
    }

    /// 非 Windows：仅保留统一的装配入口，无拖放能力。
    #[cfg(not(windows))]
    pub fn new(hwnd: isize) -> (Self, std::sync::mpsc::Receiver<DropMessage>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut tx = Box::new(tx);
        let refdata: *mut Sender<DropMessage> = &mut *tx;
        dragdrop::setup_mouse_tracking(hwnd, refdata as usize);
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
        // P0：顺序不可颠倒 —— 先撤 OLE 注册与子类化，回调不再可能读到已释放资源
        dragdrop::unregister_ole_drop(self.hwnd);
        dragdrop::remove_mouse_tracking(self.hwnd);
        #[cfg(windows)]
        {
            self._ole_target = None;
        }
        debug_assert_eq!(
            self.refdata,
            &*self._tx as *const Sender<DropMessage> as *mut Sender<DropMessage>
        );
    }
}

// refdata 指向堆分配，且只在主线程（窗口消息循环）读写；
// COM 对象由系统在窗口线程回调。标记 Send 以便经 AppState 传递。
unsafe impl Send for DropHandler {}

/// 安装拖放，返回 (生命周期守卫, 消息接收端)。
pub fn setup_dragdrop(hwnd: isize) -> (DropHandler, std::sync::mpsc::Receiver<DropMessage>) {
    DropHandler::new(hwnd)
}
