//! 事件处理器模块（P1-3 自 main.rs 拆出）
//!
//! 按职责分组：
//! - `dialogs`：编辑对话框辅助（分类新增按钮）
//! - `edit_dialog`：新建/编辑项目对话框的共享装配（P2-5 统一双路径）
//! - `wire_handler`：主窗口事件回调注册（D3 自 main.rs 迁入）

pub mod config_io;
pub mod dialogs;
pub mod edit_dialog;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app_state::{AppState, RunOutcome};
use crate::localization::Localization;
use crate::model::{Category, ViewType};
use crate::{clipboard, fileops, geometry, hotkey, model, ui_sync, util, MainWindow};
use slint::ComponentHandle;

/// 应用背景图到 UI 并持久化路径。解码失败返回 false（调用方 Toast 提示，UI 保持现状）。
/// 菜单/弹层内容总高：项 28px、分隔线("-") 1px、上下 padding 4px×2——供挂载处限高滚动
fn menu_content_height(items: &[slint::SharedString]) -> f32 {
    items
        .iter()
        .map(|s| if s.as_str() == "-" { 1.0 } else { 28.0 })
        .sum::<f32>()
        + 8.0
}

/// 统一写入右键菜单三属性（items/danger/content-height）。
/// danger 与 items 必须等长（平行数组）；高度由 items 推导（项 28px/分隔 1px + padding）。
/// 不触碰 x/y/visible——锚点与开关由调用方按语义自行控制（下钻复用原锚点）。
fn set_context_menu(ui: &MainWindow, items: Vec<slint::SharedString>, danger: Vec<bool>) {
    debug_assert_eq!(items.len(), danger.len(), "菜单文本与 danger 必须平行等长");
    let content_h = menu_content_height(&items);
    ui.set_context_menu_content_height(content_h);
    ui.set_context_menu_items(Rc::new(slint::VecModel::from(items)).into());
    ui.set_context_menu_danger(Rc::new(slint::VecModel::from(danger)).into());
}

/// "运行全部"进度快照（M-4）：工作线程写，UI 轮询 Timer 读（Mutex 保护）。
#[derive(Default, Clone, Copy)]
struct RunAllProgress {
    success: i32,
    failed: i32,
    /// 全部条目处理完毕（或被取消后退出）
    done: bool,
}

/// 单击防抖判定（v2 单击即启动）：同一条目 400ms 内的重复点击视为双击
/// 误触。按条目 id（而非下标）判定——切换分类/搜索后同一下标指向不同
/// 条目，用下标会把"切分类后点第一项"误判为同一项。
/// 命中后由调用方刷新时间戳，保证窗口内任意次点击都被抑制。
fn click_is_debounced(
    last: Option<&(String, std::time::Instant)>,
    item_id: &str,
    now: std::time::Instant,
) -> bool {
    matches!(last,
        Some((last_id, last_time))
            if last_id == item_id && now.duration_since(*last_time).as_millis() < 400
    )
}

/// 项目右键菜单动作（v2 动作映射分发）：文本/动作/danger 三数组平行，
/// 增删菜单项不依赖手写索引。
#[derive(Clone, PartialEq)]
enum ItemMenuAction {
    Edit,
    RunAsAdmin,
    Delete,
    CopyPath,
    OpenLocation,
    MoveUp,
    MoveDown,
    Run,
    RefreshIcon,
    Properties,
    /// 一级菜单"移动到..."入口（点击后原地下钻为分类列表，见 build_move_to_menu）。
    MoveToMenu,
    /// 二级菜单携带目标分类 id（排除条目当前所在分类，见 build_move_to_menu）。
    MoveTo(String),
    /// 二级菜单"返回"：恢复一级菜单内容。
    Back,
}

/// 项目右键菜单条目构建（纯函数）：返回文本与动作的平行数组。
/// 搜索态隐藏上移/下移（搜索结果排序语义与全局向量不一致，建议④）；
/// "移动到..." 单项折叠（cats.len()>=2 才显示），点击后下钻为分类列表。
fn build_item_menu(
    lb: &Localization,
    searching: bool,
    cats: &[(String, String)],
) -> (Vec<slint::SharedString>, Vec<ItemMenuAction>) {
    let mut texts: Vec<slint::SharedString> = Vec::new();
    let mut actions: Vec<ItemMenuAction> = Vec::new();
    texts.push(lb.tr("Menu_Edit").into());
    actions.push(ItemMenuAction::Edit);
    texts.push(lb.tr("Menu_RunAsAdmin").into());
    actions.push(ItemMenuAction::RunAsAdmin);
    texts.push(lb.tr("Menu_DelItem").into());
    actions.push(ItemMenuAction::Delete);
    texts.push(lb.tr("Menu_CopyPath").into());
    actions.push(ItemMenuAction::CopyPath);
    texts.push(lb.tr("Menu_OpenLocationFile").into());
    actions.push(ItemMenuAction::OpenLocation);
    if !searching {
        texts.push(lb.tr("Menu_MoveUp").into());
        actions.push(ItemMenuAction::MoveUp);
        texts.push(lb.tr("Menu_MoveDown").into());
        actions.push(ItemMenuAction::MoveDown);
    }
    texts.push(lb.tr("Menu_Run").into());
    actions.push(ItemMenuAction::Run);
    texts.push(lb.tr("Menu_RefreshIcon").into());
    actions.push(ItemMenuAction::RefreshIcon);
    texts.push(lb.tr("Menu_Properties").into());
    actions.push(ItemMenuAction::Properties);
    let move_to = lb.tr("Menu_MoveTo").to_string();
    // 折叠入口：仅存在 ≥2 个分类时可移动（单分类无目标，旧版会显示无意义的"移动到唯一分类"）
    if cats.len() >= 2 {
        texts.push(slint::SharedString::from(format!("{}...", move_to)));
        actions.push(ItemMenuAction::MoveToMenu);
    }
    (texts, actions)
}

/// "移动到..."二级菜单条目构建（纯函数）：首项"返回"，其后为可移动的分类列表。
/// `exclude_id` 为条目当前所在分类（移入原分类无意义，排除）。
fn build_move_to_menu(
    lb: &Localization,
    cats: &[(String, String)],
    exclude_id: &str,
) -> (Vec<slint::SharedString>, Vec<ItemMenuAction>) {
    let mut texts: Vec<slint::SharedString> = Vec::new();
    let mut actions: Vec<ItemMenuAction> = Vec::new();
    texts.push(lb.tr("Menu_Back").into());
    actions.push(ItemMenuAction::Back);
    for (cat_id, name) in cats {
        if cat_id == exclude_id {
            continue;
        }
        texts.push(name.as_str().into());
        actions.push(ItemMenuAction::MoveTo(cat_id.clone()));
    }
    (texts, actions)
}

/// "运行全部"完成汇总文案（纯函数）：全部成功用单参数模板，
/// 部分失败用双参数模板（成功数/失败数）。
fn run_all_summary_text(loc: &Localization, success: i32, failed: i32) -> String {
    if failed == 0 {
        loc.tr("Status_RunAllDone")
            .replace("{0}", &success.to_string())
    } else {
        loc.tr("Status_RunAllDonePartial")
            .replace("{0}", &success.to_string())
            .replace("{1}", &failed.to_string())
    }
}

/// 条目拖拽排序落点处理（纯状态操作，on_dropped_on 回调与测试复用）：
/// 守卫（越界/同位/搜索态/空分类）+ 重排 + 落盘。返回是否发生重排。
/// 搜索态排序语义与全局向量不一致，直接忽略（与菜单隐藏上移/下移同理）。
fn apply_item_drop(state: &Rc<RefCell<AppState>>, source: usize, target: usize) -> bool {
    if source == usize::MAX || source == target {
        return false;
    }
    let mut state = state.borrow_mut();
    if !state.search_query.is_empty() {
        log::debug!("Drop ignored during search");
        return false;
    }
    let cat_id = state.current_category_id.clone();
    if cat_id.is_empty() {
        return false;
    }
    if !state.manager.move_item_to_index(&cat_id, source, target) {
        return false;
    }
    state.save_if_modified();
    true
}

fn apply_background(ui: &MainWindow, state: &Rc<RefCell<AppState>>, path: &str) -> bool {
    let Some(decoded) = crate::background::decode(path) else {
        log::warn!("Background decode failed: {}", path);
        return false;
    };
    ui.set_bg_image(slint::Image::from_rgba8(decoded.buffer));
    ui.set_has_bg_image(true);
    // L-6：保存失败须提示（此前返回值被忽略，背景图仅存内存、重启即丢且无感知）
    let saved = {
        let mut st = state.borrow_mut();
        st.manager
            .config_mut()
            .extra_config
            .insert("background_image".into(), path.into());
        st.save_if_modified()
    };
    if !saved {
        let loc = state.borrow().loc.clone();
        let msg = loc.borrow().tr("Status_SaveFailed").to_string();
        ui_sync::show_toast(ui, msg, true);
    }
    true
}

pub fn wire_handler(
    ui: &MainWindow,
    state: Rc<RefCell<AppState>>,
    _loc: &Rc<RefCell<Localization>>,
    hotkey_mgr: Rc<RefCell<hotkey::HotkeyManager>>,
    hk_tx_shared: Rc<RefCell<Option<std::sync::mpsc::Sender<()>>>>,
) {
    let right_clicked_index: Rc<std::cell::Cell<Option<i32>>> = Rc::new(std::cell::Cell::new(None));
    // 确认弹窗待执行动作（P2-3：原 4 个 pending_* Option 合并为单一枚举，
    // 避免多个 Option 并联分发时漏改/级联执行；同刻仅一个确认框，至多一个动作待定）。
    enum ConfirmAction {
        DeleteItem(String),
        DeleteCategory(String),
        RunAll(String),
        RunAsAdmin(String),
        // 导入配置（P1：防误覆盖）——携带预解析结果与来源路径，
        // 确认后交 config_io::import_config_apply 执行整体替换
        Import(String, crate::model::AppConfig),
    }
    let pending_confirm: Rc<std::cell::RefCell<Option<ConfirmAction>>> =
        Rc::new(std::cell::RefCell::new(None));
    // 右键菜单来源：0=项目 1=背景 2=分类（单个 ContextMenu 复用，按来源分发）
    let menu_kind: Rc<std::cell::Cell<u8>> = Rc::new(std::cell::Cell::new(0));
    let right_clicked_category: Rc<std::cell::Cell<Option<i32>>> =
        Rc::new(std::cell::Cell::new(None));
    let pending_category_rename: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));
    // InputDialog 模式：0=无 1=分类重命名 2=快捷键设置
    let input_mode: Rc<std::cell::Cell<u8>> = Rc::new(std::cell::Cell::new(0));

    let state_file = state.clone();
    let weak_ui_file = ui.as_weak();
    ui.on_file_dropped(move |name, target| {
        log::info!(
            "File dropped: name={}, target={}",
            util::sanitize_for_log(&name),
            util::sanitize_for_log(&target)
        );
        let path = PathBuf::from(target.to_string());
        let mut state = state_file.borrow_mut();
        let cat_id = state.current_category_id.clone();
        // 拖入添加条目（内部幂等：重复 target 跳过）
        let _ = state
            .manager
            .handle_drop(std::slice::from_ref(&path), &cat_id);
        state.save_if_modified();
        // P2：图标统一走 refresh_items_ui 的异步提取/解码队列（后台线程提取 +
        // 房管 Timer 整表刷新落盘）——网络盘/光驱等慢速源不再阻塞 UI 线程；
        // 代价仅是图标延后百毫秒级上屏，且与"粘贴路径"入口行为一致。
        // （会话级防抖：提取失败条目本会话不重复自动重试，可经右键"刷新图标"强制重试。）
        let query = state.search_query.clone();
        drop(state);
        if let Some(ui) = weak_ui_file.upgrade() {
            ui_sync::refresh_items_ui(&ui, &state_file.borrow(), &cat_id, &query);
        }
    });

    let state_search = state.clone();
    let weak_ui_search = ui.as_weak();
    // 搜索防抖（P2 #3，300ms）：Timer 由闭包持有存活
    let search_timer = Rc::new(slint::Timer::default());
    ui.on_search_changed(move |text| {
        let text = text.to_string();
        log::debug!("Search changed: {}", text);
        let mut state = state_search.borrow_mut();
        state.search_query = text.clone();
        drop(state);
        let timer = search_timer.clone();
        let weak = weak_ui_search.clone();
        let state_cb = state_search.clone();
        timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(300),
            move || {
                if let Some(ui) = weak.upgrade() {
                    // P1-2：回调内重新读取当前分类。若在 300ms 窗口内切换了分类，
                    // 捕获的旧 cat_id 会把界面刷回旧分类内容。
                    let (cat_id, query) = {
                        let st = state_cb.borrow();
                        (st.current_category_id.clone(), st.search_query.clone())
                    };
                    if query != text {
                        // 期间又输入了新内容，交给更新的一次 Timer 处理
                        return;
                    }
                    ui_sync::refresh_items_ui(&ui, &state_cb.borrow(), &cat_id, &query);
                }
            },
        );
    });

    let state_cat = state.clone();
    let weak_ui_cat = ui.as_weak();
    ui.on_category_selected(move |index| {
        log::debug!("Category selected: index={}", index);
        let cat_id = {
            let state = state_cat.borrow();
            match state.manager.categories().get(index as usize) {
                Some(cat) => cat.id.clone(),
                None => return,
            }
        };
        if let Some(ui) = weak_ui_cat.upgrade() {
            // 侧栏点击：保留搜索态（列表继续显示搜索结果，与运行取项集合一致）
            ui_sync::switch_category(&ui, &state_cat, &cat_id, true);
        }
    });

    let state_newcat = state.clone();
    let weak_ui_newcat = ui.as_weak();
    ui.on_new_category(move || {
        log::debug!("New category requested");
        let loc_nc = state_newcat.borrow().loc.clone();
        let mut state = state_newcat.borrow_mut();
        let sort_order = state.manager.categories().len() as i32;
        let new_cat = Category {
            id: model::generate_id("cat"),
            name: format!(
                "{} {}",
                loc_nc.borrow().tr("Default_NewCategory"),
                sort_order + 1
            ),
            sort_order,
            view_type: ViewType::List,
            icon_size: 48,
        };
        let cat_id = new_cat.id.clone();
        state.manager.add_category(new_cat);
        state.save_if_modified();
        drop(state);
        if let Some(ui) = weak_ui_newcat.upgrade() {
            // H-2：新建分类属"数据集更换"，须清空搜索态后显示新分类全量条目，
            // 否则显示集合（新分类全量）与运行取项集合（旧搜索结果）索引错位
            ui_sync::refresh_categories_ui(&ui, &state_newcat.borrow());
            ui_sync::switch_category(&ui, &state_newcat, &cat_id, false);
        }
    });

    let state_view = state.clone();
    let weak_ui_view = ui.as_weak();
    ui.on_toggle_view(move || {
        log::debug!("View toggled");
        // Rust 的 on_ 回调覆盖 slint 层 `toggle-view => { is-icon-view = !is-icon-view; }`
        // handler，需在此自行翻转并写回 UI 属性。
        if let Some(ui) = weak_ui_view.upgrade() {
            let new_icon = !ui.get_is_icon_view();
            ui.set_is_icon_view(new_icon);
        }
        let mut state = state_view.borrow_mut();
        let cat_id = state.current_category_id.clone();
        if let Some(cat) = state
            .manager
            .config_mut()
            .data
            .categories
            .iter_mut()
            .find(|c| c.id == cat_id)
        {
            cat.view_type = match cat.view_type {
                ViewType::Icon => ViewType::List,
                ViewType::List => ViewType::Icon,
            };
        }
        state.save_if_modified();
        let query = state.search_query.clone();
        drop(state);
        if let Some(ui) = weak_ui_view.upgrade() {
            ui_sync::refresh_items_ui(&ui, &state_view.borrow(), &cat_id, &query);
        }
    });

    let state_theme = state.clone();
    let weak_ui_theme = ui.as_weak();
    ui.on_toggle_theme(move || {
        // Rust 的 on_ 回调覆盖 slint 层 `toggle-theme => { is-dark-theme = !is-dark-theme;
        // Theme.set-dark(...) }` handler，需在此自行翻转并应用主题，否则界面不切换。
        let dark = weak_ui_theme
            .upgrade()
            .map(|ui| {
                let new_dark = !ui.get_is_dark_theme();
                ui.set_is_dark_theme(new_dark);
                new_dark
            })
            .unwrap_or(false);
        let mut state = state_theme.borrow_mut();
        state
            .manager
            .config_mut()
            .extra_config
            .insert("theme".into(), if dark { "1".into() } else { "0".into() });
        state.save_if_modified();
    });

    // 菜单"快捷键"入口：打开 InputDialog 预填当前快捷键（S4）
    let input_mode_hk = input_mode.clone();
    let state_hk = state.clone();
    let weak_ui_hk = ui.as_weak();
    ui.on_hotkey(move || {
        input_mode_hk.set(2);
        if let Some(ui) = weak_ui_hk.upgrade() {
            let hk = state_hk
                .borrow()
                .manager
                .config()
                .config
                .global_hotkey
                .clone();
            let title = state_hk.borrow().loc.borrow().tr("STR_HotKey").to_string();
            ui.set_input_dialog_title(title.into());
            ui.set_input_dialog_text(hk.into());
            ui.set_input_dialog_error("".into());
            ui.set_input_dialog_visible(true);
        }
    });

    // 设置页：选择背景图（png/jpg；解码最长边 2560 封顶，见 background.rs）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_select_background(move || {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            let loc_bg = state_bg.borrow().loc.clone();
            let (bg_title, bg_filter) = {
                let lb = loc_bg.borrow();
                (
                    lb.tr("Title_SelectBgImage").to_string(),
                    format!("{}\0*.png;*.jpg;*.jpeg\0\0", lb.tr("STR_FilterImages")),
                )
            };
            let Some(path) = fileops::browse_file_dialog(&bg_title, &bg_filter) else {
                return;
            };
            if apply_background(&ui, &state_bg, &path) {
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                ui.set_settings_bg_status(name.into());
            } else {
                let loc = state_bg.borrow().loc.clone();
                let msg = loc.borrow().tr("Tip_BG_LoadFail").to_string();
                ui_sync::show_toast(&ui, msg, true);
            }
        });
    }
    // 设置页：清除背景图（恢复纯色 bg-0）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_clear_background(move || {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_has_bg_image(false);
            ui.set_bg_image(slint::Image::default());
            let none_text = {
                let mut st = state_bg.borrow_mut();
                st.manager
                    .config_mut()
                    .extra_config
                    .remove("background_image");
                st.save_if_modified();
                // 先绑定再返回：避免临时 Ref 活过 st 的 drop（E0597）
                let text = st.loc.borrow().tr("Dlg_BG_None").to_string();
                text
            };
            ui.set_settings_bg_status(none_text.into());
        });
    }
    // 设置页：遮罩开关（低频点击，立即落盘）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_dim_toggled(move |enabled| {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_bg_dim_enabled(enabled);
            let mut st = state_bg.borrow_mut();
            st.manager.config_mut().extra_set_bool("background_dim_enabled", enabled);
            st.save_if_modified();
        });
    }
    // 设置页：遮罩浓度滑块（拖动中仅更新内存与 UI，关闭设置页时统一落盘）
    {
        let state_bg = state.clone();
        let weak_ui_bg = ui.as_weak();
        ui.on_settings_dim_changed(move |percent| {
            let Some(ui) = weak_ui_bg.upgrade() else {
                return;
            };
            ui.set_bg_dim((percent / 100.0).clamp(0.0, 1.0));
            let mut st = state_bg.borrow_mut();
            st.manager.config_mut().extra_config.insert(
                "background_dim".into(),
                (percent.round() as i32).clamp(0, 100).to_string(),
            );
        });
    }
    // 设置页关闭：把拖动中暂存的遮罩浓度统一落盘
    {
        let state_bg = state.clone();
        ui.on_settings_closed(move || {
            state_bg.borrow_mut().save_if_modified();
        });
    }
    // P3-11：设置页"光标离开自动隐藏"开关：写 extra_config 落盘 + 更新运行时
    // 共享标志（拖放消费线程事件时刻读取，切换即时生效无需重启）
    {
        let state_ah = state.clone();
        ui.on_settings_auto_hide_toggled(move |enabled| {
            let mut st = state_ah.borrow_mut();
            st.manager.config_mut().extra_set_bool("auto_hide", enabled);
            st.save_if_modified();
            st.auto_hide_flag
                .store(enabled, std::sync::atomic::Ordering::Relaxed);
            log::info!("Auto hide toggled: {}", enabled);
        });
    }

    let state_new = state.clone();
    let weak_ui_new = ui.as_weak();
    ui.on_new_item_clicked(move || {
        log::debug!("New item clicked");
        edit_dialog::show_edit_dialog(&weak_ui_new, &state_new);
    });

    let state_refresh = state.clone();
    let weak_ui_refresh = ui.as_weak();
    ui.on_refresh_icons(move || {
        log::debug!("Refresh icons");
        let state = state_refresh.borrow();
        let items = state.manager.items(&state.current_category_id);
        let icon_cache = state.icon_cache.clone();
        let pending = state.pending_icon_paths.clone();
        // P1-2：手动刷新属显式强制重试，清除本分类条目的提取防抖标记
        {
            let mut attempted = state
                .icon_extract_attempted
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            for it in &items {
                let _ = attempted.remove(&it.id);
            }
        }
        drop(state);
        if let Some(ui) = weak_ui_refresh.upgrade() {
            ui_sync::spawn_icon_refresh(&ui, icon_cache, items, pending);
        }
    });

    // 单击运行指定索引项目（v2 交互变更：单击即启动，无需双击）。
    // 防抖：同一项目 400ms 内的重复点击（双击误触）忽略，避免连开两个实例；
    // 以项目 id（而非下标）判定——切换分类/搜索后同一下标指向不同项目，
    // 用下标会把"切分类后点第一项"误判为同一项。
    let state_click = state.clone();
    // 键盘导航（回车）运行：独立于单击防抖，直接用原始 state/weak。
    let weak_ui_sel = ui.as_weak();
    let state_sel = state.clone();
    // 防抖状态：Rc<RefCell>（Cell::get 要求 Copy，而 Option<(String, Instant)> 非 Copy）。
    let last_click: Rc<std::cell::RefCell<Option<(String, std::time::Instant)>>> =
        Rc::new(std::cell::RefCell::new(None));
    ui.on_item_tile_clicked(move |index| {
        let now = std::time::Instant::now();
        let clicked_id = {
            let st = state_click.borrow();
            let items = if st.search_query.is_empty() {
                st.manager.items(&st.current_category_id)
            } else {
                st.manager.search_items(&st.search_query)
            };
            match items.get(index as usize) {
                Some(it) => it.id.clone(),
                None => return,
            }
        };
        // 防抖：同 id 400ms 内的重复点击直接忽略（命中后也更新 last_click，
        // 保证 400ms 窗口内任意次点击都被抑制）。
        let debounced = click_is_debounced(last_click.borrow().as_ref(), &clicked_id, now);
        if debounced {
            log::debug!("Item click debounced: index={}", index);
            last_click.borrow_mut().replace((clicked_id, now));
            return;
        }
        last_click.borrow_mut().replace((clicked_id, now));
        log::debug!("Item clicked: index={}", index);
        ui_sync::run_item_at_index(&state_click, index);
    });

    // 项目拖拽排序（F2-1，DragArea/DropArea）：
    // 1) DragArea.data 绑定 item-drag-data(index) pure 回调——tile 创建即构造载荷
    //    （源索引字符串），消除"down 时 Rust 回写属性"的时序竞态。
    // 2) 拖到目标 tile → dropped-on(target, data) → Rust 读 data 得源索引 → 重排。
    ui.on_item_drag_data(move |index| {
        use slint::private_unstable_api::re_exports::DataTransfer;
        DataTransfer::from(slint::SharedString::from(index.to_string()))
    });

    let state_drop = state.clone();
    let weak_ui_drop = ui.as_weak();
    ui.on_dropped_on(move |target, data| {
        // data 载荷 = 源索引字符串（item-drag-data pure 回调构造）
        let source: usize = data
            .plain_text()
            .ok()
            .and_then(|s| s.to_string().parse().ok())
            .unwrap_or(usize::MAX);
        let target = target.max(0) as usize;
        log::debug!("Item dropped: source={}, target={}", source, target);
        if apply_item_drop(&state_drop, source, target) {
            let cat_id = state_drop.borrow().current_category_id.clone();
            if let Some(ui) = weak_ui_drop.upgrade() {
                ui_sync::refresh_items_ui(&ui, &state_drop.borrow(), &cat_id, "");
            }
        }
    });

    // 分类拖拽排序（F2-1，DragArea/DropArea）：data 绑定 category-drag-data(index)
    // pure 回调即时构造（消除时序竞态）；拖入目标分类 → category-dropped-on(target, data)
    // → 读 data 得源索引 → 重排 + 落盘 + 刷新侧栏。
    ui.on_category_drag_data(move |index| {
        use slint::private_unstable_api::re_exports::DataTransfer;
        DataTransfer::from(slint::SharedString::from(index.to_string()))
    });

    let state_cdrop = state.clone();
    let weak_ui_cdrop = ui.as_weak();
    ui.on_category_dropped_on(move |target, data| {
        let source: usize = data
            .plain_text()
            .ok()
            .and_then(|s| s.to_string().parse().ok())
            .unwrap_or(usize::MAX);
        let target = target.max(0) as usize;
        log::debug!("Category dropped: source={}, target={}", source, target);
        if source == usize::MAX || source == target {
            return;
        }
        let mut state = state_cdrop.borrow_mut();
        if !state.manager.move_category_to_index(source, target) {
            return;
        }
        state.save_if_modified();
        drop(state);
        if let Some(ui) = weak_ui_cdrop.upgrade() {
            ui_sync::refresh_categories_ui(&ui, &state_cdrop.borrow());
        }
    });

    // 滚轮切页（F7）：当前分类 ± delta，首尾循环回绕。
    // P1：与 on_category_selected 行为对齐——搜索态下保留 search_query，
    // 列表继续显示搜索结果（否则 refresh_items_ui 传空串显示全部分类条目，
    // 但 run_item_at_index/右键菜单仍按 search_query 取项，索引错位可能启动错误程序）。
    let state_ws = state.clone();
    let weak_ui_ws = ui.as_weak();
    ui.on_wheel_switch(move |delta| {
        log::debug!("Wheel switch: delta={}", delta);
        let cat_id = {
            let state = state_ws.borrow_mut();
            let n = state.manager.categories().len();
            if n == 0 {
                return;
            }
            let cur_idx = state
                .manager
                .categories()
                .iter()
                .position(|c| c.id == state.current_category_id)
                .unwrap_or(0);
            let new_idx = ((cur_idx as i32 + delta).rem_euclid(n as i32)) as usize;
            state.manager.categories()[new_idx].id.clone()
        };
        if let Some(ui) = weak_ui_ws.upgrade() {
            // 搜索态切页保留 search_query（否则显示与运行取项集合错位），
            // 由 switch_category 统一同步侧栏选中态与视图模式
            ui_sync::switch_category(&ui, &state_ws, &cat_id, true);
        }
    });

    ui.on_item_selected(move || {
        let ui = match weak_ui_sel.upgrade() {
            Some(u) => u,
            None => return,
        };
        let idx = ui.get_selected_item_index();
        log::debug!("Enter: run item index={}", idx);
        ui_sync::run_item_at_index(&state_sel, idx);
    });
    // 项目右键菜单（v2 重构）：动作映射分发（对齐汉堡菜单模式）。
    // MoveTo 携带目标分类 id（替代 moveto_base + 位置的索引换算）。
    let item_menu_actions: Rc<RefCell<Vec<ItemMenuAction>>> = Rc::new(RefCell::new(Vec::new()));

    let rci = right_clicked_index.clone();
    let mk_item = menu_kind.clone();
    let state_search_menu = state.clone();
    let weak_ui_right = ui.as_weak();
    let item_actions_build = item_menu_actions.clone();
    ui.on_item_right_clicked(move |index, x, y| {
        log::debug!("Item right-clicked: index={}", index);
        mk_item.set(0);
        rci.set(Some(index));
        // 搜索态排序语义与全局向量不一致，隐藏 Move Up/Down（建议④）
        let st = state_search_menu.borrow();
        let searching = !st.search_query.is_empty();
        let cats: Vec<(String, String)> = st
            .manager
            .categories()
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        // 菜单文本与动作平行构建（纯函数，增删项不依赖手写索引）
        let (menu_vec, actions) = build_item_menu(&lb, searching, &cats);
        drop(lb);
        // v2 危险项标红：删除项为 danger；由 action 映射生成，避免索引错位
        let danger: Vec<bool> = actions
            .iter()
            .map(|a| matches!(a, ItemMenuAction::Delete))
            .collect();
        *item_actions_build.borrow_mut() = actions;
        let item_count = menu_vec.len();
        if let Some(ui) = weak_ui_right.upgrade() {
            set_context_menu(&ui, menu_vec, danger);
            geometry::position_menu_at_cursor(&ui, x, y, item_count);
        }
    });

    // 背景右键菜单（P1-1）：新建项目/分类、刷新图标、粘贴、排序
    let mk_bg = menu_kind.clone();
    let rci_bg = right_clicked_index.clone();
    let state_bg = state.clone();
    let weak_ui_bg = ui.as_weak();
    ui.on_background_right_clicked(move |x, y| {
        log::debug!("Background right-clicked");
        mk_bg.set(1);
        rci_bg.set(None);
        let st = state_bg.borrow();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        let sort_tpl = lb.tr("Menu_SortBy").to_string();
        let sort_name = lb.tr("Menu_Sort_Name").to_string();
        let sort_usage = lb.tr("Menu_Sort_RunCount").to_string();
        let menu_vec = vec![
            slint::SharedString::from(lb.tr("Menu_NewItem")),
            slint::SharedString::from(lb.tr("Menu_NewC")),
            slint::SharedString::from(lb.tr("Menu_RefreshIcon")),
            slint::SharedString::from(lb.tr("Menu_Paste")),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_name)),
            slint::SharedString::from(sort_tpl.replace("{0}", &sort_usage)),
            slint::SharedString::from(lb.tr("Menu_RunAll")),
        ];
        if let Some(ui) = weak_ui_bg.upgrade() {
            // v2：背景菜单无危险项
            set_context_menu(&ui, menu_vec, vec![false; 7]);
            geometry::position_menu_at_cursor(&ui, x, y, 7);
        }
    });

    // 分类右键菜单（P1-3）：重命名/删除/上移/下移（决策①：右键上移下移替代拖拽）
    let mk_cat = menu_kind.clone();
    let rcc_cat = right_clicked_category.clone();
    let state_cat_menu = state.clone();
    let weak_ui_cat_right = ui.as_weak();
    ui.on_category_right_clicked(move |index, x, y| {
        log::debug!("Category right-clicked: index={}", index);
        mk_cat.set(2);
        rcc_cat.set(Some(index));
        let st = state_cat_menu.borrow();
        let loc_m = st.loc.clone();
        drop(st);
        let lb = loc_m.borrow();
        let menu_vec = vec![
            slint::SharedString::from(lb.tr("Menu_ReName")),
            slint::SharedString::from(lb.tr("Menu_DelC")),
            slint::SharedString::from(lb.tr("Menu_MoveUp")),
            slint::SharedString::from(lb.tr("Menu_MoveDown")),
        ];
        if let Some(ui) = weak_ui_cat_right.upgrade() {
            // v2 危险项标红：删除分类(index 1)
            set_context_menu(&ui, menu_vec, vec![false, true, false, false]);
            geometry::position_menu_at_cursor(&ui, x, y, 4);
        }
    });

    let state_menu = state.clone();
    let weak_ui_menu = ui.as_weak();
    let pcm_menu = pending_confirm.clone();
    let mk_menu = menu_kind.clone();
    let rcc_menu = right_clicked_category.clone();
    let pcr_menu = pending_category_rename.clone();
    let im_menu = input_mode.clone();
    let item_menu_actions_menu = item_menu_actions.clone();

    // 汉堡菜单（v2）：组装单一列表 + 危险项数组；动作经 action 映射分发
    //（不依赖易错的手写索引，增删菜单项时对应关系自动保持）。仅中文，无语言切换项。
    #[derive(Clone, Copy, PartialEq)]
    enum MenuAction {
        NewItem,
        NewCategory,
        DelCategory,
        RenameCategory,
        Hotkey,
        ToggleView,
        ToggleTheme,
        FollowMouse,
        AutoStart,
        Export,
        Import,
        Settings,
        About,
        Exit,
    }
    let menu_actions: Rc<RefCell<Vec<Option<MenuAction>>>> = Rc::new(RefCell::new(Vec::new()));
    let state_hb = state.clone();
    let weak_ui_hb = ui.as_weak();
    let menu_actions_hb = menu_actions.clone();
    ui.on_hamburger_clicked(move || {
        log::debug!("Hamburger menu opened");
        let ui = match weak_ui_hb.upgrade() {
            Some(u) => u,
            None => return,
        };
        let dark = ui.get_is_dark_theme();
        let icon_view = ui.get_is_icon_view();
        let items: Vec<slint::SharedString> = {
            let st = state_hb.borrow();
            let lb = st.loc.borrow();
            let mut items: Vec<slint::SharedString> = Vec::new();
            let mut actions: Vec<Option<MenuAction>> = Vec::new();
            let mut add = |s: slint::SharedString, a: Option<MenuAction>| {
                items.push(s);
                actions.push(a);
            };
            add(lb.tr("Menu_NewItem").into(), Some(MenuAction::NewItem));
            add(lb.tr("Menu_NewC").into(), Some(MenuAction::NewCategory));
            add("-".into(), None);
            add(
                lb.tr("Menu_DelCurrentC").into(),
                Some(MenuAction::DelCategory),
            );
            add("-".into(), None);
            add(
                lb.tr("Menu_RenameCurrentC").into(),
                Some(MenuAction::RenameCategory),
            );
            add(lb.tr("Menu_ViewHotKey").into(), Some(MenuAction::Hotkey));
            add("-".into(), None);
            // 切换视图：当前图标视图显示"列表"，反之显示"图标"
            if icon_view {
                add(lb.tr("Menu_ViewIcon1").into(), Some(MenuAction::ToggleView));
            } else {
                add(lb.tr("Menu_ViewIcon0").into(), Some(MenuAction::ToggleView));
            }
            let theme_key = if dark {
                "Menu_SwitchLight"
            } else {
                "Menu_SwitchDark"
            };
            add(lb.tr(theme_key).into(), Some(MenuAction::ToggleTheme));
            add("-".into(), None);
            add(
                lb.tr("Menu_FollowMouse").into(),
                Some(MenuAction::FollowMouse),
            );
            // 开机自启：菜单项带实时状态后缀（读注册表，与托盘 ✓/✗ 同源）
            let as_text = format!(
                "{}（{}）",
                lb.tr("Tray_AutoStart"),
                if crate::tray::is_autostart_enabled() {
                    lb.tr("STR_On")
                } else {
                    lb.tr("STR_Off")
                }
            );
            add(as_text.into(), Some(MenuAction::AutoStart));
            add("-".into(), None);
            add(lb.tr("Menu_ExportConfig").into(), Some(MenuAction::Export));
            add(lb.tr("Menu_ImportConfig").into(), Some(MenuAction::Import));
            add("-".into(), None);
            add(lb.tr("Menu_Settings").into(), Some(MenuAction::Settings));
            add(lb.tr("Menu_About").into(), Some(MenuAction::About));
            add(lb.tr("Menu_Exit").into(), Some(MenuAction::Exit));
            *menu_actions_hb.borrow_mut() = actions;
            items
        };
        // 危险项：删除当前分类、退出（由 action 映射生成，避免索引错位）
        let danger: Vec<bool> = menu_actions_hb
            .borrow()
            .iter()
            .map(|a| matches!(a, Some(MenuAction::DelCategory) | Some(MenuAction::Exit)))
            .collect();
        // 弹层内容总高：项 28px、分隔线 1px、上下 padding 4px——供挂载处限高滚动
        let content_h = menu_content_height(&items);
        ui.set_menu_popup_content_height(content_h);
        ui.set_menu_popup_items(Rc::new(slint::VecModel::from(items)).into());
        ui.set_menu_popup_danger(Rc::new(slint::VecModel::from(danger)).into());
    });

    let state_mps = state.clone();
    let weak_ui_mps = ui.as_weak();
    let pcm_mps = pending_confirm.clone();
    let pcr_mps = pending_category_rename.clone();
    let im_mps = input_mode.clone();
    let menu_actions_mps = menu_actions.clone();
    ui.on_menu_popup_selected(move |index| {
        log::debug!("Menu popup item selected: index={}", index);
        let ui = match weak_ui_mps.upgrade() {
            Some(u) => u,
            None => return,
        };
        let action = menu_actions_mps
            .borrow()
            .get(index as usize)
            .copied()
            .flatten();
        match action {
            Some(MenuAction::NewItem) => ui.invoke_new_item_clicked(),
            Some(MenuAction::NewCategory) => ui.invoke_new_category(),
            Some(MenuAction::DelCategory) => {
                // 删除当前分类：复用分类右键删除流程（确认后级联删除）
                let state = state_mps.borrow_mut();
                let cat_id = state.current_category_id.clone();
                if cat_id.is_empty() {
                    return;
                }
                let loc_m = state.loc.clone();
                let cat_name = state
                    .manager
                    .categories()
                    .iter()
                    .find(|c| c.id == cat_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                *pcm_mps.borrow_mut() = Some(ConfirmAction::DeleteCategory(cat_id));
                drop(state);
                ui.set_confirm_dialog_title(loc_m.borrow().tr("Title_ConfirmDelete").into());
                ui.set_confirm_dialog_message(
                    util::tr_fill(&loc_m, "Tip_DelCate", &cat_name).into(),
                );
                ui.set_confirm_confirm_text(loc_m.borrow().tr("Btn_Del").to_string().into());
                ui.set_confirm_dialog_visible(true);
            }
            Some(MenuAction::RenameCategory) => {
                // 重命名当前分类：复用分类右键重命名流程（InputDialog）
                let state = state_mps.borrow_mut();
                let cat_id = state.current_category_id.clone();
                if cat_id.is_empty() {
                    return;
                }
                let loc_m = state.loc.clone();
                let cat_name = state
                    .manager
                    .categories()
                    .iter()
                    .find(|c| c.id == cat_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                *pcr_mps.borrow_mut() = Some(cat_id);
                im_mps.set(1);
                drop(state);
                ui.set_input_dialog_title(loc_m.borrow().tr("Title_RenameCategory").into());
                ui.set_input_dialog_text(cat_name.into());
                ui.set_input_dialog_error("".into());
                ui.set_input_dialog_visible(true);
            }
            Some(MenuAction::Hotkey) => ui.invoke_hotkey(),
            Some(MenuAction::ToggleView) => {
                // 视图切换（v2 单一"切换"项，无条件 toggle）
                ui.invoke_toggle_view();
            }
            Some(MenuAction::ToggleTheme) => ui.invoke_toggle_theme(),
            Some(MenuAction::FollowMouse) => crate::tray_ui::handle_tray_ui_action(
                crate::tray_ui::TrayUiAction::ToggleFollowMouse,
                &ui,
                &state_mps,
            ),
            Some(MenuAction::AutoStart) => {
                // 切换开机自启并即时反馈（成功提示新状态，失败明确报错）
                let (msg, err) = {
                    let st = state_mps.borrow();
                    let lb = st.loc.borrow();
                    match crate::tray::toggle_autostart() {
                        Some(true) => (lb.tr("Tip_AutoStart_On").to_string(), false),
                        Some(false) => (lb.tr("Tip_AutoStart_Off").to_string(), false),
                        None => (lb.tr("Tip_AutoStart_Fail").to_string(), true),
                    }
                };
                crate::ui_sync::show_toast(&ui, msg, err);
            }
            Some(MenuAction::Export) => crate::handlers::config_io::export_config(&ui, &state_mps),
            Some(MenuAction::Import) => {
                // P1：导入前预解析并弹"覆盖确认"框，防误选文件静默覆盖全部数据；
                // 预解析（含规模统计）在菜单阶段完成，确认后由 confirm 回调执行替换。
                if let Some((import_path, imported)) =
                    crate::handlers::config_io::prepare_import(&ui, &state_mps)
                {
                    let (n_cats, n_items) =
                        (imported.data.categories.len(), imported.data.items.len());
                    let loc_m = state_mps.borrow().loc.clone();
                    *pcm_mps.borrow_mut() = Some(ConfirmAction::Import(import_path, imported));
                    ui.set_confirm_dialog_title(loc_m.borrow().tr("Title_ImportConfig").into());
                    let msg = {
                        let lb = loc_m.borrow();
                        lb.tr("Tip_ImportConfirm")
                            .replace("{0}", &n_cats.to_string())
                            .replace("{1}", &n_items.to_string())
                    };
                    ui.set_confirm_dialog_message(msg.into());
                    ui.set_confirm_confirm_text(loc_m.borrow().tr("Btn_OK").to_string().into());
                    ui.set_confirm_dialog_visible(true);
                }
            }
            Some(MenuAction::Settings) => {
                // 打开设置对话框：回填背景图与遮罩当前状态
                let st = state_mps.borrow();
                let cfg = st.manager.config();
                let status = match cfg.extra_config.get("background_image") {
                    Some(p) => std::path::Path::new(p)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.clone()),
                    None => st.loc.borrow().tr("Dlg_BG_None").to_string(),
                };
                let dim_enabled = cfg
                    .extra_config
                    .get("background_dim_enabled")
                    .map(|v| v == "1")
                    .unwrap_or(true);
                let dim_percent: f32 = cfg
                    .extra_config
                    .get("background_dim")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(35.0);
                let log_index = match cfg.extra_config.get("log_level").map(|s| s.as_str()) {
                    Some("debug") => 2,
                    // 未设置（空）与实际生效档位一致：默认 Info（见 logger::apply_level）
                    None | Some("info") | Some("") => 1,
                    _ => 0,
                };
                let auto_hide = cfg.extra_get_bool("auto_hide");
                drop(st);
                ui.set_settings_bg_status(status.into());
                ui.set_settings_bg_dim_enabled(dim_enabled);
                ui.set_settings_bg_dim_percent(dim_percent);
                ui.set_settings_log_index(log_index);
                ui.set_settings_auto_hide(auto_hide);
                ui.set_settings_dialog_visible(true);
            }
            Some(MenuAction::About) => ui.set_about_dialog_visible(true),
            Some(MenuAction::Exit) => {
                // 退出：真正结束进程（与托盘"退出"一致）；几何由主循环收尾强制落盘
                let _ = slint::quit_event_loop();
            }
            None => {}
        }
    });

    ui.on_menu_item_selected(move |index| {
        log::debug!(
            "Menu item selected: index={}, kind={}",
            index,
            mk_menu.get()
        );

        // 分类菜单（P1-3）：重命名/删除/上移/下移
        if mk_menu.get() == 2 {
            let cat_idx = rcc_menu.get();
            if let Some(idx) = cat_idx {
                match index {
                    0 => {
                        // 重命名：打开 InputDialog
                        let state = state_menu.borrow_mut();
                        let loc_m = state.loc.clone();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            let cat_name = cat.name.clone();
                            *pcr_menu.borrow_mut() = Some(cat_id);
                            im_menu.set(1);
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui.set_input_dialog_title(
                                    loc_m.borrow().tr("Title_RenameCategory").into(),
                                );
                                ui.set_input_dialog_text(cat_name.into());
                                ui.set_input_dialog_error("".into());
                                ui.set_input_dialog_visible(true);
                            }
                        }
                    }
                    1 => {
                        // 删除：确认后级联删除项目
                        let state = state_menu.borrow_mut();
                        let loc_m = state.loc.clone();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            let cat_name = cat.name.clone();
                            *pcm_menu.borrow_mut() = Some(ConfirmAction::DeleteCategory(cat_id));
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui.set_confirm_dialog_title(
                                    loc_m.borrow().tr("Title_ConfirmDelete").into(),
                                );
                                ui.set_confirm_dialog_message(
                                    util::tr_fill(&loc_m, "Tip_DelCate", &cat_name).into(),
                                );
                                ui.set_confirm_confirm_text(
                                    loc_m.borrow().tr("Btn_Del").to_string().into(),
                                );
                                ui.set_confirm_dialog_visible(true);
                            }
                        }
                    }
                    2 => {
                        let mut state = state_menu.borrow_mut();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            if state.manager.move_category_up(&cat_id) {
                                state.save_if_modified();
                                let cur = state.current_category_id.clone();
                                let q = state.search_query.clone();
                                drop(state);
                                if let Some(ui) = weak_ui_menu.upgrade() {
                                    ui_sync::refresh_categories_ui(&ui, &state_menu.borrow());
                                    ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cur, &q);
                                }
                            }
                        }
                    }
                    3 => {
                        let mut state = state_menu.borrow_mut();
                        if let Some(cat) = state.manager.categories().get(idx as usize) {
                            let cat_id = cat.id.clone();
                            if state.manager.move_category_down(&cat_id) {
                                state.save_if_modified();
                                let cur = state.current_category_id.clone();
                                let q = state.search_query.clone();
                                drop(state);
                                if let Some(ui) = weak_ui_menu.upgrade() {
                                    ui_sync::refresh_categories_ui(&ui, &state_menu.borrow());
                                    ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cur, &q);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            return;
        }

        // 背景菜单（P1-1）：新建项目/分类、刷新图标、粘贴、排序
        if mk_menu.get() == 1 {
            match index {
                0 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_new_item_clicked();
                    }
                }
                1 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_new_category();
                    }
                }
                2 => {
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.invoke_refresh_icons();
                    }
                }
                3 => {
                    // 粘贴：读剪贴板文件路径并加入当前分类
                    let paths = clipboard::clipboard_files();
                    if !paths.is_empty() {
                        let mut state = state_menu.borrow_mut();
                        let cat_id = state.current_category_id.clone();
                        let added = state.manager.handle_drop(&paths, &cat_id);
                        let count = added.len();
                        state.save_if_modified();
                        let q = state.search_query.clone();
                        let loc_m = state.loc.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            ui_sync::show_toast(
                                &ui,
                                util::tr_fill(&loc_m, "Status_Pasted", &count.to_string()),
                                false,
                            );
                        }
                    }
                }
                4 => {
                    let mut state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    state.manager.sort_items_by_name(&cat_id);
                    state.save_if_modified();
                    let q = state.search_query.clone();
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                    }
                }
                5 => {
                    let mut state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    state.manager.sort_items_by_usage(&cat_id);
                    state.save_if_modified();
                    let q = state.search_query.clone();
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                    }
                }
                6 => {
                    // 运行全部：弹确认框（Tip_RunAll），确认后逐一执行分类内项目
                    let state = state_menu.borrow_mut();
                    let cat_id = state.current_category_id.clone();
                    let cat_name = state
                        .manager
                        .categories()
                        .iter()
                        .find(|c| c.id == cat_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let loc_m = state.loc.clone();
                    *pcm_menu.borrow_mut() = Some(ConfirmAction::RunAll(cat_id));
                    drop(state);
                    if let Some(ui) = weak_ui_menu.upgrade() {
                        ui.set_confirm_dialog_title(
                            loc_m.borrow().tr("Title_ConfirmRunAll").into(),
                        );
                        ui.set_confirm_dialog_message(
                            util::tr_fill(&loc_m, "Tip_RunAll", &cat_name).into(),
                        );
                        ui.set_confirm_confirm_text(
                            loc_m.borrow().tr("Btn_Execute").to_string().into(),
                        );
                        ui.set_confirm_dialog_visible(true);
                    }
                }
                _ => {}
            }
            return;
        }

        // 项目菜单（现有逻辑 + Move Up/Down + Run/RefreshIcon/Properties/MoveTo）
        if let Some(item_idx) = right_clicked_index.get() {
            let mut state = state_menu.borrow_mut();
            let items = if state.search_query.is_empty() {
                state.manager.items(&state.current_category_id)
            } else {
                state.manager.search_items(&state.search_query)
            };
            if let Some(item) = items.get(item_idx as usize) {
                let item = item.clone();
                // 动作映射分发（对齐汉堡菜单模式）：搜索态构建菜单时已排除
                // MoveUp/MoveDown，无需索引换算与 normal_mode 守卫。
                let action = item_menu_actions_menu.borrow().get(index as usize).cloned();
                match action {
                    Some(ItemMenuAction::Edit) => {
                        log::info!("Edit item: {} ({})", item.name, item.id);
                        // P2-5：编辑态不再内联装配（原 ~150 行与新建路径重复的
                        // 对话框构造/回调）——共享装配统一在 edit_dialog.rs，
                        // 行为与新建路径一致（browse/取消/关闭/实例管理/校验）。
                        // 先结束 state 独占借用（共享装配内部按需借用）。
                        drop(state);
                        edit_dialog::show_edit_dialog_for_item(
                            &weak_ui_menu,
                            &state_menu,
                            &item,
                        );
                    }
                    Some(ItemMenuAction::RunAsAdmin) => {
                        log::info!("Run as admin: {} ({})", item.name, item.id);
                        let item_id = item.id.clone();
                        let item_name = item.name.clone();
                        let item_target = item.target.clone();
                        let loc_a = state.loc.clone();
                        *pcm_menu.borrow_mut() = Some(ConfirmAction::RunAsAdmin(item_id));
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui.set_confirm_dialog_title(
                                loc_a.borrow().tr("Title_ConfirmRunAsAdmin").into(),
                            );
                            let msg = format!(
                                "{} — {}\n{}",
                                item_name,
                                item_target,
                                loc_a.borrow().tr("Tip_UndoNotPossible")
                            );
                            ui.set_confirm_dialog_message(msg.into());
                            ui.set_confirm_confirm_text(
                                loc_a.borrow().tr("Btn_Execute").to_string().into(),
                            );
                            ui.set_confirm_dialog_visible(true);
                        }
                    }
                    Some(ItemMenuAction::Delete) => {
                        log::info!("Delete item: {} ({})", item.name, item.id);
                        let item_id = item.id.clone();
                        let item_name = item.name.clone();
                        let loc_m = state.loc.clone();
                        *pcm_menu.borrow_mut() = Some(ConfirmAction::DeleteItem(item_id));
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui.set_confirm_dialog_title(
                                loc_m.borrow().tr("Title_ConfirmDelete").into(),
                            );
                            ui.set_confirm_dialog_message(
                                util::tr_fill(&loc_m, "Tip_DelItem", &item_name).into(),
                            );
                            ui.set_confirm_confirm_text(
                                loc_m.borrow().tr("Btn_Del").to_string().into(),
                            );
                            ui.set_confirm_dialog_visible(true);
                        }
                    }
                    Some(ItemMenuAction::CopyPath) => {
                        clipboard::copy_to_clipboard(&item.target);
                    }
                    Some(ItemMenuAction::OpenLocation) => {
                        if crate::runner::is_url(&item.target) {
                            log::debug!("Skip open file location for URL target: {}", item.target);
                        } else {
                            fileops::open_file_location(&item.target);
                        }
                    }
                    Some(ItemMenuAction::Run) => {
                        log::info!("Run item: {} ({})", item.name, item.id);
                        let search_query = state.search_query.clone();
                        drop(state);
                        // P0-1：右键"运行"同样尊重 run_as_admin 标志（与单击一致）。
                        // M-5：异步执行，结果经队列回 UI 线程回写计数/提示。
                        ui_sync::run_item_async(&state_menu, item.clone(), search_query);
                    }
                    Some(ItemMenuAction::RefreshIcon) => {
                        log::info!("Refresh icon: {} ({})", item.name, item.id);
                        let icon_cache = state.icon_cache.clone();
                        let pending = state.pending_icon_paths.clone();
                        // P1-2：手动刷新属显式强制重试，清除该项提取防抖标记
                        {
                            let mut attempted = state
                                .icon_extract_attempted
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            let _ = attempted.remove(&item.id);
                        }
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::spawn_icon_refresh(
                                &ui,
                                icon_cache,
                                vec![item.clone()],
                                pending,
                            );
                        }
                    }
                    Some(ItemMenuAction::Properties) => {
                        log::info!("Properties: {} ({})", item.target, item.id);
                        if item.target.trim().is_empty() {
                            // 空目标条目（如历史占位 "New Item"）：明确提示而非静默无动作
                            let msg = {
                                let loc_m = state.loc.clone();
                                let lb = loc_m.borrow();
                                lb.tr("Err_EmptyTarget").to_string()
                            };
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                ui_sync::show_toast(&ui, msg, true);
                            }
                        } else {
                            drop(state);
                            fileops::show_properties(&item.target);
                        }
                    }
                    // 下钻：一级"移动到..." → 原地换内容为分类列表（菜单不关闭）。
                    // .slint 的 selected() 已置 visible=false；同一事件回调内重建后
                    // 再置 true，重绘发生在回调结束后，无闪烁。x/y 锚点不重设。
                    Some(ItemMenuAction::MoveToMenu) => {
                        let loc_m = state.loc.clone();
                        let cats: Vec<(String, String)> = state
                            .manager
                            .categories()
                            .iter()
                            .map(|c| (c.id.clone(), c.name.clone()))
                            .collect();
                        drop(state);
                        let lb = loc_m.borrow();
                        let (menu_vec, actions) =
                            build_move_to_menu(&lb, &cats, &item.category_id);
                        drop(lb);
                        *item_menu_actions_menu.borrow_mut() = actions;
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            // 先取长度再 move（参数从左到右求值，内联写法会 E0382）
                            let danger = vec![false; menu_vec.len()];
                            set_context_menu(&ui, menu_vec, danger);
                            ui.set_context_menu_visible(true);
                        }
                    }
                    // 返回：恢复一级菜单（搜索态/分类列表以当前 state 为准重建）。
                    Some(ItemMenuAction::Back) => {
                        let loc_m = state.loc.clone();
                        let searching = !state.search_query.is_empty();
                        let cats: Vec<(String, String)> = state
                            .manager
                            .categories()
                            .iter()
                            .map(|c| (c.id.clone(), c.name.clone()))
                            .collect();
                        drop(state);
                        let lb = loc_m.borrow();
                        let (menu_vec, actions) = build_item_menu(&lb, searching, &cats);
                        drop(lb);
                        *item_menu_actions_menu.borrow_mut() = actions;
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            let danger: Vec<bool> = item_menu_actions_menu
                                .borrow()
                                .iter()
                                .map(|a| matches!(a, ItemMenuAction::Delete))
                                .collect();
                            set_context_menu(&ui, menu_vec, danger);
                            ui.set_context_menu_visible(true);
                        }
                    }
                    Some(ItemMenuAction::MoveTo(target_cat)) => {
                        if state.manager.move_item(&item.id, &target_cat) {
                            state.save_if_modified();
                            let cat_id = state.current_category_id.clone();
                            let q = state.search_query.clone();
                            drop(state);
                            if let Some(ui) = weak_ui_menu.upgrade() {
                                // 真实 UI 中 .slint selected() 已关菜单；此处显式
                                // 复位保持与下钻/返回的 visible 控制对称（测试
                                // invoke 不经 .slint 链路，依赖本写法关闭）。
                                ui.set_context_menu_visible(false);
                                ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                            }
                        }
                    }
                    // clippy::collapsible_match：guard 仅在本臂选中时求值一次，
                    // 语义与"臂内 if 失败静默忽略"完全一致。
                    Some(ItemMenuAction::MoveUp)
                        if state.manager.move_item_up(&item.id) =>
                    {
                        state.save_if_modified();
                        let cat_id = state.current_category_id.clone();
                        let q = state.search_query.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                        }
                    }
                    Some(ItemMenuAction::MoveDown)
                        if state.manager.move_item_down(&item.id) =>
                    {
                        state.save_if_modified();
                        let cat_id = state.current_category_id.clone();
                        let q = state.search_query.clone();
                        drop(state);
                        if let Some(ui) = weak_ui_menu.upgrade() {
                            ui_sync::refresh_items_ui(&ui, &state_menu.borrow(), &cat_id, &q);
                        }
                    }
                    // `_` 兜底：guard 失败的 MoveUp/MoveDown 与 None 均静默忽略
                    // （带 guard 的臂不计入 match 穷尽性，须有通配臂）。
                    _ => {}
                }
            }
        }
    });

    let state_confirm = state.clone();
    let weak_ui_confirm = ui.as_weak();
    let pcm_ok = pending_confirm.clone();
    // 导入确认（P1）：import_config_apply 需传 hotkey_mgr/hk_tx（Rc 非 Copy），
    // 本闭包按值捕获，克隆一份避免与 input_dialog_ok 的捕获冲突（E0382）。
    let hotkey_mgr_confirm = hotkey_mgr.clone();
    let hk_tx_confirm = hk_tx_shared.clone();
    ui.on_confirm_dialog_ok(move || {
        // 只取一次：原各分支分别 take()，首个模式不匹配也会吞掉动作，
        // 导致 DeleteItem/RunAsAdmin/RunAll 确认后静默失效
        let pending = pcm_ok.borrow_mut().take();
        // 分类删除优先（级联删除项目 + 清理图标缓存）
        if let Some(ConfirmAction::DeleteCategory(cat_id)) = pending {
            let mut state = state_confirm.borrow_mut();
            // 先读取待删分类下的项目，用于后续清理 icon_cache（delete_category 后 items 已移除）
            let cat_items = state.manager.items(&cat_id);
            // P1-1：至少保留一个分类，删不动时提示并退出
            if !state.manager.delete_category(&cat_id) {
                // .to_string() 避免 tr() 返回 &str 借用临时 Ref（E0716）
                let tip = state.loc.borrow().tr("Tip_NeedOneCategory").to_string();
                drop(state);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    ui_sync::show_toast(&ui, tip, true);
                }
                return;
            }
            // 清理该分类所有项目的图标缓存
            for it in &cat_items {
                state.icon_cache.delete_cache(it);
            }
            state.save_if_modified();
            // 若删除的是当前分类，切换到第一个分类
            if state.current_category_id == cat_id {
                state.current_category_id = state
                    .manager
                    .categories()
                    .first()
                    .map(|c| c.id.clone())
                    .unwrap_or_default();
            }
            let cur = state.current_category_id.clone();
            let q = state.search_query.clone();
            drop(state);
            if let Some(ui) = weak_ui_confirm.upgrade() {
                ui_sync::refresh_categories_ui(&ui, &state_confirm.borrow());
                ui_sync::refresh_items_ui(&ui, &state_confirm.borrow(), &cur, &q);
            }
            return;
        }
        if let Some(ConfirmAction::DeleteItem(id)) = pending {
            let mut state = state_confirm.borrow_mut();
            if let Some(item) = state.manager.get_item(&id) {
                state.icon_cache.delete_cache(item);
            }
            if state.manager.delete_item(&id) {
                state.save_if_modified();
                let cat_id = state.current_category_id.clone();
                let query = state.search_query.clone();
                drop(state);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    // 条目数变化影响侧栏分类计数，需与条目列表一并刷新
                    ui_sync::refresh_categories_ui(&ui, &state_confirm.borrow());
                    ui_sync::refresh_items_ui(&ui, &state_confirm.borrow(), &cat_id, &query);
                }
                // F8/4.1：删除条目后重注册条目热键
                ui_sync::sync_item_hotkeys(&state_confirm.borrow());
            }
            // 删除项处理完毕后必须返回，避免与后续提权/运行全部 pending 级联执行
            return;
        }
        // 提权运行确认（右键菜单 → 确认弹窗 → 执行）
        if let Some(ConfirmAction::RunAsAdmin(id)) = pending {
            let (item, search_query) = {
                let state = state_confirm.borrow();
                (
                    state.manager.get_item(&id).cloned(),
                    state.search_query.clone(),
                )
            };
            if let Some(item) = item {
                // M-5：异步执行（UAC 弹窗期间不冻结界面），结果经队列回 UI 线程
                // 回写计数/提示（与单击/右键运行同管线）。
                ui_sync::run_item_async(&state_confirm, item, search_query);
            } else {
                // P5：项目已被删除时静默失败会误导用户，状态栏提示。
                log::warn!("Run-as-admin: item {} not found", id);
                if let Some(ui) = weak_ui_confirm.upgrade() {
                    let loc_m = state_confirm.borrow().loc.clone();
                    let msg = loc_m.borrow().tr("Tip_ItemNotFound").to_string();
                    ui_sync::show_toast(&ui, msg, true);
                }
            }
            return;
        }
        // 导入配置确认（P1）：菜单阶段已预解析，此处执行整体替换。
        // 置于 RunAll 之前并 return——RunAll 按值移动 cat_id 会使 pending
        // 部分移动，后续分支再读 pending 触发 E0382（CI 实测）。
        if let Some(ConfirmAction::Import(path, imported)) = pending {
            if let Some(ui) = weak_ui_confirm.upgrade() {
                crate::handlers::config_io::import_config_apply(
                    &ui,
                    &state_confirm,
                    &hotkey_mgr_confirm,
                    &hk_tx_confirm,
                    &path,
                    imported,
                );
            }
            return;
        }
        // 运行全部（背景右键菜单 → 确认弹窗 → 逐一执行分类内项目）。
        // M-4：启动全部移到工作线程逐项执行（项间 50ms 让出），UI 线程 Timer 仅
        // 轮询进度与汇总提示——UAC 提权弹窗/慢速目标不再冻结界面；窗口隐藏
        // （Esc/X/热键）或退出即取消剩余批次。
        if let Some(ConfirmAction::RunAll(cat_id)) = pending {
            let items: Vec<crate::model::Item> = {
                let state = state_confirm.borrow();
                state.manager.items(&cat_id).to_vec()
            };
            let search_query = state_confirm.borrow().search_query.clone();
            if !items.is_empty() {
                let progress = std::sync::Arc::new(std::sync::Mutex::new(RunAllProgress::default()));
                let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let outcomes = state_confirm.borrow().pending_run_results.clone();
                {
                    let progress_w = progress.clone();
                    let cancel_w = cancel.clone();
                    let search_q = search_query;
                    let spawned = std::thread::Builder::new()
                        .name("run-all".to_string())
                        .spawn(move || {
                            let total = items.len();
                            for (i, it) in items.iter().enumerate() {
                                if cancel_w.load(std::sync::atomic::Ordering::Relaxed) {
                                    break;
                                }
                                // 尊重项目"管理员运行"标志，与单击行为一致（P0）
                                let result = if it.run_as_admin {
                                    crate::runner::run_as_admin(it, &search_q)
                                } else {
                                    crate::runner::run(it, &search_q)
                                };
                                let success = result.success;
                                {
                                    let mut pg = progress_w.lock().unwrap_or_else(|e| e.into_inner());
                                    if success {
                                        pg.success += 1;
                                    } else {
                                        pg.failed += 1;
                                        log::warn!(
                                            "Run all: failed to run {} ({}) - {}",
                                            it.name,
                                            it.id,
                                            result.error_message
                                        );
                                    }
                                }
                                // 成功项经统一队列回 UI 线程回写运行计数（M-5 管线）
                                if success {
                                    if let Ok(mut q) = outcomes.lock() {
                                        q.push(RunOutcome {
                                            item_id: it.id.clone(),
                                            item_name: it.name.clone(),
                                            success: true,
                                            error: result.error,
                                        });
                                    }
                                }
                                // 批间让出 50ms，避免瞬时启动风暴
                                if i + 1 < total {
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }
                            }
                            let mut pg = progress_w.lock().unwrap_or_else(|e| e.into_inner());
                            pg.done = true;
                        });
                if let Err(e) = spawned {
                    // 线程创建失败（极端）：置 done 让轮询 Timer 正常收尾，避免空转
                    log::error!("Failed to spawn run-all thread: {}", e);
                    let mut pg = progress.lock().unwrap_or_else(|e| e.into_inner());
                    pg.done = true;
                }
                }
                // UI 轮询 Timer（100ms）：窗口隐藏即取消；完成时汇总提示。
                // slot 持有 Timer；回调尾部 take()+stop() 打破 闭包↔Timer 循环引用。
                let timer = Rc::new(slint::Timer::default());
                let slot_cb = Rc::new(RefCell::new(Some(timer.clone())));
                let weak = weak_ui_confirm.clone();
                let state_r = state_confirm.clone();
                timer.start(
                    slint::TimerMode::Repeated,
                    std::time::Duration::from_millis(100),
                    move || {
                        // 窗口隐藏（Esc/X/热键）或退出 → 取消剩余批次（用户已离开界面）
                        if let Some(ui) = weak.upgrade() {
                            if !ui.window().is_visible() {
                                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        let pg = { *progress.lock().unwrap_or_else(|e| e.into_inner()) };
                        if !pg.done {
                            return;
                        }
                        // 完成：打破 Timer 循环引用，再回写计数 + 汇总提示
                        if let Some(t) = slot_cb.borrow_mut().take() {
                            t.stop();
                        }
                        // 成功项回写运行计数并标脏软保存（去抖落盘，P1-1/M-6）
                        ui_sync::drain_run_results(&state_r);
                        let msg = {
                            let loc = state_r.borrow().loc.clone();
                            let loc_b = loc.borrow();
                            run_all_summary_text(&loc_b, pg.success, pg.failed)
                        };
                        if let Some(ui) = weak.upgrade() {
                            ui_sync::show_toast(&ui, msg, pg.failed > 0);
                        }
                    },
                );
            }
        }
    });

    let pcm_cancel = pending_confirm.clone();
    ui.on_confirm_dialog_cancel(move || {
        pcm_cancel.borrow_mut().take();
    });

    // 分类重命名确认（InputDialog）
    let pcr_ok = pending_category_rename.clone();
    let input_mode_ok = input_mode.clone();
    let state_input = state.clone();
    let weak_ui_input = ui.as_weak();
    ui.on_input_dialog_ok(move || {
        // 快捷键设置模式（菜单入口）：校验 → 重注册 → 持久化
        if input_mode_ok.get() == 2 {
            let new_hk = weak_ui_input
                .upgrade()
                .map(|ui| ui.get_input_dialog_text().to_string())
                .unwrap_or_default();
            if hotkey::parse_hotkey_string(&new_hk).is_none() {
                let loc = state_input.borrow().loc.clone();
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui.set_input_dialog_error(loc.borrow().tr("Tip_InvalidHotkey").into());
                    ui.set_input_dialog_visible(true);
                }
                return;
            }
            // P2-2：改键重注册+失败回滚逻辑抽取复用（与导入配置路径共享），
            // 成功写配置并落盘，失败回滚旧键并保持对话框提示。
            // 改键路径调用时配置尚未替换，当前 global_hotkey 即回滚旧键。
            let old_hk = state_input
                .borrow()
                .manager
                .config()
                .config
                .global_hotkey
                .clone();
            let ok = crate::handlers::config_io::apply_global_hotkey(
                &state_input,
                &hotkey_mgr,
                &hk_tx_shared,
                &new_hk,
                &old_hk,
            );
            if ok {
                input_mode_ok.set(0);
            } else {
                let loc = state_input.borrow().loc.clone();
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui.set_input_dialog_error(loc.borrow().tr("Tip_HotkeyRegisterFail").into());
                    ui.set_input_dialog_visible(true);
                }
            }
            return;
        }
        // 分类重命名模式：校验通过才消费 pending——空名拒绝时保留，
        // 用户在对话框内改正名称后重试仍生效（此前提前 take 导致
        // "空名→改正→再确认"被静默吞掉，CI 实测暴露）
        let cat_id = pcr_ok.borrow().clone();
        if let Some(cat_id) = cat_id {
            let mut state = state_input.borrow_mut();
            if let Some(cat) = state
                .manager
                .config_mut()
                .data
                .categories
                .iter_mut()
                .find(|c| c.id == cat_id)
            {
                let name = weak_ui_input
                    .upgrade()
                    .map(|ui| ui.get_input_dialog_text().to_string())
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    // 空名：保持对话框打开并提示（slint confirm 已先行关闭，此处重新显示）
                    let loc = state.loc.clone();
                    if let Some(ui) = weak_ui_input.upgrade() {
                        ui.set_input_dialog_error(loc.borrow().tr("Tip_NameEmpty").into());
                        ui.set_input_dialog_visible(true);
                    }
                    return;
                }
                // 校验通过：此刻才消费 pending（空名拒绝路径保留待重试）
                pcr_ok.borrow_mut().take();
                cat.name = name.trim().to_string();
                state.save_if_modified();
                let cur = state.current_category_id.clone();
                let q = state.search_query.clone();
                drop(state);
                input_mode_ok.set(0);
                if let Some(ui) = weak_ui_input.upgrade() {
                    ui_sync::refresh_categories_ui(&ui, &state_input.borrow());
                    ui_sync::refresh_items_ui(&ui, &state_input.borrow(), &cur, &q);
                }
            }
        }
    });
    let input_mode_cancel = input_mode.clone();
    ui.on_input_dialog_cancel(move || {
        input_mode_cancel.set(0);
        pending_category_rename.borrow_mut().take();
    });

    ui.on_background_clicked(move || {
        log::debug!("Background clicked");
    });

    // 日志档位切换（设置对话框）：持久化 + 运行时即时生效
    let state_loglvl = state.clone();
    let weak_ui_loglvl = ui.as_weak();
    ui.on_settings_log_changed(move |index| {
        let (level, level_name) = match index {
            1 => ("info", "SI_Log_Std"),
            2 => ("debug", "SI_Log_Dbg"),
            _ => ("warn", "SI_Log_Min"),
        };
        {
            let mut st = state_loglvl.borrow_mut();
            st.manager
                .config_mut()
                .extra_config
                .insert("log_level".into(), level.into());
            st.save_if_modified();
        }
        crate::logger::apply_level(level);
        log::info!("Log level changed to {}", level);
        if let Some(ui) = weak_ui_loglvl.upgrade() {
            let loc = state_loglvl.borrow().loc.clone();
            let name = loc.borrow().tr(level_name).to_string();
            let msg = util::tr_fill(&loc, "Status_LogLevelSet", &name);
            ui_sync::show_toast(&ui, msg, false);
        }
    });

    // Escape：清搜索并隐藏窗口（P2 #11）
    let state_esc = state.clone();
    let weak_ui_esc = ui.as_weak();
    ui.on_escape_pressed(move || {
        log::debug!("Escape pressed");
        if let Some(ui) = weak_ui_esc.upgrade() {
            // Esc：清搜索并刷新当前分类全量条目（统一入口，搜索框同步清空）
            ui_sync::clear_search_and_refresh(&ui, &state_esc);
            // 隐藏前强制落盘几何：托盘退出等路径不会走 on_close_requested
            geometry::flush_window_geometry(&ui, &state_esc);
            let _ = ui.hide();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_debounce_same_item_within_window() {
        let now = std::time::Instant::now();
        let last = ("i1".to_string(), now - std::time::Duration::from_millis(100));
        assert!(click_is_debounced(Some(&last), "i1", now));
    }

    #[test]
    fn test_click_debounce_other_item_not_suppressed() {
        let now = std::time::Instant::now();
        let last = ("i1".to_string(), now - std::time::Duration::from_millis(100));
        assert!(!click_is_debounced(Some(&last), "i2", now));
    }

    #[test]
    fn test_click_debounce_expired_window_runs() {
        let now = std::time::Instant::now();
        let last = ("i1".to_string(), now - std::time::Duration::from_millis(401));
        assert!(!click_is_debounced(Some(&last), "i1", now));
    }

    #[test]
    fn test_click_debounce_no_history_runs() {
        assert!(!click_is_debounced(None, "i1", std::time::Instant::now()));
    }

    #[test]
    fn test_item_menu_build_normal_mode() {
        let loc = Localization::new("zh-CN");
        // ≥2 分类才显示"移动到..."折叠入口
        let cats = vec![
            ("c1".to_string(), "Apps".to_string()),
            ("c2".to_string(), "Games".to_string()),
        ];
        let (texts, actions) = build_item_menu(&loc, false, &cats);
        assert_eq!(texts.len(), actions.len(), "文本与动作必须平行");
        // 固定 8 项 + 上移/下移 + 1 个"移动到..."入口 = 11（与分类数无关）
        assert_eq!(texts.len(), 11);
        assert!(matches!(actions[2], ItemMenuAction::Delete), "第 3 项应为删除");
        let danger_count = actions
            .iter()
            .filter(|a| matches!(a, ItemMenuAction::Delete))
            .count();
        assert_eq!(danger_count, 1, "仅删除项为 danger");
        // 末位为折叠入口；平铺的 MoveTo/旧打开文件夹项不再出现在一级菜单
        assert!(matches!(actions.last(), Some(ItemMenuAction::MoveToMenu)));
        assert!(!actions
            .iter()
            .any(|a| matches!(a, ItemMenuAction::MoveTo(_) | ItemMenuAction::Back)));
        assert_eq!(texts.last().unwrap().as_str(), "移动到...");
    }

    #[test]
    fn test_item_menu_build_search_mode_hides_reorder() {
        let loc = Localization::new("zh-CN");
        let cats = vec![
            ("c1".to_string(), "Apps".to_string()),
            ("c2".to_string(), "Games".to_string()),
        ];
        let (texts, actions) = build_item_menu(&loc, true, &cats);
        assert_eq!(texts.len(), actions.len());
        // 搜索态隐藏上移/下移：11 - 2 = 9 项
        assert_eq!(texts.len(), 9);
        assert!(!actions
            .iter()
            .any(|a| matches!(a, ItemMenuAction::MoveUp | ItemMenuAction::MoveDown)));
        assert!(matches!(actions.last(), Some(ItemMenuAction::MoveToMenu)));
    }

    #[test]
    fn test_item_menu_single_category_hides_move_to() {
        // 单分类：无可移动目标，折叠入口整体隐藏（含搜索态）
        let loc = Localization::new("zh-CN");
        let cats = vec![("c1".to_string(), "Apps".to_string())];
        let (texts, actions) = build_item_menu(&loc, false, &cats);
        assert_eq!(texts.len(), 10, "固定 8 项 + 上移/下移");
        assert!(!actions.iter().any(|a| matches!(a, ItemMenuAction::MoveToMenu)));
        let (texts_s, actions_s) = build_item_menu(&loc, true, &cats);
        assert_eq!(texts_s.len(), 8, "搜索态固定 8 项");
        assert!(!actions_s.iter().any(|a| matches!(a, ItemMenuAction::MoveToMenu)));
    }

    #[test]
    fn test_move_to_menu_build_excludes_current_category() {
        // 二级菜单：首项"返回"，排除条目当前分类，其余按序列出
        let loc = Localization::new("zh-CN");
        let cats = vec![
            ("c1".to_string(), "Apps".to_string()),
            ("c2".to_string(), "Games".to_string()),
            ("c3".to_string(), "Tools".to_string()),
        ];
        let (texts, actions) = build_move_to_menu(&loc, &cats, "c1");
        assert_eq!(texts.len(), actions.len(), "文本与动作必须平行");
        assert_eq!(texts.len(), 3, "返回 + 2 个可移动分类");
        assert_eq!(texts[0].as_str(), "返回");
        assert!(matches!(actions[0], ItemMenuAction::Back));
        assert!(matches!(&actions[1], ItemMenuAction::MoveTo(id) if id == "c2"));
        assert_eq!(texts[1].as_str(), "Games");
        assert!(matches!(&actions[2], ItemMenuAction::MoveTo(id) if id == "c3"));
        // 全部被排除（单分类）时仅剩"返回"
        let (texts_one, actions_one) = build_move_to_menu(&loc, &cats[..1], "c1");
        assert_eq!(texts_one.len(), 1);
        assert!(matches!(actions_one[0], ItemMenuAction::Back));
    }

    /// 构造带 1 分类 3 条目的 AppState（不依赖窗口；storage 指向独立临时文件）。
    fn make_drop_state() -> Rc<RefCell<AppState>> {
        use crate::icon_cache::IconCache;
        use crate::item_manager::ItemManager;
        use crate::model::{AppConfig, AppData, Category, Config, Item};
        use crate::storage::Storage;
        let app_config = AppConfig {
            version: "test".into(),
            data: AppData {
                categories: vec![Category {
                    id: "cat_1".into(),
                    name: "Programs".into(),
                    ..Default::default()
                }],
                items: vec![
                    Item {
                        id: "i1".into(),
                        name: "A".into(),
                        target: "C:\\a.exe".into(),
                        category_id: "cat_1".into(),
                        sort_order: 0,
                        ..Default::default()
                    },
                    Item {
                        id: "i2".into(),
                        name: "B".into(),
                        target: "C:\\b.exe".into(),
                        category_id: "cat_1".into(),
                        sort_order: 1,
                        ..Default::default()
                    },
                    Item {
                        id: "i3".into(),
                        name: "C".into(),
                        target: "C:\\c.exe".into(),
                        category_id: "cat_1".into(),
                        sort_order: 2,
                        ..Default::default()
                    },
                ],
            },
            config: Config::default(),
            extra_config: Default::default(),
        };
        Rc::new(RefCell::new(AppState {
            manager: ItemManager::new(app_config),
            storage: Storage::new(
                std::env::temp_dir()
                    .join(format!("gorun_handlers_drop_{}.json", std::process::id()))
                    .to_string_lossy()
                    .to_string(),
            ),
            icon_cache: IconCache::new(),
            current_category_id: "cat_1".into(),
            search_query: String::new(),
            edit_dialog: None,
            loc: Rc::new(RefCell::new(Localization::new("zh-CN"))),
            save_fail_notified: false,
            pending_icon_paths: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            icon_extract_attempted: std::sync::Mutex::new(std::collections::HashSet::new()),
            drop_handler: None,
            geometry_dirty: false,
            last_geometry_change: None,
            soft_dirty: false,
            last_soft_change: None,
            pending_run_results: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            item_hotkey_mgr: Rc::new(RefCell::new(crate::hotkey::ItemHotkeyManager::new())),
            item_hotkey_tx: {
                let (tx, _rx) = std::sync::mpsc::channel::<String>();
                tx
            },
            low_resident: Rc::new(std::cell::Cell::new(false)),
            auto_hide_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }))
    }

    #[test]
    fn test_apply_item_drop_reorders_and_persists() {
        let state = make_drop_state();
        assert!(apply_item_drop(&state, 0, 2), "有效拖拽应重排成功");
        let order: Vec<String> = state
            .borrow()
            .manager
            .items("cat_1")
            .iter()
            .map(|it| it.id.clone())
            .collect();
        assert_eq!(order, vec!["i2".to_string(), "i3".to_string(), "i1".to_string()]);
    }

    #[test]
    fn test_apply_item_drop_ignores_same_source_target() {
        let state = make_drop_state();
        assert!(!apply_item_drop(&state, 1, 1), "同位拖拽应忽略");
        let order: Vec<String> = state
            .borrow()
            .manager
            .items("cat_1")
            .iter()
            .map(|it| it.id.clone())
            .collect();
        assert_eq!(order, vec!["i1".to_string(), "i2".to_string(), "i3".to_string()]);
    }

    #[test]
    fn test_apply_item_drop_ignores_in_search_mode() {
        let state = make_drop_state();
        state.borrow_mut().search_query = "a".into();
        assert!(!apply_item_drop(&state, 0, 2), "搜索态拖拽应忽略");
        assert_eq!(state.borrow().manager.items("cat_1").len(), 3, "顺序不变");
    }

    #[test]
    fn test_run_all_summary_text_all_success() {
        let loc = Localization::new("zh-CN");
        let msg = run_all_summary_text(&loc, 3, 0);
        assert!(msg.contains("3"), "全部成功模板应含成功数: {}", msg);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_run_all_summary_text_partial_failure() {
        let loc = Localization::new("zh-CN");
        let msg = run_all_summary_text(&loc, 2, 1);
        assert!(
            msg.contains("2") && msg.contains("1"),
            "部分失败模板应同时含成功数与失败数: {}",
            msg
        );
    }
}
