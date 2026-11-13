//! 剪贴板操作（P2-4 自 main.rs 拆出；P2-7 用 RAII 守卫统一管理 Open/Close）

/// RAII 剪贴板守卫：open 成功持有，drop 自动 Close（避免各分支手动 Close 泄漏）。
#[cfg(windows)]
struct ClipboardGuard {
    _private: (),
}

#[cfg(windows)]
impl ClipboardGuard {
    /// 打开剪贴板（当前线程 HWND::default()）。失败返回 None（已记录错误）。
    fn open() -> Option<Self> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::System::DataExchange::OpenClipboard;
        match unsafe { OpenClipboard(HWND::default()) } {
            Ok(_) => Some(Self { _private: () }),
            Err(e) => {
                log::error!("Failed to open clipboard: {:?}", e);
                None
            }
        }
    }
}

#[cfg(windows)]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        use windows::Win32::System::DataExchange::CloseClipboard;
        let _ = unsafe { CloseClipboard() };
    }
}

/// 获取剪贴板中的文件路径（CF_HDROP）。失败或无内容返回空 Vec。
#[cfg(windows)]
pub fn clipboard_files() -> Vec<std::path::PathBuf> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::GetClipboardData;
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    let mut result = Vec::new();
    // P2-7：RAII 守卫，所有提前 return 路径均自动关闭剪贴板
    let _guard = match ClipboardGuard::open() {
        Some(g) => g,
        None => return result,
    };
    unsafe {
        let hdrop = match GetClipboardData(CF_HDROP.0 as u32) {
            Ok(h) => h,
            Err(_) => return result,
        };
        if hdrop.0 != 0 {
            let hg = HGLOBAL(hdrop.0 as *mut std::ffi::c_void);
            let locked = GlobalLock(hg);
            if !locked.is_null() {
                let count = DragQueryFileW(HDROP(locked as isize), u32::MAX, None);
                for i in 0..count {
                    // P4-02：先用 NULL 查询所需长度（含终止 NUL），再按精确长度分配。
                    // 固定 1024 缓冲会静默截断超长路径（Windows 上限 32767 UTF-16）。
                    let needed = DragQueryFileW(HDROP(locked as isize), i, None);
                    if needed == 0 {
                        continue;
                    }
                    let mut buf = vec![0u16; needed as usize];
                    let len = DragQueryFileW(HDROP(locked as isize), i, Some(&mut buf));
                    if len > 0 && (len as usize) < buf.len() {
                        let p = String::from_utf16_lossy(&buf[..len as usize]);
                        result.push(std::path::PathBuf::from(p));
                    }
                }
                let _ = GlobalUnlock(hg);
            }
        }
    }
    result
}

#[cfg(not(windows))]
pub fn clipboard_files() -> Vec<std::path::PathBuf> {
    log::warn!("Clipboard file paste is only supported on Windows");
    Vec::new()
}

/// 复制文本到剪贴板（CF_UNICODETEXT）。
pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Foundation::{GlobalFree, HANDLE};
        use windows::Win32::System::DataExchange::{
            EmptyClipboard, SetClipboardData,
        };
        use windows::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
        };
        use windows::Win32::System::Ole::CF_UNICODETEXT;

        // P2-7：RAII 守卫，各失败分支 return 时自动关闭剪贴板
        let _guard = match ClipboardGuard::open() {
            Some(g) => g,
            None => return false,
        };

        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let size = wide.len() * std::mem::size_of::<u16>();

        if unsafe { EmptyClipboard() }.is_err() {
            log::error!("Failed to empty clipboard");
            return false;
        }

        let h_mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) }.unwrap_or_default();
        if h_mem.is_invalid() {
            log::error!("Failed to allocate global memory for clipboard");
            return false;
        }

        let ptr = unsafe { GlobalLock(h_mem) };
        if ptr.is_null() {
            log::error!("Failed to lock global memory for clipboard");
            unsafe {
                let _ = GlobalFree(h_mem);
            }
            return false;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
            let _ = GlobalUnlock(h_mem);
            if SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(h_mem.0 as isize)).is_err() {
                log::error!("Failed to set clipboard data");
                let _ = GlobalFree(h_mem);
                return false;
            }
        }

        // 仅记录长度，避免敏感内容（路径/密钥）落入日志（P1-6）
        log::info!("Copied to clipboard: {} chars", text.chars().count());
        true
    }

    #[cfg(not(windows))]
    {
        let _ = text;
        log::warn!("Clipboard operation not supported on non-Windows platform");
        false
    }
}
