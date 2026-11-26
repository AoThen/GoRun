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

/// 去抖窗口（几何变化后多久才写盘）。拖动/缩放期间仅由 500ms 周期 Timer
/// 更新内存配置，停下超过本窗口（2s）才真正落盘，避免高频 I/O 冲掉备份链
/// 里的数据快照。
pub const GEOMETRY_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(2000);

/// H4：最大化状态持久化键（存于 extra_config，布尔 "1"/"0"，经 P3-12 的
/// extra_get_bool/extra_set_bool 读写，零 schema 变更）。导入配置时随非会话
/// 偏好一起被丢弃，与窗口几何整体替换语义一致。
pub const WINDOW_MAXIMIZED_KEY: &str = "windowMaximized";

/// H4 几何保存决策（纯逻辑，单测覆盖）：将"是否落盘、落什么"与 UI 调用解耦。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GeometrySaveAction {
    /// 最小化/离屏哨兵或无变化：整体跳过
    Skip,
    /// 最大化态：仅维护最大化标志，坐标不落盘（保留最大化前的普通几何）
    FlagOnly,
    /// 非最大化态：维护最大化标志（清零）+ 坐标尺寸落盘
    FullUpdate,
}

/// H4 决策核心。`maximized/minimized/offscreen` 取当前窗口状态；`cur_flag`
/// 为已持久化的最大化标志；`xywh_changed` 为坐标尺寸是否变化。
/// 关键点：最大化时坐标不落盘（否则最大化坐标会污染还原几何）；
/// "最大化→还原（未移动）"坐标无变化，标志清零不能被坐标相等早退吞掉，
/// 故标志 diff 判定独立于坐标比较。
// 决策函数集中窗口状态位，5 个 bool 触发 clippy fn_params_excessive_bools；
// 语义直观（逐位命名），重构为枚举收益低，按仓库 util.rs 先例 allow。
#[allow(clippy::fn_params_excessive_bools)]
pub(crate) fn geometry_save_decision(
    maximized: bool,
    minimized: bool,
    offscreen: bool,
    cur_flag: bool,
    xywh_changed: bool,
) -> GeometrySaveAction {
    if minimized || offscreen {
        return GeometrySaveAction::Skip;
    }
    if maximized {
        // 最大化：坐标不落盘（保留普通还原几何），仅维护标志
        return if cur_flag {
            GeometrySaveAction::Skip
        } else {
            GeometrySaveAction::FlagOnly
        };
    }
    // 非最大化：标志清零 + 坐标按变化落盘
    if cur_flag || xywh_changed {
        GeometrySaveAction::FullUpdate
    } else {
        GeometrySaveAction::Skip
    }
}

fn save_window_geometry_with(ui: &MainWindow, state: &Rc<RefCell<AppState>>, force: bool) {
    // 必须在事件循环内/窗口销毁前调用；position()/size() 返回物理像素
    let pos = ui.window().position();
    let size = ui.window().size();
    let w = size.width as i32;
    let h = size.height as i32;
    // P2-04：最小化时 GetWindowRect 返回约 (-32000,-32000)，直接落盘会污染配置。
    // 双保险：slint 公开 is_minimized() + 坐标哨兵检查（正常坐标远不到 -10000）。
    let offscreen = pos.x < -10000 || pos.y < -10000;
    let maximized = ui.window().is_maximized();
    let (cx, cy, cw, ch) = {
        let st = state.borrow();
        let c = &st.manager.config().config;
        (c.window_x, c.window_y, c.window_width, c.window_height)
    };
    let cur_flag = state
        .borrow()
        .manager
        .config()
        .extra_get_bool(WINDOW_MAXIMIZED_KEY);
    let xywh_changed = cx != pos.x || cy != pos.y || cw != w || ch != h;
    match geometry_save_decision(
        maximized,
        ui.window().is_minimized(),
        offscreen,
        cur_flag,
        xywh_changed,
    ) {
        GeometrySaveAction::Skip => (),
        GeometrySaveAction::FlagOnly => {
            // H4：最大化仅维护标志（坐标不落盘，保留普通还原几何）；
            // 走几何去抖链（非轮转写），与坐标写同管线。
            let mut st = state.borrow_mut();
            st.manager
                .config_mut()
                .extra_set_bool(WINDOW_MAXIMIZED_KEY, true);
            st.geometry_dirty = true;
            st.last_geometry_change = Some(std::time::Instant::now());
            if force {
                st.flush_geometry();
            }
        }
        GeometrySaveAction::FullUpdate => {
            // 诊断留痕：窗口尺寸/位置变化（定位"宽度无法拖动"类问题——若拖动边缘后
            // 日志无对应变化，说明尺寸事件未到达 slint 层，需查 winit/OS 命中测试）
            log::debug!(
                "Window geometry change: pos ({},{}) -> ({},{}), size {}x{} -> {}x{}",
                cx,
                cy,
                pos.x,
                pos.y,
                cw,
                ch,
                w,
                h
            );
            let mut st = state.borrow_mut();
            if cur_flag {
                // H4：还原（非最大化）时清除持久化标志（先于坐标写，避免两次
                // config_mut 借用冲突）
                st.manager
                    .config_mut()
                    .extra_set_bool(WINDOW_MAXIMIZED_KEY, false);
            }
            // P1-2：窗口几何属于高频非关键写，使用 storage.save() 直接落盘，
            // 避免每次拖动/缩放触发 save_with_backup → rotate_backups 产生频繁文件复制。
            let cfg = &mut st.manager.config_mut().config;
            cfg.window_x = pos.x;
            cfg.window_y = pos.y;
            cfg.window_width = w;
            cfg.window_height = h;
            st.geometry_dirty = true;
            st.last_geometry_change = Some(std::time::Instant::now());
            if force {
                st.flush_geometry();
            }
        }
    }
}

/// 窗口就绪探测间隔与上限：winit 窗口在事件循环 Resumed/ensure_window 时才真正创建，
/// 此前 `window_handle()` 返回 Err，需等待。
const WINDOW_READY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const WINDOW_READY_MAX_RETRIES: u32 = 100;

type WindowReadyCallback = Box<dyn FnOnce(&MainWindow) + 'static>;

/// 等待 winit 窗口真正创建后执行 `on_ready`（拖放装配/标题栏图标/尺寸恢复共用）。
///
/// 以 100ms 周期重试探测窗口句柄，就绪即回调；超过上限（10s）放弃并记录错误。
/// 用 `Timer::single_shot` 链式探测：SingleShot 定时器不能在自身回调里 restart
/// 同一句柄（slint 会在回调后移除槽位，再触发时 panic），每次新建定时器则无此问题。
pub fn when_window_ready(
    ui_weak: slint::Weak<MainWindow>,
    on_ready: impl FnOnce(&MainWindow) + 'static,
) {
    #[cfg(not(windows))]
    {
        let _ = (ui_weak, on_ready);
        log::debug!("when_window_ready: window handle probe is Windows-only");
    }
    #[cfg(windows)]
    {
        let callback = Rc::new(RefCell::new(
            Some(Box::new(on_ready) as WindowReadyCallback),
        ));
        probe_window_ready(ui_weak, callback, 0);
    }
}

#[cfg(windows)]
fn probe_window_ready(
    ui_weak: slint::Weak<MainWindow>,
    on_ready: Rc<RefCell<Option<WindowReadyCallback>>>,
    attempt: u32,
) {
    if let Some(ui) = ui_weak.upgrade() {
        if crate::drop_handler::try_get_hwnd(&ui).is_some() {
            if let Some(on_ready) = on_ready.borrow_mut().take() {
                on_ready(&ui);
            }
            return;
        }
    }
    if attempt >= WINDOW_READY_MAX_RETRIES {
        log::error!(
            "Window not ready after {} retries; deferred setup (size/drop/icon) aborted",
            WINDOW_READY_MAX_RETRIES
        );
        return;
    }
    let next_weak = ui_weak.clone();
    let next_callback = on_ready.clone();
    slint::Timer::single_shot(WINDOW_READY_INTERVAL, move || {
        probe_window_ready(next_weak, next_callback, attempt + 1);
    });
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
/// 参数天然多（光标/窗口/缩放/项数 8 项），分组会改动全部调用点且引入
/// 新结构体、churn 大于收益，故局部抑制并说明理由。
#[cfg(any(windows, test))]
#[expect(
    clippy::too_many_arguments,
    reason = "光标/窗口/缩放/项数 8 个纯逻辑参数，分组得不偿失"
)]
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
                pt.x,
                pt.y,
                pos.x,
                pos.y,
                sf,
                size.width as f32 / sf,
                size.height as f32 / sf,
                item_count,
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

/// 热键唤醒：恢复最小化并把窗口带到前台。
///
/// slint `show()` 只设可见标志，不会恢复最小化，也不保证激活到前台；
/// 热键唤醒需手动 SetForegroundWindow 恢复并激活窗口。
/// Windows 对后台进程设置前台有限制，但热键触发属"最近收到输入"，
/// 权限窗口期内 SetForegroundWindow 可成功。
pub fn bring_to_front(ui: &MainWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
        };
        if let Some(hwnd) = crate::drop_handler::try_get_hwnd(ui) {
            unsafe {
                let h = HWND(hwnd as *mut std::ffi::c_void);
                if IsIconic(h).as_bool() {
                    let _ = ShowWindow(h, SW_RESTORE);
                } else {
                    let _ = ShowWindow(h, SW_SHOW);
                }
                let _ = SetForegroundWindow(h);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = ui;
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

    #[test]
    fn test_geometry_save_decision_maximized_skips_xywh() {
        // H4：最大化时坐标不落盘，仅维护标志（即使坐标有变化）
        assert_eq!(
            geometry_save_decision(true, false, false, false, true),
            GeometrySaveAction::FlagOnly
        );
        // 最大化且标志已一致：整体跳过
        assert_eq!(
            geometry_save_decision(true, false, false, true, false),
            GeometrySaveAction::Skip
        );
    }

    #[test]
    fn test_geometry_save_decision_restore_clears_flag() {
        // H4：最大化→还原（坐标未移动）：坐标相等不能吞掉标志清零
        assert_eq!(
            geometry_save_decision(false, false, false, true, false),
            GeometrySaveAction::FullUpdate
        );
        // 非最大化、标志一致、坐标未变：跳过
        assert_eq!(
            geometry_save_decision(false, false, false, false, false),
            GeometrySaveAction::Skip
        );
        // 非最大化、坐标变化：落盘
        assert_eq!(
            geometry_save_decision(false, false, false, false, true),
            GeometrySaveAction::FullUpdate
        );
    }

    #[test]
    fn test_geometry_save_decision_minimized_offscreen_skip() {
        // H4：最小化/离屏哨兵：无论标志与坐标如何，整体跳过
        assert_eq!(
            geometry_save_decision(true, true, false, false, true),
            GeometrySaveAction::Skip
        );
        assert_eq!(
            geometry_save_decision(false, false, true, false, true),
            GeometrySaveAction::Skip
        );
    }
}
