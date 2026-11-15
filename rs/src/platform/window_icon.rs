//! 主窗口应用图标：把 build.rs 嵌入的 app.ico（资源 ID 1）设为标题栏图标。
//!
//! Slint/winit 不提供窗口图标 API；窗口 realize 后经 raw-window-handle
//! 取 HWND，用 WM_SETICON 做实例级设置（不改 winit 共享窗口类）。

/// 窗口 realize 后调用；非 Windows 目标为空实现。
pub fn apply_app_icon(window: &slint::Window) {
    apply_impl(window);
}

#[cfg(windows)]
fn apply_impl(window: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, LoadImageW, SendMessageW, HICON, IMAGE_ICON, ICON_BIG, ICON_SMALL,
        LR_DEFAULTSIZE, LR_SHARED, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
    };

    // slint 的固有 Window::window_handle() 返回 WindowHandle；
    // 再经 HasWindowHandle 拿借用的原始句柄（注意与固有方法同名，用完全限定调用）。
    let handle = window.window_handle();
    let Ok(borrowed) = HasWindowHandle::window_handle(&handle) else {
        return;
    };
    let RawWindowHandle::Win32(win) = borrowed.as_raw() else {
        return;
    };
    let hwnd = HWND(win.hwnd.get());
    let Ok(hmodule) = (unsafe { GetModuleHandleW(None) }) else {
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
        }
        if let Some(small) = load_icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON))
        {
            SendMessageW(
                hwnd,
                WM_SETICON,
                WPARAM(ICON_SMALL as usize),
                LPARAM(small.0 as isize),
            );
        }
    }
}

#[cfg(not(windows))]
fn apply_impl(_window: &slint::Window) {}
