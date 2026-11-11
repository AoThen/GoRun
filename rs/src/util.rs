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
