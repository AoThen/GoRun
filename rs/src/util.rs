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

/// 纯几何判断：窗口矩形（x,y,w,h）是否完全落在虚拟屏（vx,vy,vw,vh）内。
/// 抽出为纯函数以便跨平台单测；Windows 分支读取 metrics 后调用。
/// 非 Windows 下仅被单测引用（window_on_screen 恒 true），故允许 dead_code。
#[cfg_attr(not(windows), allow(dead_code))]
fn rect_fully_inside_virtual_screen(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    vx: i32,
    vy: i32,
    vw: i32,
    vh: i32,
) -> bool {
    vw > 0 && vh > 0 && w > 0 && h > 0 && x >= vx && y >= vy && x + w <= vx + vw && y + h <= vy + vh
}

/// 窗口矩形是否完全落在屏幕内（P2 #12：离屏窗口不恢复，避免不可见）。
/// 同时校验左上角与右下角，防止窗口大部分在屏幕外仍被恢复。
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
        rect_fully_inside_virtual_screen(x, y, w, h, vx, vy, vw, vh)
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

    // 虚拟屏几何判断（P2 #12 边界）：虚拟屏 (0,0)-(1920,1080)
    const VX: i32 = 0;
    const VY: i32 = 0;
    const VW: i32 = 1920;
    const VH: i32 = 1080;

    #[test]
    fn test_rect_fully_inside_normal() {
        assert!(rect_fully_inside_virtual_screen(100, 100, 800, 600, VX, VY, VW, VH));
    }

    #[test]
    fn test_rect_fully_inside_right_bottom_boundary() {
        // 右下角恰好贴虚拟屏右下边界 → 允许（含边界）
        assert!(rect_fully_inside_virtual_screen(
            1920 - 800,
            1080 - 600,
            800,
            600,
            VX,
            VY,
            VW,
            VH
        ));
    }

    #[test]
    fn test_rect_partially_offscreen_rejected() {
        // 窗口右边超出屏幕 → 拒绝（原实现只查左上角会误恢复）
        assert!(!rect_fully_inside_virtual_screen(
            1500,
            100,
            800,
            600,
            VX,
            VY,
            VW,
            VH
        ));
    }

    #[test]
    fn test_rect_fully_offscreen_rejected() {
        assert!(!rect_fully_inside_virtual_screen(
            3000,
            3000,
            800,
            600,
            VX,
            VY,
            VW,
            VH
        ));
    }

    #[test]
    fn test_rect_negative_left_offscreen_rejected() {
        // 左上角在虚拟屏左边之外
        assert!(!rect_fully_inside_virtual_screen(-50, 100, 800, 600, VX, VY, VW, VH));
    }

    #[test]
    fn test_rect_zero_size_rejected() {
        // 零/负尺寸不恢复（防御）
        assert!(!rect_fully_inside_virtual_screen(100, 100, 0, 600, VX, VY, VW, VH));
        assert!(!rect_fully_inside_virtual_screen(100, 100, 800, -1, VX, VY, VW, VH));
    }

    #[test]
    fn test_rect_secondary_monitor_negative_coords() {
        // 副屏在左侧（虚拟屏起点为负）：如主屏 (0,0)-(1920,1080)，副屏 (-1280,0)-(0,1080)
        assert!(rect_fully_inside_virtual_screen(
            -1000,
            100,
            800,
            600,
            -1280,
            0,
            3200,
            1080
        ));
        assert!(!rect_fully_inside_virtual_screen(
            -1000,
            100,
            800,
            600,
            0,
            0,
            1920,
            1080
        ));
    }
}
