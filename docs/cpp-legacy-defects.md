# C++ 原版遗留缺陷清单

> 状态：**已知不修复（仅文档记录）**
> 依据：AGENTS.md「主线决策（2026-11）」——`src/`（C++）冻结为维护模式，不再新增功能；下述缺陷不在 C++ 侧修复，仅在本文档记录。所有新功能与正确实现一律落到 `rs/`（Rust 重写，主线）。

下列缺陷按严重度排序。C++ 版保持现状以控制维护成本与回归风险；Rust 版已按对应设计规避或实现。

## 1. 双数据源（内存态 / 磁盘态可失同步）

- **位置**：`src/app/App.cpp:105-116`（Config 与 ItemManager 分别从 Storage 初始化）；`src/core/ItemManager.h:52-56`（`m_categories` / `m_itemsByCategory` 独立缓存）
- **缺陷**：`Config` 与 `ItemManager` 各自持有内存态（`m_config` 读 `m_storage`；`m_categories` / `m_itemsByCategory` 为独立副本），均从同一 `config.json` 读写。二者无统一写入路径，任一侧修改而未即时刷新另一侧时，内存态与磁盘态可能失同步。历史版本还曾同时向 `%APPDATA%\GoRun` 与 exe 目录落盘，可能遗留双份数据。
- **为何不修**：统一数据流需要重构 Config/ItemManager/Storage 依赖关系，属新架构范畴，不冻结。
- **Rust 版对照**：单一 `AppState` 持有 `manager` + `config`，所有变更写穿并即时 `save_if_modified()`；基准目录由 `config::base_dir()` 单例解析（便携优先，回退 AppData），杜绝双路径。

## 2. 图标纹理无界缓存

- **位置**：`src/core/IconTextureManager.cpp:24`（`m_textureCache[item.id] = texture;`）
- **缺陷**：按 `item.id` 累积纹理缓存，无上限；`RefreshIcon` 仅清单个 id 并调用 `UnloadTextureByPath`。普通删除项目路径未释放对应 D3D11 纹理，长期增删后显存残留不释放。
- **为何不修**：需引入缓存容量 / 引用计数策略，属功能增强。
- **Rust 版对照**：`icon_cache` 按需加载并管理生命周期；缓存为**确定性哈希命名**（FNV-1a64，键含 target/icon_path/index），编辑图标/目标自动失效，删项/删分类时定点清理。无目录级孤儿清理（重置配置/版本升级后可能残留旧图标文件）。

## 3. 删除分类无确认

- **位置**：`src/core/ItemManager.cpp:70`（`DeleteCategory` 直接级联删除）；`src/ui/MainWindow.cpp` 菜单直接调用
- **缺陷**：删除分类立即级联删除其下所有项目与图标缓存，无任何确认对话框，误删不可恢复。
- **为何不修**：需要 UI 确认流程，属于交互设计增强。
- **Rust 版对照**：分类删除经 `ConfirmDialog` 二次确认（`handlers/mod.rs` 分类菜单删除分支 + 确认弹窗回调），级联删除后立即落盘。

## 4. 图标路径未落盘

- **位置**：`src/core/ItemManager.cpp:224-225`（提取快捷方式图标后赋 `item.iconPath`，但序列化未保证持久化该字段）
- **缺陷**：从 `.lnk` / 可执行文件提取的图标路径写入内存 `Item.iconPath`，但 `Storage::Save` 序列化未可靠落盘该字段（或落盘后未触发刷新），重启后图标路径丢失，退化回默认图标。
- **为何不修**：需修正序列化映射，属数据层修复，按冻结原则不修。
- **Rust 版对照**：`storage` 全字段序列化（`icon_path` 含在 Item 结构并落盘）。

## 5. `%so-url%` 按 UTF-16 编码（非 UTF-8）

- **位置**：`src/core/Runner.cpp:84-95`
- **缺陷**：`%so-url%` 逐 `wchar_t` 编码，非 ASCII 字符直接 `_snwprintf(L"%%%02X", (unsigned)ch)` 按 UTF-16 code unit 输出，未做 UTF-8 字节编码。中文 / 表情字符搜索结果产生错误 URL 编码（例："中" 应为 `%E4%B8%AD`，实际输出 `%4E2D`）。
- **为何不修**：涉及编码策略变更，可能改变既有链接行为。
- **Rust 版对照**：按 UTF-8 字节做百分号编码，`%so%` 与 `%so-url%` 语义正确。

## 6. 剪贴板句柄泄漏

- **位置**：`src/ui/MainWindow.cpp:151-160`
- **缺陷**：`OpenClipboard` → `GlobalAlloc` / `GlobalLock` 后，异常分支未保证 `GlobalUnlock` / `GlobalFree` / `CloseClipboard` 全部调用，可能泄漏系统句柄与内存。
- **为何不修**：需 RAII / 作用域守卫重构，属健壮性增强。
- **Rust 版对照**：剪贴板访问使用作用域 / RAII 自动释放。

## 7. DPI 增量漂移

- **位置**：`src/platform/Window.cpp:152-156`（`WM_DPICHANGED` 回调，`LOWORD(wParam)` 传入新 DPI）
- **缺陷**：窗口 / ImGui 缩放采用**增量累加**（每次 `+=` 比例差），而非基于新 DPI **绝对重算**；多次跨 DPI 屏幕拖动后缩放比例逐步累积漂移。
- **为何不修**：需重写缩放基准逻辑，属架构级修改。
- **Rust 版对照**：基于当前 `scale factor` 绝对计算布局与窗口尺寸。

## 8. `%so%` 搜索参数无注入防护（安全类缺陷）

> 标注：本条为**安全类**缺陷，与其余 7 项功能/健壮性缺陷区隔。

- **位置**：`src/core/Runner.cpp:63-87`（`ExpandVariables` 将 `m_searchQuery` 原样替换进 `%so%`）；`src/core/Runner.cpp:100-102` / `:152-154`（target/arguments/workingDir 全部先展开后 `ShellExecuteExW`）；`src/core/Runner.h:16`（`SetSearchQuery` 裸 setter）
- **缺陷**：搜索词不做任何字符校验即替换进 target / arguments / workingDir 后执行。Rust 主线已对 shell 型目标（cmd/powershell/mshta/wscript/cscript + 脚本扩展名）做 `%so%` 注入防护（`runner.rs` `shell_kind` + `is_search_param_safe`），C++ 版无等价物，脚本型项目（含 `.vbs/.js/.hta/.wsf` 及指向解释器的 `.lnk`）可被搜索词注入。
- **附带缺陷 8a（陈旧搜索词残留）**：`src/ui/MainWindow.cpp:438-441` 清空搜索分支仅调 `m_itemGrid.SetSearchQuery(L"")`，未调 `m_runner->SetSearchQuery(L"")`（对照 `:499-500` 搜索态成对设置）。用户搜索过一次后退出搜索态，`m_searchQuery` 永久残留，此后**非搜索态**双击任何含 `%so%` 的项目都会注入上次的搜索词，触发面更大。
- **附带缺陷 8b（`%so%` 字面量死循环 DoS）**：`src/core/Runner.cpp:65-68` 的 `while(find(L"%so%"))` 每次从串首重新查找，若搜索词本身含字面量 `%so%`，替换后新文本仍含 `%so%` → 无限循环，界面冻结。
- **为何不修**：安全修复虽不受功能冻结限制，但 C++ 版已进入维护模式且 Rust 主线已完整实现防护；在冻结分支上修复会引入回归风险，收益低于迁移成本。建议发布说明/更新日志告知用户"C++ 构建不含 `%so%` 注入防护，请迁移至 `rs/` 构建"。
- **Rust 版对照**：`runner.rs` 的 `shell_kind`（含 `ScriptHost` 族）`+ is_search_param_safe` 对含 `%so%` 的 target/arguments/working_dir 统一拦截不安全搜索词。

---

## 备注

- 本文档为**事实记录**，非待办清单；`rs/` 侧对应实现以 `rs/` 源码为准。
- 若后续解除 `src/` 冻结，需先评估上述缺陷的修复成本与回归风险。
