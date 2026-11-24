//! UI 模型同步/刷新（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::app_state::{AppState, RunOutcome};
use crate::icon_cache::IconCache;
use crate::localization::Localization;
use crate::model::{Item, RunError, ViewType};
use crate::{CategoryModel, EditDialog, ItemModel, MainWindow};
use slint::ComponentHandle;

// v2 Toast（替代 StatusBar）：右下角浮出，成功 2s / 错误 4s 自动隐藏。
// Timer 单例存 UI 线程 thread_local（与现有 slint::Timer 使用模式一致）。
thread_local! {
    static TOAST_TIMER: Rc<RefCell<slint::Timer>> = Rc::new(RefCell::new(slint::Timer::default()));
}

pub fn show_toast(ui: &MainWindow, msg: impl Into<slint::SharedString>, error: bool) {
    ui.set_toast_message(msg.into());
    ui.set_toast_error(error);
    ui.set_toast_visible(true);
    let weak = ui.as_weak();
    let dur = if error { 4000 } else { 2000 };
    TOAST_TIMER.with(|t| {
        let timer = t.borrow_mut();
        timer.stop();
        timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(dur),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_toast_visible(false);
                }
            },
        );
    });
}

pub fn spawn_icon_refresh(
    ui: &MainWindow,
    icon_cache: IconCache,
    items: Vec<Item>,
    pending_icon_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
) {
    if items.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        // P1：单线程内预收集所有更新，避免每项一次 invoke_from_event_loop
        // 且每次 O(N) 扫描模型（旧实现总复杂度 O(N²)，大分类时卡顿 UI）。
        // L-11：图标解码同样移到本工作线程（原在 invoke 回调/UI 线程逐项
        // 读盘解码），产出 SharedPixelBuffer（Send+Sync）后仅在 UI 线程做
        // 轻量 Image 包装与行更新。
        let mut updates: Vec<(Item, slint::SharedPixelBuffer<slint::Rgba8Pixel>)> = Vec::new();
        let mut persisted: Vec<(String, String)> = Vec::new();
        for item in &items {
            let Some(path) = icon_cache.refresh_icon(item) else {
                continue;
            };
            let path_str = path.to_string_lossy().to_string();
            // 与背景图同管线（image crate 解码 → RGBA8 → from_rgba8）：
            // 软件渲染器不绘制 load_from_path 产物，且其 to_rgba8 可能取不到
            // 像素，导致内存缓存永远无法回填、图标永久空白。
            match crate::background::decode(&path_str) {
                Some(decoded) => {
                    icon_cache.store_image(item, decoded.buffer.clone());
                    updates.push((item.clone(), decoded.buffer));
                    persisted.push((item.id.clone(), path_str));
                }
                None => {
                    // 解码失败必须留痕（此前静默降级为空图标，线上无从排查）
                    log::warn!("Failed to decode extracted icon {:?}", path_str);
                }
            }
        }
        if updates.is_empty() {
            return;
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                use slint::Model;
                // P0：先在事件循环线程登记缓存路径（供 Timer 回写并落盘），
                // 否则进程重启后图标无从恢复（item_keys 只是内存映射）。
                if let Ok(mut queue) = pending_icon_paths.lock() {
                    for (id, path) in &persisted {
                        queue.insert(id.clone(), path.clone());
                    }
                }
                let model = ui.get_items();
                // 构建 id -> Image 映射做 O(1) 查找，单次遍历模型（O(N)）完成全部更新
                let id_to_img: std::collections::HashMap<&str, slint::Image> = updates
                    .iter()
                    .map(|(item, buffer)| {
                        (item.id.as_str(), slint::Image::from_rgba8(buffer.clone()))
                    })
                    .collect();
                for i in 0..model.row_count() {
                    if let Some(row) = model.row_data(i) {
                        if let Some(img) = id_to_img.get(row.id.as_str()) {
                            let mut new_row = row;
                            new_row.icon_path = img.clone();
                            model.set_row_data(i, new_row);
                        }
                    }
                }
            }
        })
        .ok();
    });
}

/// L-11：磁盘缓存图标后台解码。工作线程读盘+解码（原在 refresh_items_ui 的
/// UI 线程逐项同步解码，大分类首屏一帧以上卡顿），成功回填内存缓存并登记
/// `pending_icon_paths`——房管 Timer 的 persist_cached_icons 随后整表刷新，
/// 此时内存缓存已命中、图标即时上屏（软件渲染器 set_row_data 可能不重绘
/// #13568，整表刷新为既定兜底）。解码失败移除损坏文件并重新提取覆盖
///（对齐 P2-4 损坏缓存自愈语义，不静默降级永久空白）。
fn spawn_icon_decode(
    icon_cache: IconCache,
    jobs: Vec<(Item, String)>,
    pending_icon_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
) {
    if jobs.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        let mut persisted: Vec<(String, String)> = Vec::new();
        for (item, path) in &jobs {
            match crate::background::decode(path) {
                Some(decoded) => {
                    icon_cache.store_image(item, decoded.buffer.clone());
                    persisted.push((item.id.clone(), path.clone()));
                }
                None => {
                    log::warn!("Failed to decode cached icon {:?}, re-extracting", path);
                    let _ = std::fs::remove_file(path);
                    if let Some(new_path) = icon_cache.refresh_icon(item) {
                        if let Some(decoded) = crate::background::decode(&new_path.to_string_lossy()) {
                            icon_cache.store_image(item, decoded.buffer.clone());
                            persisted.push((
                                item.id.clone(),
                                new_path.to_string_lossy().to_string(),
                            ));
                        }
                    }
                }
            }
        }
        if persisted.is_empty() {
            return;
        }
        if let Ok(mut q) = pending_icon_paths.lock() {
            for (id, p) in persisted {
                q.insert(id, p);
            }
        }
    });
}
/// 把后台提取出的图标缓存路径回写到项目模型并落盘（P0）。
/// 必须调用方持有主线程状态；提取线程只负责产出路径，写模型留在事件循环内。
/// 有回写时整表重建条目模型：软件渲染器对 set_row_data 的图片更新可能不重绘
/// （slint ImageCacheMixin 缺陷族 #13568），全量刷新保证异步提取后图标必现。
pub fn persist_cached_icons(
    ui: &MainWindow,
    slot: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    state: &Rc<RefCell<AppState>>,
) {
    let updates: Vec<(String, String)> = {
        // 锁恢复说明：pending_icon_paths 为可丢弃可重建的待回写缓存，锁内只做
        // is_empty/drain、不会 panic；中毒仅在极端情况发生。into_inner 恢复后
        // 可能半更新，但下次保存会按需覆盖，风险可忽略（丢弃重建收益为负）。
        let mut map = match slot.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        if map.is_empty() {
            return;
        }
        map.drain().collect()
    };
    let (changed, cat_id, query) = {
        let mut st = state.borrow_mut();
        let mut changed = false;
        for (item_id, path) in updates {
            if st.manager.set_cached_icon(&item_id, &path) {
                changed = true;
            }
            // P1-2/L-11：提取/解码成功并回写 → 清除会话防抖标记（无论
            // cached_icon 是否变化——解码回填路径 cached_icon 可能本就一致），
            // 日后缓存文件再损坏时允许重新触发提取。
            let mut attempted = st
                .icon_extract_attempted
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let _ = attempted.remove(&item_id);
        }
        if changed {
            st.save_if_modified();
        }
        (
            changed,
            st.current_category_id.clone(),
            st.search_query.clone(),
        )
    };
    // L-11：即使 cached_icon 未变化（重启后解码回填场景）也须整表刷新——
    // 内存缓存刚由解码线程回填，刷新时走内存命中即时上屏（#13568 兜底）
    refresh_items_ui(ui, &state.borrow(), &cat_id, &query);
}

/// 将 RunError 映射为本地化错误文案（P1-4：显示处本地化，替代 runner 内硬编码中文）
pub fn run_error_text(loc: &Rc<RefCell<Localization>>, error: &RunError) -> String {
    let key = match error {
        RunError::None => "Err_None",
        RunError::FileNotFound => "Err_FileNotFound",
        RunError::PathNotFound => "Err_PathNotFound",
        RunError::AccessDenied => "Err_AccessDenied",
        RunError::OutOfMemory => "Err_OutOfMemory",
        RunError::DllNotFound => "Err_DllNotFound",
        RunError::Cancelled => "Err_Cancelled",
        RunError::UnsafeSearchParam => "Err_UnsafeSearchParam",
        RunError::EmptyTarget => "Err_EmptyTarget",
        RunError::Unknown => "Err_Unknown",
    };
    loc.borrow().tr(key).to_string()
}

/// 异步启动条目（M-5）：ShellExecute 移出 UI 线程，避免慢速目标（网络盘/U 盘）
/// 或 UAC 提权弹窗期间冻结界面。结果写入 `pending_run_results` 队列，由 UI 线程
/// 的房管 Timer 经 `drain_run_results` 消费：成功回写运行计数并标脏软保存，
/// 失败弹本地化错误 Toast（文案需 loc，只能在 UI 线程组装）。
pub fn run_item_async(state: &Rc<RefCell<AppState>>, item: Item, search_query: String) {
    let outcomes = state.borrow().pending_run_results.clone();
    // spawn 失败兜底仍需 id/name（item 将被 move 进线程闭包），提前克隆
    let item_id = item.id.clone();
    let item_name = item.name.clone();
    let spawn = std::thread::Builder::new()
        .name("item-run".to_string())
        .spawn(move || {
            // P0-1：勾选"管理员运行"的项目直接走提权路径（勾选即意图，弹 UAC）
            let result = if item.run_as_admin {
                crate::runner::run_as_admin(&item, &search_query)
            } else {
                crate::runner::run(&item, &search_query)
            };
            if result.success {
                log::info!("Launched: {}", item.name);
            } else {
                log::error!("Failed to launch {}: {}", item.name, result.error_message);
            }
            if let Ok(mut q) = outcomes.lock() {
                q.push(RunOutcome {
                    item_id: item.id,
                    item_name: item.name,
                    success: result.success,
                    error: result.error,
                });
            }
        });
    if let Err(e) = spawn {
        // 线程创建失败（极端）：与同步路径失败同口径提示
        log::error!("Failed to spawn run thread: {}", e);
        if let Ok(mut q) = state.borrow().pending_run_results.lock() {
            q.push(RunOutcome {
                item_id,
                item_name,
                success: false,
                error: crate::model::RunError::Unknown,
            });
        }
    }
}

/// 运行指定索引的项目（单击/回车共用）；搜索态按搜索结果取项。
/// 仅负责取项与投递，实际启动异步执行（run_item_async）。
pub fn run_item_at_index(state: &Rc<RefCell<AppState>>, index: i32) {
    let (item, search_query) = {
        let st = state.borrow();
        let items = if st.search_query.is_empty() {
            st.manager.items(&st.current_category_id)
        } else {
            st.manager.search_items(&st.search_query)
        };
        match items.get(index as usize) {
            Some(it) => (it.clone(), st.search_query.clone()),
            None => return,
        }
    };
    run_item_async(state, item, search_query);
}

/// F8/4.1：按 item_id 运行条目（条目级全局热键触发）。搜索参数为空（热键无搜索语境）。
pub fn run_item_by_id(state: &Rc<RefCell<AppState>>, item_id: &str) {
    let item = {
        let st = state.borrow();
        st.manager
            .all_items()
            .iter()
            .find(|i| i.id == item_id)
            .cloned()
    };
    let item = match item {
        Some(i) => i,
        None => {
            log::warn!("Item hotkey: item {} not found", item_id);
            return;
        }
    };
    run_item_async(state, item, String::new());
}

/// 消费异步启动结果队列（UI 线程调用，M-5/M-4 共用）：
/// 成功项回写运行计数并标脏软保存（实际落盘由 flush_soft_if_due 去抖执行）；
/// 失败项原样返回，由调用方用 loc 组装文案提示。
pub fn drain_run_results(state: &Rc<RefCell<AppState>>) -> Vec<RunOutcome> {
    let outcomes = state.borrow().pending_run_results.clone();
    let drained: Vec<RunOutcome> = match outcomes.lock() {
        Ok(mut q) => q.drain(..).collect(),
        Err(e) => e.into_inner().drain(..).collect(),
    };
    if drained.is_empty() {
        return drained;
    }
    let mut st = state.borrow_mut();
    for r in &drained {
        if r.success {
            st.manager.increment_run_count(&r.item_id);
            st.save_soft();
        }
    }
    drained
}

pub fn refresh_items_ui(ui: &MainWindow, state: &AppState, category_id: &str, query: &str) {
    let items = if query.is_empty() {
        state.manager.items(category_id)
    } else {
        state.manager.search_items(query)
    };

    let mut pending: Vec<Item> = Vec::new();
    // L-11：磁盘缓存解码任务队列（后台线程解码，UI 线程只显示占位）
    let mut decode_jobs: Vec<(Item, String)> = Vec::new();
    let ui_items: Vec<ItemModel> = items
        .iter()
        .map(|item| {
            // P1-2 统一图标决策链：
            // 1) 内存像素缓存命中（本会话提取线程回写/此前解码）→ 直接显示；
            // 2) 自定义图标源可解码（png/jpg/...）→ 显示源本身（最保真；手改
            //    iconPath 也不会被 stale 缓存遮挡——缓存键含 iconPath 自动失效）；
            // 3) 源不可解码（exe/dll 自定义源）或无自定义源 → 按键取磁盘缓存
            //    （restored 键校验 / get 键命中），解码入后台队列（L-11）；
            // 4) 缓存缺失或损坏 → 入异步提取队列（会话级防抖），本次显示空白。
            let image = if let Some(buffer) = state.icon_cache.cached_image(item) {
                slint::Image::from_rgba8(buffer)
            } else if !item.icon_path.is_empty() {
                match crate::background::decode(&item.icon_path) {
                    Some(decoded) => {
                        state.icon_cache.store_image(item, decoded.buffer.clone());
                        slint::Image::from_rgba8(decoded.buffer)
                    }
                    None => {
                        // 自定义源非图像格式（exe/dll）：转磁盘缓存/提取兜底。
                        log::debug!(
                            "Custom icon source not decodable ({:?}), falling back",
                            item.icon_path
                        );
                        display_fallback_cache(state, item, &mut pending, &mut decode_jobs)
                    }
                }
            } else {
                display_fallback_cache(state, item, &mut pending, &mut decode_jobs)
            };
            ItemModel {
                id: item.id.clone().into(),
                name: item.name.clone().into(),
                target: item.target.clone().into(),
                icon_path: image,
                search_active: !query.is_empty(),
            }
        })
        .collect();

    // P2-6：整表重建（slint #13568 workaround）会重置滚动位置。条目数不变时
    // （图标回写等触发的重建）恢复滚动偏移，避免浏览中"列表跳回顶部"；
    // 条目数变化（换分类/搜索/增删）视为正常回到顶部。
    // 已知边界：换分类/搜索前后条目数恰相等时也会继承旧滚动位置（Flickable
    // 按新内容高度钳位，无崩溃），体验可接受，换取实现最简。
    {
        use slint::Model;
        let had_rows = ui.get_items().row_count();
        let scroll_y = ui.get_item_scroll_y();
        let row_count = ui_items.len();
        let model = Rc::new(slint::VecModel::from(ui_items));
        ui.set_items(model.into());
        if row_count == had_rows {
            ui.set_item_scroll_y(scroll_y);
        }
    }

    if !pending.is_empty() {
        spawn_icon_refresh(
            ui,
            state.icon_cache.clone(),
            pending,
            state.pending_icon_paths.clone(),
        );
    }
    if !decode_jobs.is_empty() {
        spawn_icon_decode(state.icon_cache.clone(), decode_jobs, state.pending_icon_paths.clone());
    }
}

/// P1-2 兜底链（决策逻辑独立于渲染，可单测）：按键定位磁盘缓存路径。
/// 优先级：cachedIcon 落盘恢复（restored_icon_path 已含键校验）→ 重算键缓存命中。
fn fallback_cache_path(cache: &IconCache, item: &Item) -> Option<String> {
    cache
        .restored_icon_path(item)
        .or_else(|| cache.get_icon_path(item))
        .map(|p| p.to_string_lossy().into_owned())
}

/// P1-2：从兜底缓存定位图标；缓存缺失入异步提取队列（会话级防抖），
/// 缓存存在则入后台解码队列（L-11，本次显示占位、解码完成后整表刷新上屏）。
fn display_fallback_cache(
    state: &AppState,
    item: &Item,
    pending: &mut Vec<Item>,
    decode_jobs: &mut Vec<(Item, String)>,
) -> slint::Image {
    match fallback_cache_path(&state.icon_cache, item) {
        Some(p) => {
            // L-11：解码移出 UI 线程——入后台解码队列；损坏缓存的自愈
            //（移除+重提取）由解码工作线程处理（spawn_icon_decode）
            decode_jobs.push((item.clone(), p));
            slint::Image::default()
        }
        None => {
            // 诊断：无自定义图标且无缓存来源，交由异步提取队列兜底
            // （线上排查"拖入/显示无图标"的关键留痕）
            log::debug!(
                "Icon source none (pending extract): id={}, name={}",
                item.id,
                item.name
            );
            queue_extraction(state, item, pending);
            slint::Image::default()
        }
    }
}

/// P1-2 会话级提取防抖：同一 item 本会话仅自动入队一次（提取失败/无源
/// 不再每次刷新重复 spawn 提取线程）；标记清除点：提取成功回写
/// （persist_cached_icons）、编辑条目（handlers）、手动刷新（handlers）。
fn queue_extraction(state: &AppState, item: &Item, pending: &mut Vec<Item>) {
    queue_extraction_marked(&state.icon_extract_attempted, item, pending);
}

/// 防抖核心（可测）：首次入队并打标返回 true；已打标（本会话已尝试）
/// 不再入队返回 false。
fn queue_extraction_marked(
    attempted: &std::sync::Mutex<std::collections::HashSet<String>>,
    item: &Item,
    pending: &mut Vec<Item>,
) -> bool {
    let mut m = attempted.lock().unwrap_or_else(|e| e.into_inner());
    if m.insert(item.id.clone()) {
        pending.push(item.clone());
        true
    } else {
        false
    }
}

pub fn set_edit_dialog_texts(dlg: &EditDialog, loc: &Rc<RefCell<Localization>>) {
    let lb = loc.borrow();
    dlg.set_title_text(lb.tr("Title_EditItem").into());
    dlg.set_label_name(format!("{}:", lb.tr("STR_Name")).into());
    dlg.set_label_target(format!("{}:", lb.tr("STR_Target")).into());
    dlg.set_label_arguments(format!("{}:", lb.tr("STR_Arguments")).into());
    dlg.set_label_directory(format!("{}:", lb.tr("STR_WorkDir")).into());
    dlg.set_label_icon_path(format!("{}:", lb.tr("STR_IconPath")).into());
    dlg.set_label_icon_index(format!("{}:", lb.tr("STR_IconIndex")).into());
    dlg.set_label_keywords(format!("{}:", lb.tr("STR_Keywords")).into());
    dlg.set_label_remark(format!("{}:", lb.tr("STR_Remark")).into());
    dlg.set_label_hotkey(format!("{}:", lb.tr("STR_HotKey")).into());
    dlg.set_label_category(format!("{}:", lb.tr("STR_Category")).into());
    dlg.set_browse_text(format!("{}...", lb.tr("Btn_Browse")).into());
    dlg.set_run_as_admin_text(lb.tr("STR_RunAsAdmin").into());
    dlg.set_save_text(lb.tr("Btn_Save").into());
    dlg.set_cancel_text(lb.tr("Btn_Cancel").into());
}

/// 浏览选择目标后自动回填工作目录（新建时同时填名称）。
pub fn fill_from_target(d: &EditDialog, path: &str, overwrite_name: bool) {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        let dir = parent.to_string_lossy().to_string();
        if !dir.is_empty() {
            d.set_item_working_dir(dir.into());
        }
    }
    if overwrite_name {
        let name = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.is_empty() {
            d.set_item_name(name.into());
        }
    }
}

/// 重建分类侧边栏模型并写回 UI。
pub fn refresh_categories_ui(ui: &MainWindow, state: &AppState) {
    let categories: Vec<CategoryModel> = state
        .manager
        .categories()
        .iter()
        .map(|c| CategoryModel {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
            count: state.manager.items(&c.id).len() as i32,
        })
        .collect();
    ui.set_categories(Rc::new(slint::VecModel::from(categories)).into());
    // 侧栏选中态与当前分类保持同步（键盘/程序化变更后刷新）
    if let Some(idx) = state
        .manager
        .categories()
        .iter()
        .position(|c| c.id == state.current_category_id)
    {
        ui.set_sidebar_selected_index(idx as i32);
    }
}

/// 统一"切换当前分类"入口（H-1/H-2 修复）：同步 current_category_id、
/// 侧栏选中态、视图模式，并按 `keep_search` 处理搜索态。
/// - `keep_search=true`（侧栏点击/滚轮切页）：保留 `search_query`，列表继续
///   显示全局搜索结果，与 `run_item_at_index` 的取项集合保持一致（索引不错位）；
/// - `keep_search=false`（导入配置/新建分类等"数据集已更换"场景）：清空搜索态
///   并同步搜索框文本，界面显示新分类全量条目——保证"显示集合 ≡ 运行取项集合"，
///   杜绝"点 A 却启动 B"的索引错位。
pub fn switch_category(
    ui: &MainWindow,
    state: &Rc<RefCell<AppState>>,
    cat_id: &str,
    keep_search: bool,
) {
    let (is_icon, sidebar_idx, query) = {
        let mut st = state.borrow_mut();
        st.current_category_id = cat_id.to_string();
        if !keep_search {
            st.search_query.clear();
        }
        let sidebar_idx = st
            .manager
            .categories()
            .iter()
            .position(|c| c.id == cat_id)
            .map(|i| i as i32);
        let is_icon = st
            .manager
            .categories()
            .iter()
            .find(|c| c.id == cat_id)
            .map(|c| matches!(c.view_type, ViewType::Icon))
            .unwrap_or(false);
        (is_icon, sidebar_idx, st.search_query.clone())
    };
    if let Some(idx) = sidebar_idx {
        ui.set_sidebar_selected_index(idx);
    }
    ui.set_is_icon_view(is_icon);
    if !keep_search {
        ui.set_search_text("".into());
    }
    refresh_items_ui(ui, &state.borrow(), cat_id, &query);
}

/// 清空搜索态并刷新当前分类全量条目（Esc 等"离开搜索态"路径共用）。
pub fn clear_search_and_refresh(ui: &MainWindow, state: &Rc<RefCell<AppState>>) {
    ui.set_search_text("".into());
    state.borrow_mut().search_query.clear();
    let cat_id = state.borrow().current_category_id.clone();
    refresh_items_ui(ui, &state.borrow(), &cat_id, "");
}

/// F8/4.1：同步条目级全局热键——收集所有非空 item.hotkey，重注册到 ItemHotkeyManager。
/// 条目增删改/导入后调用；空列表时注销全部（无条目热键）。
/// 返回注册失败的热键串列表（空 = 全部成功），供调用方向用户反馈冲突。
pub fn sync_item_hotkeys(state: &AppState) -> Vec<String> {
    let hotkeys: Vec<(String, String)> = state
        .manager
        .all_items()
        .iter()
        .filter(|i| !i.hotkey.trim().is_empty())
        .map(|i| (i.id.clone(), i.hotkey.clone()))
        .collect();
    let tx = state.item_hotkey_tx.clone();
    let failed = state
        .item_hotkey_mgr
        .borrow_mut()
        .register_all(&hotkeys, tx);
    if !failed.is_empty() {
        log::warn!("Item hotkey registration failed for: {}", failed.join(", "));
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_error_text_maps_each_error_to_localized_key() {
        let loc = Rc::new(RefCell::new(Localization::new("zh-CN")));
        assert_eq!(run_error_text(&loc, &RunError::None), "成功");
        assert_eq!(
            run_error_text(&loc, &RunError::FileNotFound),
            "找不到指定的文件"
        );
        assert_eq!(
            run_error_text(&loc, &RunError::PathNotFound),
            "找不到指定的路径"
        );
        assert_eq!(run_error_text(&loc, &RunError::AccessDenied), "访问被拒绝");
        assert_eq!(run_error_text(&loc, &RunError::OutOfMemory), "内存不足");
        assert_eq!(
            run_error_text(&loc, &RunError::DllNotFound),
            "找不到指定的 DLL 文件"
        );
        assert_eq!(run_error_text(&loc, &RunError::Cancelled), "已取消提权请求");
        assert_eq!(
            run_error_text(&loc, &RunError::UnsafeSearchParam),
            "搜索参数含不安全命令字符，已拒绝执行"
        );
        assert_eq!(
            run_error_text(&loc, &RunError::EmptyTarget),
            "目标为空，请先编辑该条目设置目标"
        );
        assert_eq!(run_error_text(&loc, &RunError::Unknown), "未知错误");
    }

    #[test]
    fn test_fallback_cache_path_hit_and_miss() {
        let cache = crate::icon_cache::IconCache::new();
        let item = Item {
            id: "fb1".into(),
            ..Default::default()
        };
        let p = cache.cache_path(&item);
        let _ = std::fs::remove_file(&p); // 自愈清理：上次运行中断残留不影响本测试
        // 无缓存文件 → None
        assert!(fallback_cache_path(&cache, &item).is_none());
        // 键文件落盘（模拟提取成功）→ 命中
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"png").unwrap();
        assert_eq!(
            fallback_cache_path(&cache, &item),
            Some(p.to_string_lossy().into_owned())
        );
        // 删除后恢复 None
        cache.delete_cache(&item);
        assert!(fallback_cache_path(&cache, &item).is_none());
    }

    #[test]
    fn test_fallback_cache_path_stale_cached_icon_falls_back_to_key() {
        // stale cachedIcon（旧键路径）被键校验拒绝后，落回新键缓存命中
        let cache = crate::icon_cache::IconCache::new();
        let mut item = Item {
            id: "fb2".into(),
            ..Default::default()
        };
        let p = cache.cache_path(&item);
        let _ = std::fs::remove_file(&p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"png").unwrap();
        // 手改 target 前的旧键路径残留于 cachedIcon（键校验必拒绝）
        item.cached_icon = "C:\\stale\\old_key.png".into();
        let got = fallback_cache_path(&cache, &item).expect("应命中重算键缓存");
        assert_eq!(got, p.to_string_lossy());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_queue_extraction_session_debounce() {
        let attempted: std::sync::Mutex<std::collections::HashSet<String>> =
            std::sync::Mutex::new(std::collections::HashSet::new());
        let mut pending: Vec<Item> = Vec::new();
        let item = Item {
            id: "q1".into(),
            ..Default::default()
        };
        // 首次入队并打标
        assert!(queue_extraction_marked(&attempted, &item, &mut pending));
        assert_eq!(pending.len(), 1);
        // 会话内第二次被防抖
        assert!(!queue_extraction_marked(&attempted, &item, &mut pending));
        assert_eq!(pending.len(), 1);
        // 清除标记（模拟回写成功/手动刷新）后可再入队
        attempted.lock().unwrap().remove("q1");
        assert!(queue_extraction_marked(&attempted, &item, &mut pending));
        assert_eq!(pending.len(), 2);
    }
}
