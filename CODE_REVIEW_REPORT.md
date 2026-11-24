# GoRun 项目审查报告

- **审查对象**：`/workspace/GoRun`（Windows 快速启动器，Rust 2021 + Slint 1.18，主线 `rs/`）
- **代码规模**：Rust 源码约 13,830 行（30 个文件）+ Slint UI 约 3,478 行 + 测试 1,280 行
- **审查方式**：按要求以**最高并发 1**（即串行）方式执行 **10 轮连续审查**，每轮聚焦一个专题，最后交叉复检并汇总去重
- **审查人**：Reasonix（自主目标模式）
- **报告日期**：2026-11-24（基于 commit `278b8df`）

---

## 0. 执行摘要

GoRun 整体代码质量**高于同类个人项目的平均水平**：模块拆分清晰（P0~P3 重构痕迹完整）、注释翔实且解释"为什么"、错误路径大多有兜底、安全设计（注入防护、路径校验、原子写、备份轮转、日志脱敏）有成体系考量，测试覆盖面（单元 + 集成 + GUI + E2E）在 GUI 桌面项目里算扎实。

本轮审查共记录 **36 项发现**：**高危 4 项、中危 20 项、低危/建议 12 项**（已去重、已交叉复核）。没有发现可被远程利用的漏洞——本项目是本地单机应用，攻击面主要是"恶意配置文件导入"与"本机同权限进程"，现有 P0-1 注入防护、路径校验、键校验等机制已覆盖主要风险。

需要优先处理的是 4 项高危：**图标提取的线程模型（GDI/Shell API 在无窗口工作线程调用）**、**拖入文件时 UI 线程同步提取图标造成卡顿**、**缓存整表清空策略导致的读盘风暴**、**最大化状态保存/恢复缺陷**。

> ⚠️ 局限性：本审查环境无 `cargo`/`rustc`，全部结论来自静态阅读，**未做编译与运行验证**。仓库自带 Windows CI（clippy `-D warnings` + 单元/集成/GUI 测试），建议把"待验证"项放入该 CI 确认。

---

## 1. 项目功能概览（审查中确认的实际能力）

| 功能域 | 实现状态 | 关键文件 |
|---|---|---|
| 窗口框架 | ✅ Slint 原生渲染，无 WebView/Electron；最小 360×240，几何记忆 + 离屏检测 + DPI 换算 | `main.rs` `geometry.rs` `util.rs` |
| 分类管理 | ✅ 增删改、右键上下移、拖拽排序、至少保留一个分类、级联删除 | `item_manager.rs` `handlers/mod.rs` |
| 项目管理 | ✅ 添加/编辑/删除、单击运行（400ms 同项防抖）、管理员运行确认、拖拽排序、移动到分类 | `item_manager.rs` `ui_sync.rs` |
| 程序启动 | ✅ ShellExecuteExW 异步执行、.lnk 解析、PATH 裸名解析、相对路径解析、`%mp%/%mr%/%so%/%so-url%` 变量、UAC 提权 | `runner.rs` |
| 全局快捷键 | ✅ 默认 Ctrl+/（含历史迁移链 Ctrl+Alt+M→Ctrl+M→Ctrl+/）、条目级热键（每条目可配） | `hotkey.rs` |
| 托盘 | ✅ 显隐切换、开机自启（HKCU Run，引号防劫持）、跟随鼠标、低驻留模式、TaskbarCreated 重建 | `tray.rs` `tray_ui.rs` |
| 数据持久化 | ✅ JSON 便携优先（exe 同级，回退 %APPDATA%）、原子写+fsync+rename、单份 .bak1、损坏隔离 `.corrupt.<ts>`、孤儿条目归置、旧版迁移 | `storage.rs` `config.rs` `model.rs` |
| 图标缓存 | ✅ FNV-1a64 键、异步提取 + 内存缓存 + 磁盘缓存、cachedIcon 落盘恢复 + 键校验、损坏自愈、GC | `icon_cache.rs` `ui_sync.rs` |
| 搜索 | ✅ 子串 + 拼音首字母/全拼（带缓存）、按使用次数排序、300ms 防抖 | `item_manager.rs` |
| 主题/背景 | ✅ 深/浅色令牌、自定义背景图（最长边 2560 封顶）+ 暗化遮罩 | `theme.slint` `background.rs` |
| 导入导出 | ✅ JSON 导出/导入，保留会话偏好（language/follow_mouse/auto_hide/low_resident） | `handlers/config_io.rs` |
| 多语言 | ✅ 固定简体中文（编译期嵌入 zh-CN.json，v2 决策） | `localization.rs` |
| 设置 | ✅ 背景图、遮罩浓度、日志档位（warn/info/debug）、光标离开自动隐藏 | `settings_dialog.slint` `handlers/mod.rs` |

文档（README/AGENTS）与代码一致性抽查**通过**：默认列表视图、FNV-1a64、原子写+单备份、孤儿归置、热键迁移链、P0-1 注入防护、tmp 清理策略等声明均能在代码中找到对应实现。

---

## 2. 高危问题（High，建议尽快处理）

### H1. 图标提取在无窗口工作线程调用 GDI/Shell API，线程模型存疑
- **位置**：`rs/src/ui_sync.rs` `spawn_icon_refresh`/`spawn_icon_decode` → `rs/src/icon_cache.rs` `refresh_icon` → `extract_hicon`/`extract_hicon_from_target`/`save_hicon_as_png`
- **问题**：图标提取与 PNG 保存被移到 `std::thread::spawn` 的工作线程执行，其中 `save_hicon_as_png` 调用 `GetDC(None)`/`CreateCompatibleDC`/`GetDIBits`/`GetIconInfo`，提取调用 `PrivateExtractIconsW`/`SHGetFileInfoW`。这些 GDI/Shell 调用大多"能"跨线程，但 `GetDC(None)` 取屏幕 DC、GDI 对象句柄的线程亲和性、以及 Shell 图标缓存接口并非在所有线程环境下都有文档保证；工作线程也未 `CoInitialize`（`SHGetFileInfoW` 部分路径对 COM 初始化有隐含要求）。
- **影响**：偶发提取失败、图标空白、或极小概率 GDI 资源异常；症状为"有的机器图标正常、有的机器空白"，难以复现排查。
- **建议**：
  1. 在 Windows CI/真机上做专项验证（大量条目 + 频繁刷新）；
  2. 稳妥做法是把提取放到**专用常驻工作线程**（启动时 `CoInitializeEx`，或用隐藏消息窗口线程串行处理提取请求）；
  3. 至少在工作线程入口补 `CoInitializeEx(COINIT_APARTMENTTHREADED)`（或 MULTITHREADED，按 Shell 要求），失败时记录日志。

### H2. 拖入文件在 UI 线程同步提取图标（≤8 个），可卡顿界面
- **位置**：`rs/src/handlers/mod.rs` `on_file_dropped`（`SYNC_ICON_LIMIT = 8` 分支直接循环 `icon_cache.refresh_icon(&item)`）
- **问题**：`refresh_icon` 内部是 `PrivateExtractIconsW` + 48px 位图解码 + PNG 编码 + 磁盘写 + fsync 级 rename。从网络盘/U 盘/慢速磁盘拖入 8 个文件时，UI 线程会被阻塞数百毫秒到数秒——与项目自己在 M-5 中"把 ShellExecute 移出 UI 线程避免冻结"的动机直接冲突。
- **影响**：拖放时界面掉帧/卡死感；最坏情况（网络路径超时）卡住数秒。
- **建议**：删除同步捷径，统一走 `spawn_icon_refresh` 异步管线（已有会话级防抖 + 整表刷新兜底）；若要保留"拖入即见图标"体验，可对 1~2 个文件保留同步、其余异步。

### H3. `store_image`/拼音缓存超限"整表清空"，制造读盘风暴
- **位置**：`rs/src/icon_cache.rs` `store_image`（`IMAGE_CACHE_MAX=512`）、`rs/src/item_manager.rs` `pinyin_match`（`PINYIN_CACHE_MAX=4096`）
- **问题**：缓存满后 `map.clear()` 再插入。512 项图标像素缓存一旦触发清空，下一次列表刷新会对所有条目重新读盘+解码（L-11 已把解码移到后台，但仍是集中式读盘风暴）；拼音缓存同理。
- **影响**：条目数在阈值附近时出现周期性卡顿尖峰；用户体验为"偶尔整屏图标转圈"。
- **建议**：改为 LRU（`lru` crate 或手写双向链表），或至少"清一半"而非整表清空；拼音缓存条目小，保持现状可接受。

### H4. 窗口最大化状态未被记录，重启后以"超大普通窗口"恢复
- **位置**：`rs/src/geometry.rs` `save_window_geometry_with` + `rs/src/main.rs` 几何恢复
- **问题**：保存时只记录 position/size，不记录 maximized/fullscreen 状态。窗口最大化时 `window().position()/size()` 返回的是最大化后的物理矩形（如 2560×1440），保存后重启会按该尺寸恢复为**普通窗口**（且 `clamp_size_to_virtual_screen` 只在超 80% 虚拟屏时缩放，最大化尺寸恰好 ≤ 屏幕尺寸，不会被缩）。最小化已有防护（`is_minimized()` + 哨兵坐标），最大化没有。
- **影响**：用户习惯最大化使用时，每次重启都变成占满屏幕的普通窗口，需再次手动最大化。
- **建议**：在 extra_config 增加 `window_maximized` 标志，`is_maximized()` 时置位；恢复时 `ShowWindow(SW_MAXIMIZE)` 或 slint `maximize()`。

---

## 3. 中危问题（Medium）

| # | 位置 | 问题 | 建议 |
|---|---|---|---|
| M1 | `storage.rs` `cleanup_stale_tmp` / `icon_cache.rs` `cleanup_and_gc` | 以"文件 mtime < 本进程启动时刻"判定残留；**系统时钟回拨**（NTP 校正）时可能误删并发实例正在写的 tmp/新缓存 | 改用创建时间（Windows 有 CreationTime），或跳过"mtime 在未来 5 分钟内"的文件 |
| M2 | `storage.rs` `load()` extra_config | 非字符串 JSON 值被 `to_string()` 成 "1"/"true"；而布尔偏好统一按 `=="1"` 判定，手写 `"true"` 会被判 false | 文档明确配置值规范；或 `extra_get_bool` 兼容 "true"/"1" |
| M3 | `handlers/config_io.rs` `import_config` | 导入后 `save_if_modified()` 失败时仅提示，**内存数据已替换**而磁盘还是旧数据，UI 与磁盘不一致直到下次成功保存 | 保存失败时恢复导入前的 AppConfig 快照再提示 |
| M4 | `ui_sync.rs` `refresh_items_ui` | 自定义 `icon_path`（png/jpg）命中不了内存缓存时在 **UI 线程同步 `background::decode`**；自定义图标多时首屏卡顿 | 与 L-11 一致：解码任务也丢给后台队列，UI 先显示占位 |
| M5 | `tray.rs` `TrayGlobals` | `unsafe impl Send/Sync` 的安全性依赖"托盘窗口线程 = 创建线程"这一未成文约束；`OnceLock<HWND/HICON>` 跨线程只读成立，但若未来有人在其他线程 `popup_tray_menu` 会踩坑 | 在 `TrayGlobals` 文档与 `send`/`sync` 注释中写明约束；或改用 `ThreadId` 断言 |
| M6 | `main.rs` 单实例互斥 | `CreateMutexW` 失败（`GetLastError` 非 ALREADY_EXISTS）一律按"创建失败"退出；`Local\GoRun_SingleInstance` 名字被占用/权限异常时会误退出 | 区分 ERROR_ALREADY_EXISTS 与其他错误；其他错误记录日志后可继续（降级为允许多实例）或明确提示 |
| M7 | `hotkey.rs` `register_with_notifier` | UI 线程 `result_rx.recv_timeout(5s)` 等待 pump 注册结果；极端情况下改键时 UI 冻结最多 5s | 缩短到 1~2s；或注册结果改异步回调 |
| M8 | `fileops.rs` 对话框 | `GetOpenFileNameW/GetSaveFileNameW` 的 `hwndOwner = HWND::default()`（无 owner），对话框可能不置顶、可被主窗口遮挡 | 传入主窗口 HWND（需把 hwnd 传到 fileops 层） |
| M9 | `fileops.rs` | 未调用 `CommDlgExtendedError`，"用户取消"与"发生错误"都返回 None，错误被静默吞掉 | 错误时 log 一条（不影响返回值语义） |
| M10 | `icon_cache.rs` `cleanup_and_gc` | 启动时 GC 以 live_files（cachedIcon 文件名集合）判定孤儿；刚提取完但尚未回写 cachedIcon 的 PNG 有极小概率被误删（随后自愈重提取） | GC 时把 `pending_icon_paths` 里的文件名也并入 live 集合 |
| M11 | `geometry.rs` | 右键菜单定位用 `GetCursorPos` 而非事件携带的 x/y（`position_menu_at_cursor` 里 `let _ = (x, y);`）——多显示器/窗口跨屏时光标坐标换算依赖虚拟屏假设，基本正确，但 DPI 混合精度场景可能偏差 | 保留现状可接受；如需精确可用事件坐标换算 |
| M12 | `main.rs` 热键消费线程 | `while hk_rx.recv().is_ok()` 线程无 join、退出靠进程结束；`invoke_from_event_loop` 失败被 `.ok()` 吞掉 | 可接受；若追求整洁可在 quit 时 drop sender 并 join |
| M13 | `tray_ui.rs` `start_tray_handler` | 托盘消息线程 `rx.recv()` 循环，Quit 时 `quit_event_loop` 后 break——若 Quit 前通道先断（TrayIcon drop），线程自然结束；OK。但 `invoke_from_event_loop` 在事件循环结束后会失败并被吞，日志缺失 | 失败时补 debug 日志，便于排查退出阶段丢动作 |
| M14 | `handlers/mod.rs` `on_item_right_clicked` | 菜单项与动作数组在右键时构建、点击时消费；若期间条目集合变化（异步删除/导入），`items.get(item_idx)` 可能取到不同条目——已有搜索态排除 MoveUp/Down，但 Edit/Run 等仍按索引取项 | 改为在菜单构建时捕获 item_id，点击时按 id 取项（与防抖逻辑同思路），彻底消除索引错位 |
| M15 | `item_manager.rs` `search_items` | 每次搜索对每个条目做 3 次 `to_lowercase` + 拼音匹配；数千条目时每次按键 O(N·L)。已有 300ms 防抖与拼音缓存 | 条目 >2000 的用户可感知；可预建小写索引或增量搜索 |
| M16 | `runner.rs` `url_encode` | 空格编码为 `+`（表单编码），路径段语义下 `%20` 更标准 | 保留亦可；如需严格 URL 语义改 `%20` |
| M17 | `runner.rs` `shell_kind` | 覆盖 cmd/bat/cmd/powershell/pwsh/ps1/mshta/wscript/cscript/vbs/js/jse/hta/wsf/wsh，较全面但非穷尽（如 autoit、.psm1 模块、第三方解释器） | 维持黑名单 + 文档说明"防护针对常见注入面"；高危场景建议用户避免 `%so%` 进 shell 目标 |
| M18 | `main.rs` `flush_geometry_if_due(true)` 等退出路径 | 事件循环结束后 `state.borrow_mut()` 执行落盘——若落盘耗时（fsync），进程退出延迟数百 ms，无感知问题 | 可接受；记录即可 |
| M19 | `storage.rs` `load()` | `de_i32_str_or_num` 对字符串解析失败会整文件解析失败 → 走"损坏隔离"；对手改配置（如 `"windowX": "abc"`）过于严苛 | 可改为解析失败回退默认值 + warn 日志，避免整文件被隔离 |
| M20 | `logger.rs` | 日志无上限保留策略只有 10MB 轮转 + 一份 .old，OK；但 `--debug` 分配控制台后无释放（进程退出回收） | 可接受 |

---

## 4. 低危 / 建议项（Low）

| # | 位置 | 说明 |
|---|---|---|
| L1 | `item_grid.slint` `grid-content-height` | 空列表时计算可为负（`Math.floor((0+cols-1)/cols)*step - step + 2*pad`），依赖 Flickable 钳制；建议 `max(0px, ...)` |
| L2 | `sidebar.slint` 滚轮累积 | 阈值 40px 对高分辨率/平滑滚轮可能偏慢；可设为可调或按 delta 归一化 |
| L3 | `edit_dialog.slint` | 固定 500×500，11 行内容在低 DPI 小窗口可能贴边；建议改随内容/屏幕自适应 |
| L4 | `util.rs` `sanitize_for_log` | 仅折叠 `\n`/`\r`，而 `logger::sanitize` 更全面；建议统一复用 logger 版本 |
| L5 | `build.rs` | `OUT_DIR`/`CARGO_MANIFEST_DIR` 直接 `unwrap()`，环境异常时 panic 信息不友好；改 `expect("...")` |
| L6 | CI（`.github/workflows/rust-build.yml`） | 已有 clippy `-D warnings` + 单元 + storage + Windows 原生测试；建议补 `cargo fmt --check` 与 `cargo deny/audit`（供应链检查） |
| L7 | `main_window.slint` Esc 分层 | 弹窗→菜单→清搜索→隐藏，处理完善；建议为 Esc 隐藏路径也提供设置开关（当前仅"光标离开自动隐藏"） |
| L8 | `localization.rs` | `lang_code` 仅存不用（v2 固定中文）；API 保留合理，建议注释已说明 |
| L9 | `runner.rs` `is_search_param_safe` | 黑名单含 `& | < > ^ ( ) % ! " ; $ \` \r \n`，未含单引号（cmd/PS 单引号内安全），整体合理；文档可补充"为什么这些字符危险" |
| L10 | `icon_cache.rs` `cache_key` | FNV-1a64 含 id+icon_path+icon_index+target，编辑自动失效 + restored 键校验，设计良好；哈希碰撞仅致图标错乱（可接受，概率极低） |
| L11 | `platform/dragdrop.rs` | `DragQueryFileW` 两段式长度查询 + `from_utf16_lossy`，正确；`ReleaseStgMedium` 归还及时 |
| L12 | `clipboard.rs` | RAII ClipboardGuard + GlobalAlloc/SetClipboardData 所有权处理正确；复制内容不落日志（只记长度），隐私意识好 |

---

## 5. 安全专项结论

威胁模型：本地单机工具，配置文件可被用户/外部程序改写；导入他人配置文件；运行用户配置的任意目标程序。

| 检查项 | 结论 |
|---|---|
| 命令注入（`%so%` 进 shell 型目标） | ✅ 已防护：`uses_search_var` 覆盖 `%so%` 与 `%so-url%`；`shell_kind` 识别 cmd/bat/cmd/powershell/pwsh/ps1/mshta/wscript/cscript 及脚本扩展名；`is_search_param_safe` 黑名单拦截元字符；env 展开先于搜索替换防 `%VAR%` 借道 |
| 路径穿越（图标缓存键） | ✅ `cache_key` 为哈希十六进制文件名，`cache_path` 恒在 `icons/` 下（有测试 `test_cache_path_no_traversal`） |
| 缓存键伪造/stale 图标 | ✅ `restored_icon_path` 要求 cachedIcon 与重算键一致，否则走重新提取自愈 |
| 开机自启劫持 | ✅ 写 HKCU Run 时 exe 路径加引号，防同名影子二进制 |
| 日志注入/隐私 | ✅ logger 折叠控制字符；复制内容只记长度；Debug 档才记搜索词/路径，默认 Info |
| 配置文件完整性 | ⚠️ 单机信任模型，无签名（AGENTS.md 已明示理由：HMAC 密钥可逆、收益趋零）；靠原子写+备份+隔离兜底——**可接受的设计决策** |
| 导入恶意配置 | ⚠️ `Storage::load` 有 64MB 上限防 OOM；但条目 `target` 可指向任意程序/URL（功能本身如此）；`%so%` 防护缓解了注入面。建议：导入时可对"含 `%so%` 且目标为 shell 型"的条目给出醒目警告 |
| 超大图片 DoS | ✅ 背景图解码前读尺寸，>10240 边直接拒绝；解码后 2560 封顶 |
| UAC 提权 | ✅ runas 走 ShellExecuteExW，取消映射为 Cancelled；确认弹窗先行 |
| 竞态/TOCTOU | ✅ 单实例互斥 + 单 UI 线程状态 + Mutex 队列；tmp 唯一名 + rename 原子替换 |

**总体安全评价**：在"单机信任模型"的既定前提下，安全设计完整且务实；无远程攻击面。唯一值得加强的是导入配置时对高危条目（shell 目标 + 搜索变量）的显式告警。

---

## 6. 性能专项结论

- **做得好的**：搜索单遍历合并 filter+map（原 6 次 to_lowercase → 1 次）；图标更新 O(N) 单遍 + id→image 哈希映射（原 O(N²)）；整表重建保留滚动位置；几何/软保存双去抖（2s）避免高频 fsync；拼音文本级缓存；run-all 批间 50ms 让出；低驻留模式隐藏时暂停 500ms Timer 非必要工作。
- **主要瓶颈**：见 H2（拖入同步提取）、H3（缓存整表清空）、M4（自定义图标 UI 线程解码）、M15（大条目数搜索）。
- **I/O 放大控制**：结构性变更才轮转 .bak1，runCount/几何走 soft save 不轮转——设计正确，避免"运行→退出"冲刷备份。

---

## 7. 稳定性 / 并发专项结论

- **线程清单**：主 UI 线程（Slint 事件循环）、hotkey-pump、item-hotkey-pump、托盘消息线程、托盘 UI 桥线程、拖放消费线程、图标提取/解码工作线程（按需 spawn）、item-run 工作线程（按需）、run-all 工作线程。
- **同步原语**：`Mutex`（pending_icon_paths / pending_run_results / icon_extract_attempted / tray texts / run-all progress）、`AtomicBool`（auto_hide / cancel / follow_mouse / low_resident / LEFT_SENT）、`mpsc`（热键/托盘/拖放）、`OnceLock`（base_dir / tray globals）。
- **锁中毒恢复**：所有可恢复缓存锁都用 `unwrap_or_else(|e| e.into_inner())` 并附理由注释——策略一致、合理。
- **panic 面**：非测试代码仅 `hotkey.rs:136` 一处 `expect`（单字节字符串必有字符，逻辑上不可失败）；无 `panic!`/`todo!`/`unreachable!`；`panic = "abort"`（release）+ panic_hook 记录日志。
- **主要风险**：H1（工作线程 GDI/Shell 线程亲和性）、M5（TrayGlobals Send/Sync 依赖未成文约束）、M12/M13（后台线程退出阶段 invoke 失败被吞）。

---

## 8. 测试与 CI 评价

- `storage_test.rs`（251 行）：往返、损坏、备份轮转/恢复、tmp 无残留、原子写——扎实。
- `gui_integration_test.rs`（1029 行）：渲染、回调、E2E 增删搜持久化、级联删除、托盘开关、编辑对话框基线（P2-5 锁定行为）、H-1/H-2 索引一致性回归——覆盖面好。
- 单元测试内嵌各模块（runner 注入防护、几何 clamp/DPI、拼音、.url 解析、日志档位、导入会话偏好过滤、图标缓存键）。
- CI：两个 Windows job（build+clippy+unit+storage+artifact；原生平台测试）+ Linux 侧 clippy 基线；**缺 fmt 检查与依赖审计**。
- 缺口：图标提取路径（H1）无自动化测试（可理解，GDI 难测）；建议至少加一个"多线程并发 refresh_icon 不 panic、产物可解码"的冒烟测试。

---

## 9. 修复优先级路线图

1. **立即**：H2 拖入同步提取改异步（改动小、收益直接）；M14 右键菜单按 item_id 取项（消除索引错位残留风险）。
2. **短期**：H3 缓存改 LRU；M4 自定义图标后台解码；H4 最大化状态持久化；M3 导入失败回滚。
3. **中期**：H1 图标提取线程模型专项验证与改造（专用提取线程 + CoInitialize）；M1 时钟回拨防御；M6 单实例错误区分；CI 补 fmt + cargo deny。
4. **择机**：其余 Medium/Low 项随相关模块迭代处理。

---

## 10. 审查方法与轮次记录

| 轮次 | 专题 | 主要覆盖文件 |
|---|---|---|
| 1 | 架构 + 存储/模型 | `storage.rs` `config.rs` `model.rs` `app_state.rs` |
| 2 | 启动器/热键/托盘 FFI | `runner.rs` `hotkey.rs` `tray.rs` `tray_ui.rs` |
| 3 | UI 同步/图标缓存/事件处理 | `ui_sync.rs` `icon_cache.rs` `handlers/mod.rs` |
| 4 | 拖放/剪贴板/几何/数据管理 | `platform/dragdrop.rs` `platform/mod.rs` `drop_handler.rs` `clipboard.rs` `fileops.rs` `geometry.rs` `item_manager.rs` |
| 5 | UI 层 + 测试 + CI | `main_window.slint` `item_grid.slint` `sidebar.slint` `item_tile.slint` `toolbar.slint` `settings_dialog.slint` `edit_dialog.slint` `tests/*` `rust-build.yml` |
| 6 | 并发/panic/泄漏扫描 | 全库 grep（unwrap/expect/panic/thread::spawn/unsafe 分布） |
| 7 | 安全专项 | 注入/路径/权限/隐私逐项核对（`runner.rs` `icon_cache.rs` `tray.rs` `logger.rs`） |
| 8 | 性能专项 | 锁粒度/IO 放大/重复计算/缓存策略 |
| 9 | 功能完整性与文档一致性 | README/AGENTS 声明 vs 代码逐条抽查 |
| 10 | 交叉复检 + 汇总去重 | Esc/确认框/导入导出/编辑对话框收尾路径复检，本报告成文 |

**结论**：GoRun 是一个工程质量良好的项目，核心风险集中在图标提取线程模型与几处缓存/恢复策略；按上述路线图处理后，可达到适合日常使用的稳定度。
