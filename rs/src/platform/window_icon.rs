//! 主窗口应用图标：把 build.rs 嵌入的 app.ico（资源 ID 1）设为标题栏图标。
//!
//! Slint/winit 不提供窗口图标 API；窗口就绪后取 HWND，用 WM_SETICON
//! 做实例级设置（不改 winit 共享窗口类）。

/// 窗口就绪后调用（hwnd 来自 [`crate::drop_handler::try_get_hwnd`]）；
/// 非 Windows 目标为空实现。
pub fn apply_app_icon(hwnd: isize) {
    apply_impl(hwnd);
}

#[cfg(windows)]
fn apply_impl(hwnd: isize) {
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, LoadImageW, SendMessageW, HICON, IMAGE_ICON, ICON_BIG, ICON_SMALL,
        LR_DEFAULTSIZE, LR_SHARED, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
    };

    let hwnd = HWND(hwnd);
    let Ok(hmodule) = (unsafe { GetModuleHandleW(None) }) else {
        log::warn!("Failed to get module handle; title bar icon not set");
        return;
    };
    let hinstance = HINSTANCE(hmodule.0);
    // 资源 ID 1 = build.rs winresource 嵌入的 app.ico（与托盘一致）；
    // LR_SHARED 句柄与进程同寿，无需 DestroyIcon。
    let load_icon = |cx: i32, cy: i32| -> Option<HICON> {
        unsafe {
            LoadImageW(
                hinstance,
                windows::core::PCWSTR(1 as *const u16),
                IMAGE_ICON,
                cx,
                cy,
                LR_DEFAULTSIZE | LR_SHARED,
            )
            .ok()
            .map(|h| HICON(h.0))
        }
    };
    unsafe {
        if let Some(big) = load_icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)) {
            SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_BIG as usize),
                LPARAM(big.0 as isize),
            );
        } else {
            log::warn!("Failed to load app icon (big); title bar icon not set");
        }
        if let Some(small) = load_icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON))
        {
            SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_SMALL as usize),
                LPARAM(small.0 as isize),
            );
        } else {
            log::warn!("Failed to load app icon (small); title bar icon not set");
        }
    }
}

#[cfg(not(windows))]
fn apply_impl(_hwnd: isize) {}
