use crate::model::{Item, RunError, RunResult};

#[cfg(windows)]
pub fn run(item: &Item) -> RunResult {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = OsStr::new(&item.target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let args = if item.arguments.is_empty() {
        None
    } else {
        let v: Vec<u16> = OsStr::new(&item.arguments)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        Some(v)
    };
    let dir = if item.working_dir.is_empty() {
        None
    } else {
        let v: Vec<u16> = OsStr::new(&item.working_dir)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        Some(v)
    };

    let verb = if item.run_as_admin {
        "runas\0".encode_utf16().collect::<Vec<_>>()
    } else {
        "open\0".encode_utf16().collect::<Vec<_>>()
    };

    unsafe {
        let mut sei = windows::Win32::UI::Shell::SHELLEXECUTEINFOW::default();
        sei.cbSize = std::mem::size_of::<windows::Win32::UI::Shell::SHELLEXECUTEINFOW>() as u32;
        sei.lpFile = windows::core::PCWSTR(wide.as_ptr());
        sei.lpVerb = windows::core::PCWSTR(verb.as_ptr());
        if let Some(a) = &args {
            if !item.arguments.is_empty() {
                sei.lpParameters = windows::core::PCWSTR(a.as_ptr());
            }
        }
        if let Some(d) = &dir {
            sei.lpDirectory = windows::core::PCWSTR(d.as_ptr());
        }
        sei.nShow = windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL.0;

        let result = windows::Win32::UI::Shell::ShellExecuteExW(&mut sei);
        if result.into() {
            RunResult {
                success: true,
                error: RunError::None,
                error_message: String::new(),
            }
        } else {
            let err = windows::Win32::Foundation::GetLastError();
            RunResult {
                success: false,
                error: map_error(err.0),
                error_message: format!("Windows error {}", err.0),
            }
        }
    }
}

#[cfg(not(windows))]
pub fn run(_item: &Item) -> RunResult {
    RunResult {
        success: false,
        error: RunError::Unknown,
        error_message: "Only Windows is supported".to_string(),
    }
}

fn map_error(code: u32) -> RunError {
    use windows::Win32::Foundation;
    match code {
        Foundation::ERROR_FILE_NOT_FOUND => RunError::FileNotFound,
        Foundation::ERROR_PATH_NOT_FOUND => RunError::PathNotFound,
        Foundation::ERROR_ACCESS_DENIED => RunError::AccessDenied,
        Foundation::ERROR_NOT_ENOUGH_MEMORY => RunError::OutOfMemory,
        Foundation::ERROR_DLL_NOT_FOUND => RunError::DllNotFound,
        _ => RunError::Unknown,
    }
}

pub fn is_url(target: &str) -> bool {
    let t = target.to_lowercase();
    t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("ftp://")
        || t.starts_with("ftps://")
        || t.starts_with("steam://")
        || t.starts_with("mailto:")
}
