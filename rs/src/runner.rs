use std::path::Path;

use crate::model::{Item, RunError, RunResult};

pub fn run(item: &Item, search_param: &str) -> RunResult {
    execute(item, false, search_param)
}

pub fn run_as_admin(item: &Item, search_param: &str) -> RunResult {
    execute(item, true, search_param)
}

/// 运行分类内全部项目（逐一执行，单个失败不影响后续）。
/// 返回逐项结果（顺序与 `items` 一致），供调用方反馈与递增运行计数。
pub fn run_all(items: &[Item], search_param: &str) -> Vec<RunResult> {
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let result = run(item, search_param);
        if !result.success {
            log::warn!(
                "Run all: failed to run {} ({}) - {}",
                item.name,
                item.id,
                result.error_message
            );
        }
        results.push(result);
    }
    results
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

/// 展开顺序（P0-1）：%mp%/%mr% → 环境变量 → %so%/%so-url%。
/// 环境变量展开必须发生在搜索参数替换之前，防止搜索文本中的 %VAR% 被展开。
/// 生产路径用 execute 内的分步展开（注入检查在两步之间），此一步式封装仅供测试/外部调用。
#[allow(dead_code)]
pub fn expand_variables(text: &str, base_target: &str, search_param: &str) -> String {
    expand_search(&expand_pre(text, base_target), search_param)
}

/// 展开 %mp%/%mr% 与 Windows 环境变量（不含搜索参数）。
pub fn expand_pre(text: &str, base_target: &str) -> String {
    let mut result = text.to_string();

    if result.contains("%mp%") {
        if let Some(parent) = Path::new(base_target).parent() {
            result = result.replace("%mp%", &parent.to_string_lossy());
        }
    }

    if result.contains("%mr%") {
        if let Some(path) = Path::new(base_target).parent() {
            let path_str = path.to_string_lossy();
            if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
                let drive = &path_str[..2];
                result = result.replace("%mr%", drive);
            }
        }
    }

    expand_win_env_vars(&result)
}

/// 替换 %so%/%so-url%。单遍扫描：搜索文本中出现的字面量 %so%/%so-url% 不会被二次展开。
pub fn expand_search(text: &str, search_param: &str) -> String {
    let encoded = url_encode(search_param);
    let mut out = String::new();
    let mut rest = text;
    loop {
        let so = rest.find("%so%");
        let url = rest.find("%so-url%");
        // 两个标记互不为子串（"%so%" 不是 "%so-url%" 的子串），同位不可能同时命中
        let (pos, use_url) = match (so, url) {
            (None, None) => break,
            (Some(s), u) => (s, u.map_or(false, |u| u < s)),
            (None, Some(u)) => (u, true),
        };
        out.push_str(&rest[..pos]);
        if use_url {
            out.push_str(&encoded);
            rest = &rest[pos + "%so-url%".len()..];
        } else {
            out.push_str(search_param);
            rest = &rest[pos + "%so%".len()..];
        }
    }
    out.push_str(rest);
    out
}

pub fn get_error_message(error: &RunError) -> String {
    match error {
        RunError::None => "成功".to_string(),
        RunError::FileNotFound => "找不到指定的文件".to_string(),
        RunError::PathNotFound => "找不到指定的路径".to_string(),
        RunError::AccessDenied => "访问被拒绝".to_string(),
        RunError::OutOfMemory => "内存不足".to_string(),
        RunError::DllNotFound => "找不到指定的 DLL 文件".to_string(),
        RunError::Cancelled => "用户取消了提权请求".to_string(),
        RunError::UnsafeSearchParam => "搜索参数含不安全命令字符，已拒绝执行".to_string(),
        RunError::Unknown => "未知错误".to_string(),
    }
}

/// P1-7：在目录列表中查找可执行文件（纯函数，便于单测）。
/// 先试精确名，再对无扩展名的输入按 PATHEXT 逐后缀尝试。
pub fn find_in_path_dirs(
    name: &str,
    dirs: &[std::path::PathBuf],
    pathext: &[String],
) -> Option<std::path::PathBuf> {
    if name.is_empty() {
        return None;
    }
    let bare = std::path::Path::new(name).extension().is_none();
    for dir in dirs {
        let exact = dir.join(name);
        if exact.is_file() {
            return Some(exact);
        }
        if bare {
            for ext in pathext {
                let c = dir.join(format!("{}.{}", name, ext.trim_start_matches('.')));
                if c.is_file() {
                    return Some(c);
                }
            }
        }
    }
    None
}

/// P1-7：裸文件名（不含路径分隔符）在 PATH 中解析，解析不到返回 None。
pub fn resolve_bare_name(target: &str) -> Option<String> {
    if target.is_empty() || target.contains('/') || target.contains('\\') {
        return None;
    }
    let dirs: Vec<std::path::PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).map(|d| d.to_path_buf()).collect())
        .unwrap_or_default();
    let pathext: Vec<String> = std::env::var_os("PATHEXT")
        .map(|p| {
            p.to_string_lossy()
                .split(';')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_else(|| {
            [".COM", ".EXE", ".BAT", ".CMD"].iter().map(|s| s.to_string()).collect()
        });
    find_in_path_dirs(target, &dirs, &pathext)
        .map(|p| p.to_string_lossy().to_string())
}

fn execute(item: &Item, admin: bool, search_param: &str) -> RunResult {
    let base_target = item.target.clone();
    let mut target = expand_search(&expand_pre(&item.target, &base_target), search_param);

    // P1-7：裸文件名（如 notepad）按 PATH 解析为完整路径，解析失败保持原样走存在性检查
    if !is_url(&target) {
        if let Some(resolved) = resolve_bare_name(&target) {
            target = resolved;
        }
    }

    if !is_url(&target) && !Path::new(&target).exists() {
        return RunResult {
            success: false,
            error: RunError::FileNotFound,
            error_message: get_error_message(&RunError::FileNotFound),
        };
    }

    // .lnk 解析（P1-8）：解析快捷方式真实目标/参数/工作目录；失败则回退
    // ShellExecute 自身处理 .lnk。仅对存在且以 .lnk 结尾的本地目标解析。
    let resolved = resolve_link_if_lnk(&target);
    let final_target: &str = resolved
        .as_ref()
        .map(|r| r.0.as_str())
        .unwrap_or(&target);

    // 参数模板：快捷方式参数优先（覆盖项目参数），否则项目参数（先展开 mp/mr/env，
    // 搜索参数延迟到注入检查通过后再替换）。
    let args_template: Option<String> = resolved
        .as_ref()
        .and_then(|r| r.1.clone())
        .or_else(|| {
            if item.arguments.is_empty() {
                None
            } else {
                Some(expand_pre(&item.arguments, &base_target))
            }
        });

    // %so% 注入防护（P0-1）：shell 型目标/脚本宿主将搜索参数替换进任意字段时，
    // 搜索参数含 shell 元字符则拒绝执行，防止 cmd /c %so% 类配置被注入。
    // 检查覆盖 target/arguments/working_dir 三个可含 %so% 的字段（%so% 非环境变量，
    // expand_pre 不消耗它，故检查原始字段等价于检查展开前形态）。
    let so_in_fields = args_template
        .as_ref()
        .map(|t| t.contains("%so%"))
        .unwrap_or(false)
        || item.target.contains("%so%")
        || item.working_dir.contains("%so%");
    if so_in_fields
        && matches!(
            shell_kind(final_target),
            ShellKind::Cmd | ShellKind::PowerShell | ShellKind::ScriptHost
        )
        && !is_search_param_safe(search_param)
    {
        log::warn!(
            "Run rejected: shell target {} with %so% and unsafe search param",
            final_target
        );
        return RunResult {
            success: false,
            error: RunError::UnsafeSearchParam,
            error_message: get_error_message(&RunError::UnsafeSearchParam),
        };
    }
    let final_args = args_template.map(|t| expand_search(&t, search_param));
    let final_dir = resolved
        .as_ref()
        .and_then(|r| r.2.clone())
        .or_else(|| {
            if item.working_dir.is_empty() {
                None
            } else {
                Some(expand_search(
                    &expand_pre(&item.working_dir, &base_target),
                    search_param,
                ))
            }
        });

    shell_execute(final_target, final_args.as_deref(), final_dir.as_deref(), admin)
}

/// shell 型目标判定：%so% 注入防护仅对会二次解释参数的解释器生效。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Cmd,
    PowerShell,
    /// 会二次解释参数的脚本宿主：mshta/wscript/cscript 及常见脚本扩展名。
    /// %so% 注入防护必须覆盖它们，否则 `mshta javascript:...` 类 payload 可绕过。
    ScriptHost,
    Other,
}

pub fn shell_kind(target: &str) -> ShellKind {
    let name = target.to_lowercase();
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(&name);
    if file_name == "cmd.exe"
        || file_name == "cmd"
        || file_name.ends_with(".bat")
        || file_name.ends_with(".cmd")
    {
        ShellKind::Cmd
    } else if file_name == "powershell.exe"
        || file_name == "pwsh.exe"
        || file_name.ends_with(".ps1")
    {
        ShellKind::PowerShell
    } else if file_name == "mshta.exe"
        || file_name == "mshta"
        || file_name == "wscript.exe"
        || file_name == "wscript"
        || file_name == "cscript.exe"
        || file_name == "cscript"
        || file_name.ends_with(".vbs")
        || file_name.ends_with(".js")
        || file_name.ends_with(".jse")
        || file_name.ends_with(".hta")
        || file_name.ends_with(".wsf")
        || file_name.ends_with(".wsh")
    {
        ShellKind::ScriptHost
    } else {
        ShellKind::Other
    }
}

/// 搜索参数是否可安全替换进 shell 型目标的命令行（P0-1）。
pub fn is_search_param_safe(param: &str) -> bool {
    const UNSAFE: &[char] = &[
        '&', '|', '<', '>', '^', '(', ')', '%', '!', '"', ';', '$', '`', '\r', '\n',
    ];
    !param.chars().any(|c| UNSAFE.contains(&c))
}

/// 解析 .lnk 目标（跨平台入口）：仅对存在且以 .lnk 结尾的本地目标解析，
/// 非 Windows 或解析失败返回 None（回退 ShellExecute 自身处理 .lnk）。
fn resolve_link_if_lnk(target: &str) -> Option<(String, Option<String>, Option<String>)> {
    #[cfg(windows)]
    {
        if !is_url(target) && target.to_lowercase().ends_with(".lnk") {
            resolve_lnk(target)
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        None
    }
}

/// 解析 Windows 快捷方式（.lnk）为真实目标/参数/工作目录。
/// 使用 IShellLinkW + IPersistFile COM 接口。
#[cfg(windows)]
fn resolve_lnk(path: &str) -> Option<(String, Option<String>, Option<String>)> {
    use windows::core::Interface;
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::IShellLinkW;

    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return None;
        }
        let result = (|| {
            // CLSID_ShellLink = {00021401-0000-0000-C000-000000000046}（windows 0.57 未导出该常量，用 GUID 字面量）
            let clsid = windows::core::GUID::from_u128(0x0002140100000000C000000000000046);
            let link: IShellLinkW =
                CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER).ok()?;
            let persist: IPersistFile = link.cast().ok()?;
            let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            persist
                .Load(windows::core::PCWSTR(path_wide.as_ptr()), STGM_READ)
                .ok()?;

            let mut fd = WIN32_FIND_DATAW::default();
            let target = read_shell_link_string(|buf| {
                let _ = link.GetPath(buf, std::ptr::addr_of_mut!(fd), 0);
            });
            let target = match target {
                Some(t) if !t.is_empty() => t,
                _ => return None,
            };

            let args = read_shell_link_string(|buf| {
                let _ = link.GetArguments(buf);
            });
            let args = match args {
                Some(a) if !a.is_empty() => Some(a),
                _ => None,
            };

            let dir = read_shell_link_string(|buf| {
                let _ = link.GetWorkingDirectory(buf);
            });
            let dir = match dir {
                Some(d) if !d.is_empty() => Some(d),
                _ => None,
            };

            Some((target, args, dir))
        })();
        CoUninitialize();
        result
    }
}

/// 动态读取 IShellLinkW 字符串成员（P1-5）：缓冲 1024→65536 逐档扩容，
/// 以首个 NUL 为界取完整值，避免定长 1024 缓冲静默截断长路径/参数。
#[cfg(windows)]
fn read_shell_link_string(mut read: impl FnMut(&mut [u16])) -> Option<String> {
    const CAPS: [usize; 4] = [1024, 4096, 16384, 65536];
    for cap in CAPS {
        let mut buf = vec![0u16; cap];
        read(&mut buf);
        match buf.iter().position(|&c| c == 0) {
            Some(end) => return Some(String::from_utf16_lossy(&buf[..end])),
            // 满缓冲无 NUL：最后一档记警告返回截断值，否则扩容重试
            None if cap == CAPS[CAPS.len() - 1] => {
                log::warn!(
                    "Shell link string exceeded {} chars, truncated",
                    cap
                );
                return Some(String::from_utf16_lossy(&buf));
            }
            None => {}
        }
    }
    None
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

        if success.is_ok() {
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
                    error: RunError::Cancelled,
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

#[cfg(windows)]
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
        // 两遍法（P1-4）：先传 None 取所需长度，再按精确长度分配，避免硬编码 32768 截断
        let needed = unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), None) };
        if needed == 0 {
            return input.to_string();
        }
        let mut buf: Vec<u16> = vec![0u16; needed as usize];
        let written =
            unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(buf.as_mut_slice())) };
        if written == 0 {
            return input.to_string();
        }
        // 第二次调用（lpdst 非空）的返回值是否含 NUL 在 Windows 文档与实测间不一致，
        // 稳健处理：截断到 written 后去除尾随 NUL。
        buf.truncate(written as usize);
        let mut s = String::from_utf16_lossy(&buf);
        while s.ends_with('\0') {
            s.pop();
        }
        s
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

fn url_encode(input: &str) -> String {
    // URL 标准要求 UTF-8 字节百分号编码（C++ 版按 UTF-16 码元编码是缺陷，此处有意修正）
    let mut encoded = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_expand_variables_mp_mr() {
        let out = expand_variables("%mp%|%mr%", "C:\\Tools\\app.exe", "");
        assert!(out.contains("C:\\Tools"));
        assert!(out.contains("C:"));
    }

    #[test]
    fn test_expand_variables_so() {
        let out = expand_variables("run %so%", "C:\\app.exe", "query");
        assert_eq!(out, "run query");
    }

    #[test]
    fn test_expand_variables_so_url_encodes_space_as_plus() {
        let out = expand_variables("%so-url%", "C:\\app.exe", "a b");
        assert_eq!(out, "a+b");
    }

    #[test]
    fn test_expand_variables_no_marker() {
        let out = expand_variables("plain text", "C:\\app.exe", "q");
        assert_eq!(out, "plain text");
    }

    #[test]
    fn test_url_encode_chars() {
        assert_eq!(url_encode("A1-_~."), "A1-_~.");
        assert_eq!(url_encode(" "), "+");
        // UTF-8 百分号编码（非 ASCII 逐字节 %XX）
        assert_eq!(url_encode("中"), "%E4%B8%AD");
        assert_eq!(url_encode("é"), "%C3%A9");
    }

    #[test]
    fn test_get_error_message() {
        assert_eq!(get_error_message(&RunError::FileNotFound), "找不到指定的文件");
        assert_eq!(get_error_message(&RunError::None), "成功");
    }

    #[test]
    fn test_expand_search_single_pass_no_reexpand() {
        // 搜索文本中的字面量 %so% 不被二次展开（P0-1 单遍替换）
        let out = expand_variables("x %so% y", "C:\\app.exe", "%so%");
        assert_eq!(out, "x %so% y");
        // %so-url% 编码结果中不含标记，且搜索文本内的字面量 %so% 保留
        let out = expand_variables("%so-url%", "C:\\app.exe", "a %so% b");
        assert_eq!(out, "a+%25so%25+b");
    }

    #[test]
    fn test_expand_search_env_not_applied_to_search_text() {
        // 环境变量展开先于搜索参数替换：搜索文本中的 %VAR% 保持原样（P0-1）
        let out = expand_variables("run %so%", "C:\\app.exe", "%TEMP%");
        assert_eq!(out, "run %TEMP%");
    }

    #[test]
    fn test_shell_kind_detection() {
        assert_eq!(shell_kind("C:\\Windows\\System32\\cmd.exe"), ShellKind::Cmd);
        assert_eq!(shell_kind("C:\\Windows\\System32\\CMD.EXE"), ShellKind::Cmd);
        assert_eq!(shell_kind("C:\\x\\build.bat"), ShellKind::Cmd);
        assert_eq!(shell_kind("C:\\x\\run.CMD"), ShellKind::Cmd);
        assert_eq!(
            shell_kind("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
            ShellKind::PowerShell
        );
        assert_eq!(shell_kind("C:\\x\\pwsh.exe"), ShellKind::PowerShell);
        assert_eq!(shell_kind("C:\\x\\script.ps1"), ShellKind::PowerShell);
        // 脚本宿主（P1-01）：mshta/wscript/cscript + 常见脚本扩展名
        assert_eq!(shell_kind("C:\\Windows\\System32\\mshta.exe"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\Windows\\System32\\wscript.exe"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\Windows\\System32\\cscript.exe"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.vbs"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.js"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.jse"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.hta"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.wsf"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\evil.wsh"), ShellKind::ScriptHost);
        assert_eq!(shell_kind("C:\\x\\app.exe"), ShellKind::Other);
        assert_eq!(shell_kind("notepad"), ShellKind::Other);
    }

    #[test]
    fn test_is_search_param_safe() {
        assert!(!is_search_param_safe("a&b"));
        assert!(!is_search_param_safe("a|b"));
        assert!(!is_search_param_safe("50% off"));
        assert!(!is_search_param_safe("a;rm"));
        assert!(!is_search_param_safe("a\nb"));
        assert!(is_search_param_safe("hello world"));
        assert!(is_search_param_safe("中文搜索"));
    }

    #[test]
    #[cfg(windows)]
    fn test_expand_pre_expands_env_vars() {
        // 两遍法（P1-4）：真实环境变量展开成功
        let out = expand_pre("%TEMP%", "C:\\app.exe");
        assert!(!out.is_empty());
        assert_ne!(out, "%TEMP%");
    }

    #[test]
    #[cfg(windows)]
    fn test_execute_rejects_unsafe_search_for_cmd() {
        // cmd.exe + /c %so% + 危险搜索参数 → 拒绝且不执行（P0-1）
        let item = Item {
            id: "t".into(),
            name: "t".into(),
            target: "C:\\Windows\\System32\\cmd.exe".into(),
            arguments: "/c echo %so%".into(),
            ..Default::default()
        };
        let r = run(&item, "a&b");
        assert!(!r.success);
        assert_eq!(r.error, RunError::UnsafeSearchParam);
    }

    #[test]
    #[cfg(windows)]
    fn test_execute_rejects_unsafe_search_for_batch() {
        let dir = std::env::temp_dir();
        let bat = dir.join(format!("gorun_test_batch_{}.bat", std::process::id()));
        std::fs::write(&bat, "@echo off\n").unwrap();
        let item = Item {
            id: "t".into(),
            name: "t".into(),
            target: bat.to_string_lossy().to_string(),
            arguments: "%so%".into(),
            ..Default::default()
        };
        let r = run(&item, "a&b");
        assert!(!r.success);
        assert_eq!(r.error, RunError::UnsafeSearchParam);
        let _ = std::fs::remove_file(&bat);
    }

    #[test]
    fn test_find_in_path_dirs_exact_and_ext() {
        let dir = std::env::temp_dir().join(format!("gorun_path_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let exact = dir.join("mytool");
        std::fs::write(&exact, b"x").unwrap();
        let exe = dir.join("run.exe");
        std::fs::write(&exe, b"x").unwrap();
        let dirs = vec![dir.clone()];
        // 精确名命中
        assert_eq!(find_in_path_dirs("mytool", &dirs, &[]), Some(dir.join("mytool")));
        // 无扩展名输入按后缀命中（Windows 大小写不敏感用 .EXE 验证；Linux 大小写敏感用 .exe）
        let ext = if cfg!(windows) { ".EXE" } else { ".exe" };
        let exts = vec![ext.to_string()];
        let got = find_in_path_dirs("run", &dirs, &exts).unwrap();
        assert_eq!(
            got.file_name().unwrap().to_string_lossy().to_lowercase(),
            "run.exe"
        );
        // 带扩展名输入不做后缀拼接
        assert_eq!(
            find_in_path_dirs("run.exe", &dirs, &exts),
            Some(dir.join("run.exe"))
        );
        // 未命中
        assert_eq!(find_in_path_dirs("nope", &dirs, &exts), None);
        assert_eq!(find_in_path_dirs("", &dirs, &exts), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_bare_name_rejects_paths() {
        // 含分隔符的输入不进入 PATH 搜索
        assert_eq!(resolve_bare_name("C:\\x\\a.exe"), None);
        assert_eq!(resolve_bare_name("rel/a"), None);
    }

    #[test]
    fn test_run_all_returns_per_item_results() {
        // 逐项结果条数与输入对齐（失败项亦计入，便于调用方统计成功/失败）
        let items = vec![
            Item { id: "a".into(), name: "A".into(), target: "Z:\\__nonexistent__.exe".into(), ..Default::default() },
            Item { id: "b".into(), name: "B".into(), target: "Z:\\__nonexistent__.exe".into(), ..Default::default() },
            Item { id: "c".into(), name: "C".into(), target: "Z:\\__nonexistent__.exe".into(), ..Default::default() },
        ];
        let results = run_all(&items, "q");
        assert_eq!(results.len(), items.len());
    }
}
