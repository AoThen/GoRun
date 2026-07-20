use std::path::PathBuf;
use std::sync::mpsc;

use windows::Win32::Foundation::POINTL;
use windows::Win32::System::Com::{
    CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_NONE, FORMATETC, IDataObject, IDispatch, CLSCTX_ALL,
};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

use crate::platform::DropMessage;

fn extract_files_from_hdrop(hdrop: *const u8) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let hdrop = HDROP(hdrop as isize);
    let count = unsafe { DragQueryFileW(hdrop, 0xFFFFFFFF, None) };
    let mut buf = vec![0u16; 32768];
    for i in 0..count {
        let len = unsafe { DragQueryFileW(hdrop, i, Some(&mut buf)) };
        if len > 0 {
            let s = String::from_utf16_lossy(&buf[..len as usize]);
            result.push(PathBuf::from(s));
        }
    }
    result
}

pub fn setup(hwnd: isize) -> mpsc::Receiver<DropMessage> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        unsafe {
            if windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED
                    | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE,
            )
            .is_err()
            {
                return;
            }

            let drop_target = DropTarget::new(tx);
            let drop_target_ptr: *const DropTarget = &drop_target;
            let hr = windows::Win32::System::Ole::RegisterDragDrop(
                windows::Win32::Foundation::HWND(hwnd as *mut _),
                drop_target_ptr as *mut _ as *mut _
            );
            if hr.is_err() {
                return;
            }

            loop {
                let mut msg = std::mem::MaybeUninit::<windows::Win32::UI::WindowsAndMessaging::MSG>::uninit();
                let ret = windows::Win32::UI::WindowsAndMessaging::GetMessageW(
                    msg.as_mut_ptr(),
                    None,
                    0,
                    0,
                );
                if ret.0 == 0 {
                    break;
                }
                let msg = msg.assume_init();
                let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                let _ = windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }
    });
    rx
}

#[repr(C)]
pub struct DropTarget {
    vtable: &'static DropTargetVTable,
    pub tx: mpsc::Sender<DropMessage>,
    ref_count: std::sync::atomic::AtomicU32,
}

impl DropTarget {
    fn new(tx: mpsc::Sender<DropMessage>) -> DropTarget {
        DropTarget {
            vtable: &IDROPTARGET_VTBL,
            tx,
            ref_count: std::sync::atomic::AtomicU32::new(1),
        }
    }
}

impl DropTarget {
    unsafe extern "system" fn query_interface(
        this: *mut std::ffi::c_void,
        riid: *const windows::core::GUID,
        ppv: *mut *mut std::ffi::c_void,
    ) -> windows::core::HRESULT {
        if *riid == windows::core::GUID::of::<IDropTarget>()
            || *riid == windows::core::GUID::of::<windows::Win32::System::Com::IUnknown>()
        {
            let this = this as *const DropTarget;
            *ppv = this as *mut std::ffi::c_void;
            ((*this).ref_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            windows::Win32::Foundation::S_OK
        } else {
            *ppv = std::ptr::null_mut();
            windows::Win32::Foundation::E_NOINTERFACE
        }
    }

    unsafe extern "system" fn add_ref(this: *const std::ffi::c_void) -> u32 {
        let this = this as *const DropTarget;
        (*this).ref_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    unsafe extern "system" fn release(this: *const std::ffi::c_void) -> u32 {
        let this = this as *const DropTarget;
        (*this).ref_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) - 1
    }

    unsafe extern "system" fn drag_enter(
        this: *const std::ffi::c_void,
        pDataObj: *const std::ffi::c_void,
        _grfKeyState: u32,
        _pt: POINTL,
        pdwEffect: *mut u32,
    ) -> windows::core::HRESULT {
        let this = this as *const DropTarget;
        let obj = &*(pDataObj as *const IDataObject);
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: windows::Win32::System::Com::DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: windows::Win32::System::Com::TYMED_HGLOBAL.0,
        };
        if obj.QueryGetData(&format).is_ok() {
            *pdwEffect = DROPEFFECT_COPY.0;
            let _ = (*this).tx.send(DropMessage::Enter);
            windows::Win32::Foundation::S_OK
        } else {
            *pdwEffect = DROPEFFECT_NONE.0;
            windows::Win32::Foundation::S_OK
        }
    }

    unsafe extern "system" fn drag_over(
        this: *const std::ffi::c_void,
        _grfKeyState: u32,
        _pt: POINTL,
        pdwEffect: *mut u32,
    ) -> windows::core::HRESULT {
        let this = this as *const DropTarget;
        *pdwEffect = DROPEFFECT_COPY.0;
        let _ = (*this).tx.send(DropMessage::Over(0.0, 0.0));
        windows::Win32::Foundation::S_OK
    }

    unsafe extern "system" fn drag_leave(this: *const std::ffi::c_void) -> windows::core::HRESULT {
        let this = this as *const DropTarget;
        let _ = (*this).tx.send(DropMessage::Leave);
        windows::Win32::Foundation::S_OK
    }

    unsafe extern "system" fn drop(
        this: *const std::ffi::c_void,
        pDataObj: *const std::ffi::c_void,
        _grfKeyState: u32,
        _pt: POINTL,
        _pdwEffect: *mut u32,
    ) -> windows::core::HRESULT {
        let this = this as *const DropTarget;
        let obj = &*(pDataObj as *const IDataObject);
        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: windows::Win32::System::Com::DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: windows::Win32::System::Com::TYMED_HGLOBAL.0,
        };
        if let Ok(medium) = obj.GetData(&format) {
            let hglobal = medium.Anonymous.hGlobal;
            let ptr = windows::Win32::System::Com::GlobalLock(hglobal);
            if !ptr.is_null() {
                let files = extract_files_from_hdrop(ptr as *const u8);
                let _ = (*this).tx.send(DropMessage::Drop(files));
                let _ = windows::Win32::System::Com::GlobalUnlock(hglobal);
            }
        }
        windows::Win32::Foundation::S_OK
    }
}

#[repr(C)]
pub struct DropTargetVTable {
    pub QueryInterface: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *const windows::core::GUID,
        *mut *mut std::ffi::c_void,
    ) -> windows::core::HRESULT,
    pub AddRef: unsafe extern "system" fn(*const std::ffi::c_void) -> u32,
    pub Release: unsafe extern "system" fn(*const std::ffi::c_void) -> u32,
    pub DragEnter: unsafe extern "system" fn(
        *const std::ffi::c_void,
        *const std::ffi::c_void,
        u32,
        POINTL,
        *mut u32,
    ) -> windows::core::HRESULT,
    pub DragOver: unsafe extern "system" fn(*const std::ffi::c_void, u32, POINTL, *mut u32) -> windows::core::HRESULT,
    pub DragLeave: unsafe extern "system" fn(*const std::ffi::c_void) -> windows::core::HRESULT,
    pub Drop: unsafe extern "system" fn(
        *const std::ffi::c_void,
        *const std::ffi::c_void,
        u32,
        POINTL,
        *mut u32,
    ) -> windows::core::HRESULT,
}

static IDROPTARGET_VTBL: DropTargetVTable = DropTargetVTable {
    query_interface: DropTarget::query_interface,
    add_ref: DropTarget::add_ref,
    release: DropTarget::release,
    drag_enter: DropTarget::drag_enter,
    drag_over: DropTarget::drag_over,
    drag_leave: DropTarget::drag_leave,
    drop: DropTarget::drop,
};
