//! 窗口拖放与鼠标跟踪（Windows），双轨实现：
//!
//! - **拖放**：OLE `IDropTarget`（`RegisterDragDrop`）。旧式
//!   `DragAcceptFiles`+`WM_DROPFILES` 依赖窗口子类化，winit 窗口生命周期内
//!   可能重建导致子类化失效、拖放静默失灵；OLE 注册是 Windows 现代标准路径。
//! - **鼠标跟踪**：轻量子类化 `WM_MOUSEMOVE`/`WM_MOUSELEAVE`，支撑 F7
//!   自动隐藏（普通鼠标离开窗口即触发，与是否处于拖放会话无关）。

#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::mpsc::Sender;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use super::DropMessage;

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINTL, WPARAM};
#[cfg(windows)]
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
// OLE 拖放 API 在 System::Ole（非 UI::Shell）；CF_HDROP 为 CLIPBOARD_FORMAT(u16)。
#[cfg(windows)]
use windows::Win32::System::Ole::{
    RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop, CF_HDROP, DROPEFFECT_COPY, IDropTarget,
    IDropTarget_Impl,
};
// grfkeystate 参数类型 MODIFIERKEYS_FLAGS 位于 SystemServices（0.57 实签）。
#[cfg(windows)]
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TRACKMOUSEEVENT, TME_LEAVE};
#[cfg(windows)]
use windows::Win32::UI::Shell::{DefSubclassProc, DragQueryFileW, HDROP, RemoveWindowSubclass};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
#[cfg(windows)]
use windows::Win32::UI::Controls::WM_MOUSELEAVE;

/// 鼠标离开状态（F7 自动隐藏）：置位表示已发过 MouseLeft（光标在窗口外），
/// 下次 WM_MOUSEMOVE（回到窗口）时发 MouseEnter 取消隐藏。单窗口单实例。
#[cfg(windows)]
static LEFT_SENT: AtomicBool = AtomicBool::new(false);

/// 子类化回调：仅处理鼠标进出跟踪（F7 自动隐藏）。
/// 模块级定义（非嵌套），供 setup/remove 成对引用（RemoveWindowSubclass 需同函数指针）。
#[cfg(windows)]
unsafe extern "system" fn mouse_subclass_proc(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> LRESULT {
    if umsg == WM_MOUSEMOVE {
        // F7 自动隐藏：光标在窗口内移动时注册 leave 跟踪（每次 move 重置）
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        let _ = TrackMouseEvent(&mut tme);
        // 之前发过 MouseLeft（光标在窗口外）且现在又 move → 回到窗口，通知取消隐藏
        if LEFT_SENT.swap(false, Ordering::SeqCst) {
            let sender = &*(_dwrefdata as *const Sender<DropMessage>);
            sender.send(DropMessage::MouseEnter).ok();
        }
        DefSubclassProc(hwnd, umsg, wparam, lparam)
    } else if umsg == WM_MOUSELEAVE {
        // F7 自动隐藏：光标离开窗口 → 置位并通知（消费线程延迟后隐藏）
        LEFT_SENT.store(true, Ordering::SeqCst);
        let sender = &*(_dwrefdata as *const Sender<DropMessage>);
        sender.send(DropMessage::MouseLeft).ok();
        LRESULT(0)
    } else {
        DefSubclassProc(hwnd, umsg, wparam, lparam)
    }
}

/// 安装鼠标跟踪子类化（F7 自动隐藏）。
#[cfg(windows)]
pub fn setup_mouse_tracking(hwnd: isize, sender_ptr: usize) {
    use windows::Win32::UI::Shell::SetWindowSubclass;

    unsafe {
        let hwnd = HWND(hwnd);
        // 已装过则先摘除，避免重复子类化叠加（幂等）。
        let _ = RemoveWindowSubclass(hwnd, Some(mouse_subclass_proc), 0);
        let ok = SetWindowSubclass(hwnd, Some(mouse_subclass_proc), 0, sender_ptr);
        if ok.as_bool() {
            log::debug!("Mouse tracking subclass installed on hwnd={}", hwnd.0);
        } else {
            log::warn!("SetWindowSubclass failed for hwnd={}", hwnd.0);
        }
    }
}

/// 移除鼠标跟踪子类化（与 setup 成对，uidsubclass=0 一致）。
#[cfg(windows)]
pub fn remove_mouse_tracking(hwnd: isize) {
    unsafe {
        let result = RemoveWindowSubclass(HWND(hwnd), Some(mouse_subclass_proc), 0);
        if !result.as_bool() {
            log::debug!("RemoveWindowSubclass failed for hwnd={}", hwnd);
        }
    }
}

/// OLE 拖放目标：DragEnter/Leave 桥接到 MouseEnter/MouseLeft（F7 语义），
/// Drop 时枚举 CF_HDROP 文件路径发往消费线程。
#[cfg(windows)]
#[windows::core::implement(IDropTarget)]
struct OleDropTarget {
    sender: Arc<Sender<DropMessage>>,
}

#[cfg(windows)]
// implement 宏约定（0.57+，0.62 起宏内置无需 feature）：直接为原 struct 实现 _Impl trait（无 _Impl 包装类型）
impl IDropTarget_Impl for OleDropTarget {
    fn DragEnter(
        &self,
        _pdataobj: Option<&IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        LEFT_SENT.store(false, Ordering::SeqCst);
        self.sender.send(DropMessage::MouseEnter).ok();
        if !pdweffect.is_null() {
            unsafe { *pdweffect = DROPEFFECT_COPY };
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        if !pdweffect.is_null() {
            unsafe { *pdweffect = DROPEFFECT_COPY };
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        LEFT_SENT.store(true, Ordering::SeqCst);
        self.sender.send(DropMessage::MouseLeft).ok();
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: Option<&IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        _pt: &POINTL,
        pdweffect: *mut windows::Win32::System::Ole::DROPEFFECT,
    ) -> windows::core::Result<()> {
        if !pdweffect.is_null() {
            unsafe { *pdweffect = DROPEFFECT_COPY };
        }
        if let Some(data) = pdataobj {
            let fmt = FORMATETC {
                cfFormat: CF_HDROP.0,
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
                ptd: std::ptr::null_mut(),
            };
            // SAFETY：GetData/ReleaseStgMedium 为标准 COM 数据传输调用，
            // fmt 为栈上有效结构；成功返回的 medium 必须用 ReleaseStgMedium 归还。
            unsafe {
                match data.GetData(&fmt) {
                    Ok(mut medium) => {
                        let hdrop = HDROP(medium.u.hGlobal.0 as isize);
                        let file_count = DragQueryFileW(hdrop, u32::MAX, None);
                        let mut paths = Vec::new();
                        for i in 0..file_count {
                            let len = DragQueryFileW(hdrop, i, None);
                            if len > 0 {
                                let mut buf = vec![0u16; (len + 1) as usize];
                                let copied = DragQueryFileW(hdrop, i, Some(&mut buf));
                                if copied > 0 {
                                    let path = String::from_utf16_lossy(&buf[..copied as usize]);
                                    paths.push(PathBuf::from(path));
                                }
                            }
                        }
                        ReleaseStgMedium(&mut medium);
                        self.sender.send(DropMessage::Drop(paths)).ok();
                    }
                    Err(e) => {
                        log::warn!("GetData(CF_HDROP) failed on drop: {}", e);
                    }
                }
            }
        }
        Ok(())
    }
}

/// 注册 OLE 拖放目标。成功时返回 COM 包装，调用方必须保持其存活
/// （RegisterDragDrop 不增加引用计数，对象失效后系统回调将踩空）。
#[cfg(windows)]
pub fn register_ole_drop(
    hwnd: isize,
    sender: Arc<Sender<DropMessage>>,
) -> Option<IDropTarget> {
    unsafe {
        // 幂等：先撤销旧注册再注册
        let _ = RevokeDragDrop(HWND(hwnd));
        let target: IDropTarget = OleDropTarget { sender }.into();
        match RegisterDragDrop(HWND(hwnd), &target) {
            Ok(()) => {
                log::debug!("OLE drop target registered on hwnd={}", hwnd);
                Some(target)
            }
            Err(e) => {
                log::warn!("RegisterDragDrop failed for hwnd={}: {}", hwnd, e);
                None
            }
        }
    }
}

/// 撤销 OLE 拖放注册。
#[cfg(windows)]
pub fn unregister_ole_drop(hwnd: isize) {
    unsafe {
        if let Err(e) = RevokeDragDrop(HWND(hwnd)) {
            log::debug!("RevokeDragDrop failed for hwnd={}: {}", hwnd, e);
        }
    }
}

// 非 Windows 平台 stub：签名用完整路径，避免依赖 cfg(windows) 的 use 导入。
#[cfg(not(windows))]
pub fn setup_mouse_tracking(_hwnd: isize, _sender_ptr: usize) {}

#[cfg(not(windows))]
pub fn remove_mouse_tracking(_hwnd: isize) {}

#[cfg(not(windows))]
pub fn register_ole_drop(
    _hwnd: isize,
    _sender: std::sync::Arc<std::sync::mpsc::Sender<super::DropMessage>>,
) -> Option<()> {
    None
}

#[cfg(not(windows))]
pub fn unregister_ole_drop(_hwnd: isize) {}
