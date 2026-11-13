//! 文件操作：对话框选择 / 属性页 / 打开所在位置（P2-4 自 main.rs 拆出）

/// 系统文件选择对话框。取消或非 Windows 平台返回 None。
#[cfg(windows)]
pub fn browse_file_dialog(title: &str, filter: &str) -> Option<String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST,
    };
    use windows::core::{PCWSTR, PWSTR};

    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let filter_w: Vec<u16> = filter.encode_utf16().chain(std::iter::once(0)).collect();
    // P3-04：Windows 长路径上限 32767 个 UTF-16，1024 会截断超长路径导致对话框失败
    let mut file_buf = vec![0u16; 32768];
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: HWND::default(),
        lpstrFilter: PCWSTR(filter_w.as_ptr()),
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: file_buf.len() as u32,
        lpstrTitle: PCWSTR(title_w.as_ptr()),
        Flags: OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_EXPLORER,
        ..Default::default()
    };
    if unsafe { GetOpenFileNameW(&mut ofn).as_bool() } {
        let len = file_buf
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(file_buf.len());
        let path = String::from_utf16_lossy(&file_buf[..len]);
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn browse_file_dialog(_title: &str, _filter: &str) -> Option<String> {
    None
}

/// 系统保存文件对话框。取消或非 Windows 平台返回 None。
#[cfg(windows)]
pub fn save_file_dialog(title: &str, filter: &str) -> Option<String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Controls::Dialogs::{
        GetSaveFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
    };
    use windows::core::{PCWSTR, PWSTR};

    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let filter_w: Vec<u16> = filter.encode_utf16().chain(std::iter::once(0)).collect();
    let mut file_buf = vec![0u16; 32768];
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: HWND::default(),
        lpstrFilter: PCWSTR(filter_w.as_ptr()),
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: file_buf.len() as u32,
        lpstrTitle: PCWSTR(title_w.as_ptr()),
        Flags: OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_EXPLORER,
        ..Default::default()
    };
    if unsafe { GetSaveFileNameW(&mut ofn).as_bool() } {
        let len = file_buf
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(file_buf.len());
        let path = String::from_utf16_lossy(&file_buf[..len]);
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn save_file_dialog(_title: &str, _filter: &str) -> Option<String> {
    None
}

/// 显示文件属性页（ShellExecuteExW "properties" verb）。
#[cfg(windows)]
pub fn show_properties(path: &str) {
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    if path.trim().is_empty() {
        log::warn!("Properties: empty target");
        return;
    }

    let verb = "properties\0";
    let verb_wide: Vec<u16> = verb.encode_utf16().collect();
    let file_wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut sei: SHELLEXECUTEINFOW = unsafe { mem::zeroed() };
    sei.cbSize = mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_INVOKEIDLIST;
    sei.lpVerb = PCWSTR(verb_wide.as_ptr());
    sei.lpFile = PCWSTR(file_wide.as_ptr());
    sei.nShow = SW_SHOW.0;

    let success = unsafe { ShellExecuteExW(&mut sei) };
    if success.is_err() {
        log::error!("Failed to show properties for: {}", path);
    }
}

#[cfg(not(windows))]
pub fn show_properties(_path: &str) {
    log::warn!("Properties dialog is only supported on Windows");
}

/// 打开文件所在位置并选中（explorer /select）。
#[cfg(windows)]
pub fn open_file_location(path: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    if crate::runner::is_url(path) {
        log::debug!("Skip open file location for URL target: {}", path);
        return;
    }

    // P2-10：explorer /select 对含逗号的路径解析错位（已知 explorer 命令行限制），
    // 此时回退为仅打开父目录（可看到文件但无法选中），保证可用性；
    // 其余路径保持 /select 选中行为。
    let select = !path.contains(',');
    let target = if select {
        path.to_string()
    } else {
        match Path::new(path).parent() {
            // parent 为空（相对路径无目录分量）时回退原路径，避免 explorer 空参打开未知目录
            Some(parent) if !parent.as_os_str().is_empty() => {
                log::debug!("Path contains comma; opening parent dir: {:?}", parent);
                parent.to_string_lossy().into_owned()
            }
            _ => path.to_string(),
        }
    };
    let escaped = target.replace('"', "\"\"");
    let params_str = if select {
        format!("/select,\"{}\"", escaped)
    } else {
        format!("\"{}\"", escaped)
    };
    let params = OsStr::new(&params_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe {
        let result = ShellExecuteW(
            HWND::default(),
            windows::core::w!("open"),
            windows::core::w!("explorer"),
            windows::core::PCWSTR(params.as_ptr()),
            windows::core::PCWSTR(std::ptr::null()),
            SW_SHOW,
        );
        if result.0 <= 32 {
            log::error!(
                "Failed to open file location, error code: {}",
                result.0
            );
        } else {
            log::info!("Opened file location: {}", target);
        }
    }
}

#[cfg(not(windows))]
pub fn open_file_location(path: &str) {
    log::warn!(
        "Open file location not supported on non-Windows platform: {}",
        path
    );
}
