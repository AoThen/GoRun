//! 窗口几何/定位（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::MainWindow;
use slint::ComponentHandle;

pub fn save_window_geometry(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    save_window_geometry_with(ui, state, false)
}

/// 立即落盘窗口几何（close/退出路径用，跳过去抖）。
pub fn flush_window_geometry(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    save_window_geometry_with(ui, state, true)
}

/// 去抖窗口（几何变化后多久才写盘）。拖动/缩放期间每 500ms 只更新内存，
/// 停下超过该时长才落盘，避免高频 I/O 冲掉备份链里的数据快照。
pub const GEOMETRY_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(2000);

fn save_window_geometry_with(
    ui: &MainWindow,
    state: &Rc<RefCell<AppState>>,
    force: bool,
) {
    // 必须在事件循环内/窗口销毁前调用；position()/size() 返回物理像素（与 C++ GetWindowRect 一致）
    let pos = ui.window().position();
    // P2-04：最小化时 GetWindowRect 返回约 (-32000,-32000)，直接落盘会污染配置。
    // 双保险：slint 公开 is_minimized() + 坐标哨兵检查（正常坐标远不到 -10000）。
    if ui.window().is_minimized() || pos.x < -10000 || pos.y < -10000 {
        return;
    }
    let size = ui.window().size();
    let w = size.width as i32;
    let h = size.height as i32;
    let (cx, cy, cw, ch) = {
        let st = state.borrow();
        let c = &st.manager.config().config;
        (c.window_x, c.window_y, c.window_width, c.window_height)
    };
    if cx == pos.x && cy == pos.y && cw == w && ch == h {
        return;
    }
    let mut state = state.borrow_mut();
    // P1-2：窗口几何属于高频非关键写，使用 storage.save() 直接落盘，
    // 避免每次拖动/缩放触发 save_with_backup → rotate_backups 产生频繁文件复制。
    let cfg = &mut state.manager.config_mut().config;
    cfg.window_x = pos.x;
    cfg.window_y = pos.y;
    cfg.window_width = w;
    cfg.window_height = h;
    state.geometry_dirty = true;
    state.last_geometry_change = Some(std::time::Instant::now());
    if force {
        state.flush_geometry();
    }
}

/// 显示窗口时定位到鼠标位置（跟随鼠标模式）。
pub fn position_at_cursor(ui: &MainWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_ok() } {
            // P2-04：GetCursorPos 与 set_position 均为物理像素语义，
            // 直接用物理尺寸居中，避免 DPI≠100% 时把逻辑宽高混入物理坐标造成偏移。
            let size = ui.window().size();
            let w = (size.width as f32).max(1.0);
            let h = (size.height as f32).max(1.0);
            let x = (pt.x as f32 - w * 0.5).max(0.0) as i32;
            let y = (pt.y as f32 - h * 0.5).max(0.0) as i32;
            use slint::PhysicalPosition;
            ui.window().set_position(PhysicalPosition::new(x, y));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = ui;
    }
}

#[cfg(any(windows, test))]
/// 右键菜单宽度（逻辑像素，与 context_menu.slint 的 width:160px 对齐）。
pub(crate) const MENU_WIDTH: f32 = 160.0;
#[cfg(any(windows, test))]
/// 菜单项高度（逻辑像素，与 context_menu.slint 的 28px 对齐）。
pub(crate) const MENU_ITEM_HEIGHT: f32 = 28.0;
#[cfg(any(windows, test))]
/// 菜单上下 padding 合计（context_menu.slint VerticalBox padding:4px ×2）。
pub(crate) const MENU_PADDING: f32 = 8.0;

/// 计算右键菜单锚点（窗口内逻辑坐标），并 clamp 到窗口内防溢出。
/// 纯逻辑（无平台依赖），可单测；Windows 分支用物理像素换算后调用。
#[cfg(any(windows, test))]
pub(crate) fn clamp_menu_pos(
    cursor_x: i32,
    cursor_y: i32,
    win_pos_x: i32,
    win_pos_y: i32,
    scale_factor: f32,
    win_w_logical: f32,
    win_h_logical: f32,
    item_count: usize,
) -> (f32, f32) {
    let menu_h = item_count as f32 * MENU_ITEM_HEIGHT + MENU_PADDING;
    let lx = ((cursor_x - win_pos_x) as f32 / scale_factor)
        .clamp(0.0, (win_w_logical - MENU_WIDTH).max(0.0));
    let ly = ((cursor_y - win_pos_y) as f32 / scale_factor)
        .clamp(0.0, (win_h_logical - menu_h).max(0.0));
    (lx, ly)
}

/// 定位右键菜单到鼠标处（换算窗口内逻辑坐标并 clamp 到窗口内防溢出；
/// 高度 clamp 按菜单项数动态计算，建议③）。
pub fn position_menu_at_cursor(ui: &MainWindow, x: f32, y: f32, item_count: usize) {
    #[cfg(windows)]
    {
        let _ = (x, y);
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_ok() } {
            let sf = ui.window().scale_factor();
            let size = ui.window().size();
            let pos = ui.window().position();
            // v2：菜单项高 22→28px（context_menu.slint），高度系数同步，否则底缘 clamp 失效弹层被裁切
            let (lx, ly) = clamp_menu_pos(
                pt.x, pt.y, pos.x, pos.y, sf,
                size.width as f32 / sf, size.height as f32 / sf, item_count,
            );
            ui.set_context_menu_x(lx);
            ui.set_context_menu_y(ly);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = item_count;
        ui.set_context_menu_x(x);
        ui.set_context_menu_y(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 窗口在屏幕 (100,200)，100% DPI，逻辑 800×600
    const WX: i32 = 100;
    const WY: i32 = 200;
    const SF: f32 = 1.0;
    const WW: f32 = 800.0;
    const WH: f32 = 600.0;

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "期望 {} 实际 {}", b, a);
    }

    #[test]
    fn test_clamp_menu_pos_centered() {
        // 光标在窗口内 (500,400) → 菜单左上角约 (500-100, 400-200) = (400,200)
        let (lx, ly) = clamp_menu_pos(600, 400, WX, WY, SF, WW, WH, 5);
        assert_close(lx, 500.0);
        assert_close(ly, 200.0);
    }

    #[test]
    fn test_clamp_menu_pos_top_left_clamped() {
        // 光标在窗口左上角外 → clamp 到 (0,0)
        let (lx, ly) = clamp_menu_pos(50, 100, WX, WY, SF, WW, WH, 5);
        assert_close(lx, 0.0);
        assert_close(ly, 0.0);
    }

    #[test]
    fn test_clamp_menu_pos_right_edge_clamped() {
        // 光标贴近右缘 → lx clamp 到 win_w - 160
        let (lx, ly) = clamp_menu_pos(WX + 900, 400, WX, WY, SF, WW, WH, 5);
        assert_close(lx, WW - MENU_WIDTH);
        assert_close(ly, 200.0);
    }

    #[test]
    fn test_clamp_menu_pos_bottom_edge_clamped() {
        // 光标贴近底缘 → ly clamp 到 win_h - menu_h；5 项高 = 5*28+8 = 148
        let (lx, ly) = clamp_menu_pos(600, WY + 900, WX, WY, SF, WW, WH, 5);
        assert_close(lx, 500.0);
        assert_close(ly, WH - (5.0 * MENU_ITEM_HEIGHT + MENU_PADDING));
    }

    #[test]
    fn test_clamp_menu_pos_tall_menu_fits_zero() {
        // 菜单高于窗口（100 项 = 100*28+8 = 2808 > 600）→ ly clamp 到 0
        let (lx, ly) = clamp_menu_pos(600, 400, WX, WY, SF, WW, WH, 100);
        assert_close(ly, 0.0);
        assert_close(lx, 500.0);
    }

    #[test]
    fn test_clamp_menu_pos_dpi_scale() {
        // 150% DPI：物理窗口尺寸 1200×900 → 逻辑 800×600
        let (lx, ly) = clamp_menu_pos(750, 700, 150, 300, 1.5, 1200.0, 900.0, 5);
        // (750-150)/1.5 = 400； (700-300)/1.5 = 266.67（未越界）
        assert_close(lx, 400.0);
        assert_close(ly, 266.6667);
    }

    #[test]
    fn test_clamp_menu_pos_menu_width_const() {
        assert_eq!(MENU_WIDTH, 160.0);
        assert_eq!(MENU_ITEM_HEIGHT, 28.0);
        assert_eq!(MENU_PADDING, 8.0);
    }
}
