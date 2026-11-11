//! 窗口几何/定位（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::MainWindow;
use slint::ComponentHandle;

pub fn save_window_geometry(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
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
    if state.storage.save(state.manager.config()) {
        state.manager.reset_modified();
    } else {
        log::warn!("Failed to save window geometry (will retry on next change)");
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
            const MENU_W: f32 = 160.0;
            let menu_h = item_count as f32 * 22.0 + 8.0;
            let win_w = size.width as f32 / sf;
            let win_h = size.height as f32 / sf;
            let lx = ((pt.x - pos.x) as f32 / sf).clamp(0.0, (win_w - MENU_W).max(0.0));
            let ly = ((pt.y - pos.y) as f32 / sf).clamp(0.0, (win_h - menu_h).max(0.0));
            ui.set_context_menu_x(lx);
            ui.set_context_menu_y(ly);
        }
    }
    #[cfg(not(windows))]
    {
        ui.set_context_menu_x(x);
        ui.set_context_menu_y(y);
    }
}
