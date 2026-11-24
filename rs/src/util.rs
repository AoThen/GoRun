//! 通用工具函数（P2-4 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::localization::Localization;

/// 翻译键值并填充 {0}/%s 占位符（本地化扩展）
pub fn tr_fill(loc: &Rc<RefCell<Localization>>, key: &str, arg: &str) -> String {
    loc.borrow().tr(key).replace("{0}", arg).replace("%s", arg)
}

/// 日志注入防护：折叠控制字符，避免破坏日志行结构。
pub fn sanitize_for_log(s: &str) -> String {
    s.replace('\n', "\\n").replace('\r', "\\r")
}

/// 纯几何判断：窗口矩形（x,y,w,h）是否完全落在虚拟屏（vx,vy,vw,vh）内。
/// 抽出为纯函数以便跨平台单测；Windows 分支读取 metrics 后调用。
/// 非 Windows 下仅被单测引用（window_on_screen 恒 true），故允许 dead_code。
#[cfg_attr(not(windows), allow(dead_code))]
// 8 个几何参数均为独立标量，引入 Rect 结构体会波及全部调用点与测试；
// 保留扁平签名并在此精确抑制（替代文件级 allow，让新告警重新可见）。
#[allow(clippy::too_many_arguments)]
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

/// 窗口物理尺寸超过虚拟屏 80% 时等比缩回（修复历史版本 DPI 漂移留下的超大尺寸）。
pub fn clamp_size_to_virtual_screen(w: i32, h: i32) -> (i32, i32) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        };
        let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        clamp_size_impl(w, h, vw, vh)
    }
    #[cfg(not(windows))]
    {
        (w, h)
    }
}

fn clamp_size_impl(w: i32, h: i32, vw: i32, vh: i32) -> (i32, i32) {
    const LIMIT_RATIO: f64 = 0.8;
    if vw <= 0 || vh <= 0 || w <= 0 || h <= 0 {
        return (w, h);
    }
    let max_w = (vw as f64 * LIMIT_RATIO).floor() as i32;
    let max_h = (vh as f64 * LIMIT_RATIO).floor() as i32;
    let scale = (max_w as f64 / w as f64)
        .min(max_h as f64 / h as f64)
        .min(1.0);
    if scale < 1.0 {
        // 极端超大值缩放后可能向零截断：至少保留 1px，防止请求零尺寸窗口
        let cw = ((w as f64 * scale) as i32).max(1);
        let ch = ((h as f64 * scale) as i32).max(1);
        (cw, ch)
    } else {
        (w, h)
    }
}

/// 查询指定坐标所在显示器的缩放系数（有效 DPI / 96）。
/// 供窗口创建前把"保存的物理尺寸"换算成逻辑尺寸，使首帧即命中目标大小，
/// 消除"先按默认逻辑尺寸显示、再缩到保存尺寸"的启动闪动。
/// 需在 MainWindow::new() 之后调用：winit EventLoop 已建立 Per-Monitor V2 感知，
/// GetDpiForMonitor 才返回真实有效 DPI。失败或数值异常返回 None，
/// 调用方退回"窗口就绪后延迟恢复"的原路径。
pub fn monitor_scale_at(x: i32, y: i32) -> Option<f32> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
        use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
        if monitor.is_invalid() {
            return None;
        }
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }.ok()?;
        // 显示器 x/y DPI 恒等（方形像素），取 x 即可
        scale_from_dpi(dpi_x)
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        None
    }
}

/// DPI → 缩放系数（纯函数便于单测）：96 DPI = 1.0；低于 96 视为异常返回 None。
#[cfg(any(windows, test))]
fn scale_from_dpi(dpi: u32) -> Option<f32> {
    if dpi < 96 {
        return None;
    }
    Some(dpi as f32 / 96.0)
}

/// 对话框逻辑尺寸按屏幕可用范围钳制（P2：小屏/高缩放设备上，固定 500 逻辑 px 高的
/// 编辑窗口物理高度会超出工作区，被系统截断后固定布局的底部按钮行挤出可视区）。
/// `scale` 为窗口当前缩放系数（物理 px = 逻辑 px × scale）。Windows 下按主屏
/// 工作区（已扣任务栏）减去标题栏/边框余量换算逻辑上限后取 min；非 Windows 原样返回。
/// 需在窗口 show 之后调用（此时 scale_factor 才是真实值）。
pub fn clamp_dialog_logical_size(w: f32, h: f32, scale: f32) -> (f32, f32) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXBORDER, SM_CXFULLSCREEN, SM_CYCAPTION, SM_CYFULLSCREEN,
        };
        // 工作区（SM_CX/CYFULLSCREEN）已扣除任务栏，再扣标题栏+双边框余量
        let avail_w = unsafe { GetSystemMetrics(SM_CXFULLSCREEN) };
        let avail_h = unsafe { GetSystemMetrics(SM_CYFULLSCREEN) };
        let chrome_h = unsafe { GetSystemMetrics(SM_CYCAPTION) } + 2 * unsafe { GetSystemMetrics(SM_CXBORDER) };
        clamp_dialog_logical_size_impl(w, h, scale, avail_w, avail_h, chrome_h)
    }
    #[cfg(not(windows))]
    {
        let _ = (w, h, scale);
        (w, h)
    }
}

/// 钳制纯函数（便于单测）：`avail_w/avail_h` 为可用物理高宽（已扣任务栏），
/// `chrome_h` 为标题栏+边框的物理像素总余量；另留 2% 余量吸收取整/度量误差。
fn clamp_dialog_logical_size_impl(
    w: f32,
    h: f32,
    scale: f32,
    avail_w_phys: i32,
    avail_h_phys: i32,
    chrome_h: i32,
) -> (f32, f32) {
    if avail_w_phys <= 0 || avail_h_phys <= 0 || scale <= 0.0 || w <= 0.0 || h <= 0.0 {
        return (w, h);
    }
    let s = scale as f64;
    let max_w = ((avail_w_phys as f64 * 0.98).floor() / s).max(1.0);
    let max_h = (((avail_h_phys - chrome_h).max(1) as f64 * 0.98).floor() / s).max(1.0);
    let cw = (w as f64).min(max_w);
    let ch = (h as f64).min(max_h);
    (cw as f32, ch as f32)
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
        assert_eq!(
            tr_fill(&l, "Status_RunFail", "Notepad"),
            "运行失败：Notepad"
        );
    }

    #[test]
    fn test_tr_fill_percent_s_placeholder() {
        let l = loc("zh-CN");
        assert_eq!(
            tr_fill(&l, "Tip_DelItemFormat", "Notepad"),
            "确定要删除 Notepad 吗？"
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
        assert!(rect_fully_inside_virtual_screen(
            100, 100, 800, 600, VX, VY, VW, VH
        ));
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
            1500, 100, 800, 600, VX, VY, VW, VH
        ));
    }

    #[test]
    fn test_rect_fully_offscreen_rejected() {
        assert!(!rect_fully_inside_virtual_screen(
            3000, 3000, 800, 600, VX, VY, VW, VH
        ));
    }

    #[test]
    fn test_rect_negative_left_offscreen_rejected() {
        // 左上角在虚拟屏左边之外
        assert!(!rect_fully_inside_virtual_screen(
            -50, 100, 800, 600, VX, VY, VW, VH
        ));
    }

    #[test]
    fn test_rect_zero_size_rejected() {
        // 零/负尺寸不恢复（防御）
        assert!(!rect_fully_inside_virtual_screen(
            100, 100, 0, 600, VX, VY, VW, VH
        ));
        assert!(!rect_fully_inside_virtual_screen(
            100, 100, 800, -1, VX, VY, VW, VH
        ));
    }

    #[test]
    fn test_rect_secondary_monitor_negative_coords() {
        // 副屏在左侧（虚拟屏起点为负）：如主屏 (0,0)-(1920,1080)，副屏 (-1280,0)-(0,1080)
        assert!(rect_fully_inside_virtual_screen(
            -1000, 100, 800, 600, -1280, 0, 3200, 1080
        ));
        assert!(!rect_fully_inside_virtual_screen(
            -1000, 100, 800, 600, 0, 0, 1920, 1080
        ));
    }

    #[test]
    fn test_clamp_size_within_limits_unchanged() {
        // 800x600 在 1920x1080 的 80%（1536x864）以内 → 原样返回
        assert_eq!(clamp_size_impl(800, 600, 1920, 1080), (800, 600));
    }

    #[test]
    fn test_clamp_size_height_exceeding_scales_proportionally() {
        // 高度超限：按高度比例等比缩（2880x1620 → 高度缩到 864 → 1536x864）
        assert_eq!(clamp_size_impl(2880, 1620, 1920, 1080), (1536, 864));
    }

    #[test]
    fn test_clamp_size_width_exceeding_scales_proportionally() {
        // 宽度超限：按宽度比例等比缩（3000x600 → 宽度缩到 1536）
        let (w, h) = clamp_size_impl(3000, 600, 1920, 1080);
        assert_eq!(w, 1536);
        assert_eq!(h, 307); // 600*0.512 = 307.2 → 307
    }

    #[test]
    fn test_clamp_size_degenerate_inputs_unchanged() {
        // 屏幕信息异常或尺寸非法时不做缩放
        assert_eq!(clamp_size_impl(800, 600, 0, 1080), (800, 600));
        assert_eq!(clamp_size_impl(800, 600, 1920, -1), (800, 600));
        assert_eq!(clamp_size_impl(0, 600, 1920, 1080), (0, 600));
    }

    #[test]
    fn test_clamp_size_extreme_value_never_zero() {
        // 极端超大值缩放后向零截断 → 至少保留 1px
        assert_eq!(clamp_size_impl(400, 1_000_000_000, 1920, 1080), (1, 864));
    }

    #[test]
    fn test_scale_from_dpi() {
        // 96 DPI = 100% 缩放；常见 Windows 档位 120=125%、144=150%
        assert_eq!(scale_from_dpi(96), Some(1.0));
        assert_eq!(scale_from_dpi(120), Some(1.25));
        assert_eq!(scale_from_dpi(144), Some(1.5));
        // 异常值（<96）不换算，交由调用方退回延迟恢复路径
        assert_eq!(scale_from_dpi(0), None);
        assert_eq!(scale_from_dpi(72), None);
    }

    #[test]
    fn test_clamp_dialog_size_within_limits_unchanged() {
        // 1080p@100%：可用高 1032−25=1007 物理 → 逻辑上限 986，500 不变
        assert_eq!(
            clamp_dialog_logical_size_impl(500.0, 500.0, 1.0, 1920, 1032, 25),
            (500.0, 500.0)
        );
    }

    #[test]
    fn test_clamp_dialog_size_small_screen_scales_down() {
        // 768p@150%（工作区高 720 物理）：可用 695 物理 → ×0.98=681 → /1.5=454 逻辑
        let (w, h) = clamp_dialog_logical_size_impl(500.0, 500.0, 1.5, 1366, 720, 25);
        assert_eq!(w, 500.0); // 宽度上限 1366×0.98/1.5≈892，不裁
        assert_eq!(h, 454.0);
    }

    #[test]
    fn test_clamp_dialog_size_high_dpi_1080p() {
        // 1080p@200%：可用高 1007 物理 → ×0.98=986 → /2=493 逻辑
        let (_, h) = clamp_dialog_logical_size_impl(500.0, 500.0, 2.0, 1920, 1032, 25);
        assert_eq!(h, 493.0);
    }

    #[test]
    fn test_clamp_dialog_size_degenerate_inputs_unchanged() {
        assert_eq!(
            clamp_dialog_logical_size_impl(500.0, 500.0, 0.0, 1920, 1032, 25),
            (500.0, 500.0)
        );
        assert_eq!(
            clamp_dialog_logical_size_impl(500.0, 500.0, 1.0, 0, 1032, 25),
            (500.0, 500.0)
        );
        assert_eq!(
            clamp_dialog_logical_size_impl(500.0, 500.0, 1.0, 1920, -1, 25),
            (500.0, 500.0)
        );
    }

    #[test]
    fn test_clamp_dialog_size_never_below_one() {
        // 极端：扣完装饰余量后至少按 1 物理像素参与换算，结果不低于 1 逻辑 px
        let (_, h) = clamp_dialog_logical_size_impl(500.0, 500.0, 1.0, 1920, 10, 25);
        assert!(h >= 1.0);
    }
}
