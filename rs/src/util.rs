//! 通用工具函数（P2-4 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::localization::Localization;

/// 翻译键值并填充 {0}/%s 占位符（本地化扩展）
pub fn tr_fill(loc: &Rc<RefCell<Localization>>, key: &str, arg: &str) -> String {
    loc.borrow()
        .tr(key)
        .replace("{0}", arg)
        .replace("%s", arg)
}

/// 日志注入防护：折叠控制字符，避免破坏日志行结构。
pub fn sanitize_for_log(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

/// 窗口矩形是否落在屏幕内（P2 #12：离屏窗口不恢复，避免不可见）。
pub fn window_on_screen(x: i32, y: i32, w: i32, h: i32) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        // P2-04：用虚拟桌面 metrics 而非主屏，副屏上的窗口才能被正确判为可恢复。
        let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        vw > 0 && vh > 0 && x >= vx && y >= vy && x < vx + vw && y < vy + vh
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y, w, h);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(lang: &str) -> Rc<RefCell<Localization>> {
        Rc::new(RefCell::new(Localization::new(lang)))
    }

    #[test]
    fn test_tr_fill_brace_placeholder() {
        let l = loc("zh-CN");
        assert_eq!(tr_fill(&l, "Status_RunFail", "Notepad"), "运行失败：Notepad");
    }

    #[test]
    fn test_tr_fill_percent_s_placeholder() {
        let l = loc("zh-CN");
        assert_eq!(
            tr_fill(&l, "Tip_DelItemFormat", "Notepad"),
            "确定要删除 Notepad 吗？"
        );
        let le = loc("en-US");
        assert_eq!(
            tr_fill(&le, "Tip_DelItemFormat", "Notepad"),
            "Are you sure you want to delete Notepad?"
        );
    }

    #[test]
    fn test_tr_fill_missing_key_returns_key_with_placeholder() {
        // 缺键时 tr 返回键本身，再做占位符替换（特征化）
        let l = loc("zh-CN");
        assert_eq!(tr_fill(&l, "MissingKey", "x"), "MissingKey");
        assert_eq!(tr_fill(&l, "Missing{0}Key", "x"), "MissingxKey");
        assert_eq!(tr_fill(&l, "Missing%sKey", "x"), "MissingxKey");
    }

    #[test]
    fn test_tr_fill_replaces_both_placeholders() {
        let l = loc("zh-CN");
        assert_eq!(tr_fill(&l, "{0} and %s", "v"), "v and v");
    }

    #[test]
    fn test_sanitize_for_log_folds_newlines() {
        assert_eq!(sanitize_for_log("a\nb"), "a\\nb");
        assert_eq!(sanitize_for_log("a\rb"), "a\\rb");
        assert_eq!(sanitize_for_log("a\n\rb"), "a\\n\\rb");
        assert_eq!(sanitize_for_log("normal"), "normal");
    }

    #[test]
    fn test_sanitize_for_log_keeps_tab_and_other_control() {
        // 与 logger::sanitize 区别：util 版本仅折叠 \n \r，不动制表符/其他控制字符
        assert_eq!(sanitize_for_log("tab\there"), "tab\there");
        assert_eq!(sanitize_for_log("a\tb"), "a\tb");
    }
}
