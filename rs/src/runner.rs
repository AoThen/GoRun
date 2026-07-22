#![allow(dead_code)]

use std::path::Path;

use crate::model::{Item, RunError, RunResult};

pub fn run(item: &Item) -> RunResult {
    execute(item, false)
}

pub fn run_as_admin(item: &Item) -> RunResult {
    execute(item, true)
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

pub fn expand_variables(target: &str, search_param: &str) -> String {
    let mut result = target.to_string();

    if result.contains("%mp%") {
        if let Some(parent) = Path::new(&result).parent() {
            result = result.replace("%mp%", &parent.to_string_lossy());
        }
    }

    if result.contains("%mr%") {
        if let Some(path) = Path::new(&result).parent() {
            let path_str = path.to_string_lossy();
            if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
                let drive = &path_str[..2];
                result = result.replace("%mr%", drive);
            }
        }
    }

    result = result.replace("%so%", search_param);
    result = result.replace("%so-url%", &url_encode(search_param));

    expand_win_env_vars(&result)
}

pub fn get_error_message(error: &RunError) -> String {
    match error {
        RunError::None => "成功".to_string(),
        RunError::FileNotFound => "找不到指定的文件".to_string(),
        RunError::PathNotFound => "找不到指定的路径".to_string(),
        RunError::AccessDenied => "访问被拒绝".to_string(),
        RunError::OutOfMemory => "内存不足".to_string(),
        RunError::DllNotFound => "找不到指定的 DLL 文件".to_string(),
        RunError::Unknown => "未知错误".to_string(),
    }
}

fn execute(item: &Item, admin: bool) -> RunResult {
    let target = expand_variables(&item.target, "");

    if !is_url(&target) && !Path::new(&target).exists() {
        return RunResult {
            success: false,
            error: RunError::FileNotFound,
            error_message: get_error_message(&RunError::FileNotFound),
        };
    }

    let arguments = if item.arguments.is_empty() {
        None
    } else {
        Some(item.arguments.as_str())
    };

    let working_dir = if item.working_dir.is_empty() {
        None
    } else {
        Some(item.working_dir.as_str())
    };

    shell_execute(&target, arguments, working_dir, admin)
}

fn shell_execute(
    target: &str,
    arguments: Option<&str>,
    working_dir: Option<&str>,
    admin: bool,
) -> RunResult {
    #[cfg(windows)]
    {
        use std::mem;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{CloseHandle, GetLastError};
        use windows::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let verb = if admin { "runas\0" } else { "open\0" };
        let verb_wide: Vec<u16> = verb.encode_utf16().collect();
        let file_wide: Vec<u16> = to_wide_chars(target);
        let params_wide: Vec<u16> = arguments.map_or(vec![0], |s| to_wide_chars(s));
        let dir_wide: Vec<u16> = working_dir.map_or(vec![0], |s| to_wide_chars(s));

        let mut sei: SHELLEXECUTEINFOW = unsafe { mem::zeroed() };
        sei.cbSize = mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = PCWSTR(verb_wide.as_ptr());
        sei.lpFile = PCWSTR(file_wide.as_ptr());
        sei.lpParameters = if arguments.is_some() {
            PCWSTR(params_wide.as_ptr())
        } else {
            PCWSTR::null()
        };
        sei.lpDirectory = if working_dir.is_some() {
            PCWSTR(dir_wide.as_ptr())
        } else {
            PCWSTR::null()
        };
        sei.nShow = SW_SHOWNORMAL.0;

        let success = unsafe { ShellExecuteExW(&mut sei) };

        if success.as_bool() {
            if !sei.hProcess.is_invalid() {
                unsafe { CloseHandle(sei.hProcess) };
            }
            RunResult {
                success: true,
                error: RunError::None,
                error_message: get_error_message(&RunError::None),
            }
        } else {
            let error_code = unsafe { GetLastError() };

            if admin && error_code.0 == 1223 {
                return RunResult {
                    success: false,
                    error: RunError::AccessDenied,
                    error_message: "用户取消了提权请求".to_string(),
                };
            }

            let run_error = map_error(error_code.0);
            RunResult {
                success: false,
                error: run_error.clone(),
                error_message: get_error_message(&run_error),
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (target, arguments, working_dir, admin);
        RunResult {
            success: false,
            error: RunError::Unknown,
            error_message: get_error_message(&RunError::Unknown),
        }
    }
}

fn map_error(error_code: u32) -> RunError {
    match error_code {
        2 => RunError::FileNotFound,
        3 => RunError::PathNotFound,
        5 => RunError::AccessDenied,
        8 => RunError::OutOfMemory,
        126 => RunError::DllNotFound,
        _ => RunError::Unknown,
    }
}

fn expand_win_env_vars(input: &str) -> String {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

        let wide: Vec<u16> = to_wide_chars(input);
        let mut buf: Vec<u16> = vec![0u16; 32768];
        let len =
            unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(buf.as_mut_slice())) };

        if len == 0 {
            return input.to_string();
        }

        buf.truncate(len as usize);
        if buf.last() == Some(&0) {
            buf.pop();
        }

        String::from_utf16_lossy(&buf)
    }

    #[cfg(not(windows))]
    {
        input.to_string()
    }
}

#[cfg(windows)]
fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(not(windows))]
fn to_wide_chars(_s: &str) -> Vec<u16> {
    Vec::new()
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
