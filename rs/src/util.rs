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
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        let sw = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let sh = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        sw > 0 && sh > 0 && x < sw && y < sh && (x + w) > 0 && (y + h) > 0
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y, w, h);
        true
    }
}
