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
- **Rust 版对照**：`icon_cache` 按需加载并管理生命周期，配合资源上限 / 失效清理。

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

---

## 备注

- 本文档为**事实记录**，非待办清单；`rs/` 侧对应实现以 `rs/` 源码为准。
- 若后续解除 `src/` 冻结，需先评估上述缺陷的修复成本与回归风险。
