# GoRun UI v2 重设计 —— 改动设计与实施计划

> 状态：已实施（提交 `9415cf0`，T1-T4 全部落地；静态审核通过，未本地编译，CI 复核） | 定稿依据：HTML 原型 `docs/prototype/gorun-ui-prototype.html`（已确认）
> 范围：主线 `rs/`（Rust + Slint）；`src/`（C++）冻结版不受影响。
> 原则：最小改动、不动数据 schema、复用现有回调分发；`geometry.rs` 仅同步右键菜单尺寸常量（见 §3）。

---

## 1. 背景与目标

现有 Slint UI（`rs/ui/`）存在四类问题：

1. **色系混乱**：紫蓝底 `#1a1a2e` + 蓝卡片 `#0f3460` + 中性灰菜单 + 玫红 accent `#e94560` 四种色系并存；红色（危险语义）被用作选中/hover/主按钮。
2. **图标底板突兀**：每个图标带深蓝方块底（`theme.slint` bg-card）。
3. **结构过重**：传统五组菜单栏（各 90px 宽）与"极速启动器"定位冲突；弹层定位硬编码 `10px + group * 94px`（`main_window.slint`，语言切换即错位）；StatusBar 价值低。
4. **可点击性差**：New Item / List / Dark / + Category 均为裸文本，无按钮形态。

目标：对齐已确认的原型——中性灰阶 + 蓝 accent、图标无底板、工具行（搜索 + 3 图标按钮）、汉堡菜单右锚定弹层、Toast 替代 StatusBar、空状态引导、右键菜单支持分隔线与危险项标红。

---

## 2. 设计规格 v1（定稿）

### 2.1 色彩令牌

| 令牌 | 深色 | 浅色 | 用途 |
|------|------|------|------|
| `bg-0` | `#202020` | `#f3f3f3` | 主背景 |
| `bg-1` | `#262626` | `#fbfbfb` | 侧边栏/标题栏 |
| `bg-2` | `#2c2c2c` | `#ffffff` | 弹层/菜单/对话框/Toast |
| `bg-3` | `#323232` | `#ffffff` | 输入框 |
| `stroke-1` | `#383838` | `#e0e0e0` | 分隔线 |
| `stroke-2` | `#454545` | `#d0d0d0` | 控件描边 |
| `text-1` | `#f5f5f5` | `#1a1a1a` | 主文本 |
| `text-2` | `#a8a8a8` | `#5c5c5c` | 次要文本/占位 |
| `hover` | `#2f2f2f` | `#ebebeb` | 悬浮底 |
| `accent` | `#4cc2ff` | `#005fb8` | 选中/主按钮/指示条 |
| `accent-subtle` | `rgba(76,194,255,.12)` | `rgba(0,95,184,.08)` | 选中底 |
| `accent-fg` | `#101010` | `#ffffff` | accent 按钮上文字 |
| `danger` | `#ff99a4` | `#c50f1f` | 危险项/错误 |
| `danger-subtle` | `rgba(255,153,164,.12)` | `rgba(197,15,31,.08)` | 危险项 hover 底 |
| `danger-fg` | `#2b0000` | `#ffffff` | danger 按钮上文字 |

**规则**：红仅表示危险；删除确认按钮 = danger 实底 + danger-fg 文字。

### 2.2 尺寸令牌

| 类别 | 值 |
|------|-----|
| 圆角 | `radius-xs` 4px（菜单项/输入框/按钮）· `radius-md` 6px（卡片/弹层/tile）· `radius-lg` 10px（对话框） |
| 间距 | 4 / 8 / 12 / 16（theme 现有 3 档：`spacing-sm` 4 / `spacing-md` 8 / `spacing-lg` 16；新增 `spacing-panel` 12px，用于面板/内容区内边距） |
| 字号 | `xs` 11px（徽章）· `sm` 12px（网格项目名）· `md` 13px（菜单/正文/侧栏标题/列表行名）· `lg` 16px（对话框标题） |
| 图标 | 网格 48px · 列表 22px · 搜索 14px |
| 网格单元 | 96×88（原 95×95） |
| 列表行 | 34px（原 36px） |
| 侧边栏 | 宽 180px（原 160px），分类项高 32px |
| 工具行 | 高 44px，搜索框高 30px，图标按钮 32×32 |
| 菜单项 | 高 28px（原：ContextMenu 22px、MenuPopup 24px），弹层宽 200px（汉堡）/ 160px（右键） |
| 对话框 | 宽 340px，圆角 10px，遮罩 `rgba(0,0,0,.5)` 深色 / `.4` 浅色 |

### 2.3 组件规格

- **工具行**：搜索框（左，弹性）+ 视图切换 / 主题切换 / 汉堡 三个图标按钮（右）。图标用 Slint 内置字符或自绘 Rectangle（汉堡 = 3 横线、视图 = 2×2 方格/3 横线切换、主题 = 双色半圆），不引入外部图片资源。
- **侧边栏**：分类项 32px；选中 = `accent-subtle` 底 + 左缘 3px accent 指示条；hover = `hover` 底；行尾右对齐 11px 计数徽章；底部「＋ 新建分类」行 hover 显底。
- **网格 tile**：图标 48px 无底板直接渲染；hover/选中 = radius 6 圆角底；选中另加 1px accent 描边（outline 语义，Slint 用 border Rectangle 实现）。
- **列表行**：34px，图标 22px。
- **汉堡弹层（MenuPopup 复用）**：右锚定（`x = 窗口宽 - 弹层宽 - 8px`，`y = 80px`），彻底消除硬编码组定位；`bg-2` + `stroke-1` + radius 6 + drop-shadow。
- **右键菜单（ContextMenu 复用）**：同弹层视觉；`items` 中值为 `"-"` 渲染 1px 分隔线；新增平行 bool 数组标记危险项，渲染为 `danger` 色文字。
- **对话框**：Confirm/Input/About 统一模板——遮罩 + radius 10 + 标题 16px/600 + 取消（stroke-2 描边按钮）+ 确认（accent 实底；删除场景 danger 实底）。
- **Toast**：右下角浮出，`bg-2` + stroke-1 + 阴影，radius 6，字号 12px；成功 2s、错误 4s（文字 danger 色）；承接全部 `set_status_text` 语义。
- **空状态**：48px 虚线圆角框 + 主文案（Tip_NoItems）+ 副文案「拖入文件或快捷方式即可添加」（新键 `Tip_DropToAdd`）。

---

## 3. 改动总览（文件级）

| 文件 | 操作 | 内容 |
|------|------|------|
| `rs/ui/theme.slint` | **重写** | 全量新令牌（上表），`set-dark(is-dark)` 按新色板赋值 |
| `rs/ui/main_window.slint` | **重构** | 删 MenuBar/StatusBar；新增 Toolbar/Toast；MenuPopup 右锚定；键盘增强 |
| `rs/ui/components/search_bar.slint` | **重写** | 更名 `toolbar.slint`（组件 `Toolbar`）：搜索框 + 3 图标按钮 |
| `rs/ui/components/status_bar.slint` | **删除** | 由 `toast.slint` 取代 |
| `rs/ui/components/toast.slint` | **新增** | Toast 组件 |
| `rs/ui/components/menu_bar.slint` | **删除** | 汉堡按钮并入 Toolbar |
| `rs/ui/components/menu_popup.slint` | 修改 | 新令牌 + 分隔线 + 右锚定定位 |
| `rs/ui/components/context_menu.slint` | 修改 | 新令牌 + 分隔线 + 危险项 |
| `rs/ui/components/sidebar.slint` | 修改 | 指示条/计数徽章/新令牌/宽度 180px |
| `rs/ui/components/item_grid.slint` | 修改 | 单元 96×88、新令牌 |
| `rs/ui/components/item_tile.slint` | 修改 | 图标 48 无底板、列表行 34px、选中描边 1px |
| `rs/ui/dialogs/confirm_dialog.slint` | 修改 | 令牌名 + radius 10 + danger 按钮配色 |
| `rs/ui/dialogs/input_dialog.slint` | 修改 | 令牌名 + 统一模板 |
| `rs/ui/dialogs/about_dialog.slint` | 修改 | 令牌名 + 统一模板 |
| `rs/ui/edit_dialog.slint` | 修改 | 接入 theme 令牌（原为无主题裸默认样式） |
| `rs/ui/main_window.slint` + `rs/ui/components/sidebar.slint`（CategoryModel struct 定义两处） | 修改 | CategoryModel 增 `count: int` |
| `rs/src/geometry.rs` | 修改 | 右键菜单尺寸常量同步：`MENU_W` 160 不变、项高系数 `22.0`→`28.0`（L99-100 clamp 计算） |
| `rs/src/ui_sync.rs` | 修改 | CategoryModel 构造填充 count；新增 `show_toast()` |
| `rs/src/tray_ui.rs` | 修改 | `apply_localized_texts` 删 menu-title 接线、补副文案 |
| `rs/src/handlers/mod.rs` | **重构** | 汉堡菜单组装/分发；右键菜单 danger 数组；`set_status_text`→`show_toast` |
| `rs/src/handlers/config_io.rs` | 修改 | `set_status_text`→`show_toast`（5 处） |
| `rs/src/handlers/dialogs.rs` | 修改 | `set_status_text`→`show_toast`（1 处） |
| `rs/src/main.rs` | 修改 | `set_status_text`→`show_toast`（3 处）；CategoryModel 构造补 count |
| `rs/tests/gui_integration_test.rs` | 修改 | CategoryModel 构造补 count |
| `resources/lang/*.json`（3 份） | 修改 | 新增 `Tip_DropToAdd`；弃用键保留不接线 |
| `docs/manual-test-checklist.md` | 修改 | 新增 UI v2 手测项 |
| `README.md` / `AGENTS.md` | 修改 | UI 组件描述同步 |

**不改动**：数据 schema、config 键（`theme`、`viewType`、窗口几何）、拖拽/搜索/运行逻辑、窗口尺寸 800×600、托盘。

---

## 4. 分阶段实施

> 每阶段独立可验收，静态审查通过后进入下一阶段。

### T1 设计令牌重写（纯视觉，行为零变化）

0. **渲染器现状确认（2026-11 已核实，转为长期守护项）**：`rs/Cargo.toml` slint 为 `default-features = false, features = ["renderer-software", ...]`，`Cargo.lock` 无任何 skia 依赖——项目已运行在软件渲染器上，无需切换。后续仅需守住两点：
   - 禁止引入 skia renderer feature（`rg 'renderer-skia|skia' rs/Cargo.toml` 期望 0 命中，否则二进制暴涨 20MB+）；
   - software renderer 的 drop-shadow 为 CPU 近似模糊，阴影边缘比 GPU 略"硬"（§6 降级方案兜底）；真实观感按 §7.4 截图比对确认。
   - 另：AGENTS.md 技术栈表写"Slint 1.7+ (Skia 渲染)"与事实不符，随 §8 文档同步修正为"software 渲染器"。
1. **`theme.slint` 全量重写**：令牌按 §2.1/§2.2 命名；`set-dark(is-dark)` 回调保留原机制。
2. **全组件引用改名**（grep `Theme.` 定位所有引用点逐一替换）：
   - `bg-primary`→`bg-0`，`bg-secondary`→`bg-1`，`bg-card`→`bg-2`（注意：tile 中图标底板引用在 T2 删除）
   - `text-primary`→`text-1`，`text-secondary`→`text-2`
   - `menu-bg`→`bg-2`，`menu-hover`→`hover`，`menu-border`→`stroke-1`
   - `error`→`danger`；新增 `accent-subtle/accent-fg/danger-subtle/danger-fg/stroke-2`
   - `radius-sm` 0px→4px；`font-size-sm` 11px→12px；`font-size-md` 14px→13px；新增 `font-size-xs`(11px)、`font-size-lg`(16px)
   - `icon-size` 64px→48px
3. **验收**：grep 确认无旧令牌残留（`bg-primary|bg-secondary|bg-card|text-primary|text-secondary|menu-bg|menu-hover|menu-border|Theme.error`）；仅改色/改名不改布局，行为与现状一致。

### T2 组件规格落地（视觉形态对齐原型）

1. **`item_tile.slint`**：删除图标 `Rectangle` 底板（bg-2 色）——图标 `Image` 直接置于容器；网格单元内容对齐 96×88；列表行 34px/图标 22px；选中态由 2px accent 边框改为 1px（覆盖层 Rectangle border-width）。
2. **`item_grid.slint`**：`cell-width` 95→96、`cell-height` 95→88（`column-count` 公式已引用 `cell-width`/`cell-height`，改值即自动生效）；空状态接入副文案。
3. **`sidebar.slint`**：宽 160→180px；分类项加左缘 3px 指示条（Rectangle）；行尾计数徽章（新属性 `count`）；「＋ 新建分类」行加 hover 底。
4. **`menu_popup.slint` / `context_menu.slint`**：项高 28px；radius 6；`drop-shadow-blur/shadow-color` 阴影；分隔线渲染（`item == "-"` 时渲染 1px `stroke-1` 矩形）；危险项文字 `danger` 色。
5. **对话框 ×3 + `edit_dialog.slint`**：radius 10、标题 `font-size-lg`、取消=描边按钮、确认=accent 实底（ConfirmDialog 删除场景 danger 实底）；`edit_dialog.slint` 接入 Theme（背景/文本/输入框令牌），消除深色主题下白色对话框。
6. **验收**：与原型逐组件比对（截图对照 `docs/prototype/gorun-ui-prototype.html` 各区块）；grep 确认 `icon-size` 引用处尺寸一致。

### T3 结构重构（工具行 + 汉堡 + Toast）

1. **新增 `toolbar.slint`**（替代 search_bar.slint，组件名 `Toolbar`）：
   - 布局：搜索框（高 30px，`bg-3`+`stroke-2` 描边，focus 时 accent 描边）+ spacer + 三个 32×32 图标按钮（视图切换/主题切换/汉堡）。
   - 回调：`search-changed`、`toggle-view`、`toggle-theme`、`new-item-clicked`（保留既有）+ 新增 `hamburger-clicked`。
   - **必须保留** `field-focused`（in-out）与内部 `changed has-focus` 回写链路（`main_window` 回车守卫 `main_window.slint:195-205` 依赖它区分搜索聚焦态；T4 Ctrl+F 亦依赖）。
   - 文案属性删减：`switch-list/icon/light/dark-text` 删除（图标无文字）；保留 `search-hint`、`new-item-text`。
2. **`main_window.slint` 重构**：
   - 删除：MenuBar import 与实例、`menu-title-*` 5 属性、`open-menu-group`、`menu-bar-group-clicked`、StatusBar、`menu-popup-x` 硬编码算式。
   - 新增：`Toolbar` 实例；`MenuPopup` 定位改 `x: root.width - self.width - 8px; y: 80px`（Slint 声明式右锚定）；Toast 实例（`toast-message: string`、`toast-error: bool`、`toast-visible: bool` 属性）。
   - `menu-bar-item-selected` 重命名为 `menu-popup-selected(index)`（保持单一入口语义）。
3. **`handlers/mod.rs` 菜单重构**：
   - `on_menu_bar_group_clicked` → `on_hamburger_clicked`：组装**单一列表**（顺序：新建项目/新建分类/─/删除当前分类*/─/重命名当前分类/项目快捷键/─/切换视图/切换主题/─/跟随鼠标/开机自启/─/简体中文/繁體中文/English/─/导出配置/导入配置/─/关于/退出*；`*` 为 danger 项；分隔线以 `"-"` 占位；「切换主题」文案按当前主题取 `Menu_SwitchLight/Dark`）。原 group Cell 与 `menu-popup-visible` 联动逻辑删除。
   - `on_menu_bar_item_selected` → `on_menu_popup_selected(index)`：原 5 组 match 中的全部动作（新建/删分类确认流/重命名 InputDialog 流/hotkey/视图切换含条件判断/主题切换/跟随鼠标/自启/三语言/导入导出/关于/隐藏退出）平移到单一 index match，**动作实现零改动**。
4. **Toast 接线**：
   - `ui_sync.rs` 新增 `show_toast(ui: &MainWindow, msg: impl Into<SharedString>, error: bool)`：set 三属性 + `slint::Timer` 单例重启（成功 2s / 错误 4s 置 `toast-visible=false`）。
   - 替换全部 `ui.set_status_text(x)`（共 18 处：`main.rs` 3、`handlers/mod.rs` 7、`config_io.rs` 5、`dialogs.rs` 1、`ui_sync.rs` 2）→ `show_toast(&ui, x, false)`，错误语义处（启动失败/导入失败/校验失败等）传 `true`；`status-text` 属性与 `status_bar.slint` 删除。
5. **`tray_ui.rs::apply_localized_texts`**：删 5 行 `set_menu_title_*`；删 `sb_switch_*` 接线；其余（search hint/new item/empty/sidebar 文案）保留。
6. **验收**：grep 确认 `status-text|menu_bar_group|menu-title|status_bar` 无残留；语言切换后汉堡列表文案正确；Toast 计时与 error 配色正确；Esc 分层（对话框→弹层→清搜索→隐藏）回归。

### T4 交互补齐

1. **键盘**：`main_window.slint` FocusScope 增加 `Home/End`（首尾导航）；`Ctrl+F` 调用 Toolbar 新增 `focus-search()` 回调（内部 LineEdit `.focus()`）。
2. **右键菜单 danger/分隔线**：
   - `main_window.slint` 新增平行属性 `context-menu-danger: [bool]`；ContextMenu 按索引渲染危险色。
   - `handlers/mod.rs` 3 处 `set_context_menu_items`（分类/项目/空白）同步组装 danger 数组；项目菜单分隔线：运行/管理员运行 ─ 编辑 ─ 删除*。
3. **计数徽章数据**：CategoryModel 为 Slint struct，定义于 `main_window.slint` 与 `sidebar.slint` **两处**，均加 `count: int`；三处构造点补值（`main.rs:374-385`、`ui_sync.rs:286-295`、`gui_integration_test.rs:35-43`，`count = items.filter(cat==).len()`）。
4. **空状态副文案**：`empty-text` 主文案 + 新 `empty-hint-text` 属性；`tray_ui.rs` 接线 `Tip_DropToAdd`；三语言文件新增该键（简：「拖入文件或快捷方式即可添加」/ 繁 / 英 "Drop files or shortcuts here to add"）。
5. **验收**：Home/End/Ctrl+F 生效；删除项红色、分隔线渲染；徽章随增删实时刷新；三语言切换无缺键（缺失时 tr 回退键名可肉眼发现）。

---

## 5. 数据与兼容性

- **数据 schema**：零变更。`count` 仅存在于 UI 模型（内存态），不落盘。
- **配置兼容**：`theme`（"0"/"1"）、`viewType`、窗口几何键全部沿用。
- **语言文件**：仅新增 `Tip_DropToAdd`；弃用键（`Menu_File/Menu_Edit/Menu_View/Menu_Set/Menu_Help`、`SB_Switch*`）保留不删，避免三文件 diff 膨胀与回滚成本。
- **GUI 集成测试**：`gui_integration_test.rs` 仅构造模型，未引用 menu_bar/status_text（已核实），仅需补 CategoryModel `count` 字段。

## 6. 风险与回滚

| 风险 | 缓解 |
|------|------|
| MenuPopup 右锚定在极小窗口下越界 | `x` 表达式加 `max(8px, ...)` 下限；窗口 min-width 800px 下不会触发 |
| 菜单项高 22→28px 后弹层高度计算失配 | 同步 `geometry.rs:99-100` 项高系数 22.0→28.0（`MENU_W` 160 不变），否则底缘 clamp 失效弹层被裁切 |
| Slint `drop-shadow-*` 属性版本能力差异 | 当前 `renderer-software` 下阴影为 CPU 近似模糊（边缘略硬，可接受）；若版本能力不支持则回退 `stroke-1` 描边无阴影 |
| LineEdit `focus()` API 行为差异（Ctrl+F） | 回调式实现隔离在 Toolbar 内部；失败仅该快捷键无效，不影响主线 |
| Toast Timer 单例线程归属 | Timer 在 UI 线程创建/重启（与现有 slint::Timer 使用模式一致） |
| 回滚 | 四阶段独立提交，任一阶段 revert 不影响数据；语言文件只增不改 |

## 7. 验证计划

按仓库约定（AGENTS.md）：**不做本地编译/运行验证，静态审查为主，CI 复核**。

1. **静态审查**：每阶段结束后人工逐文件核对 §4 验收项。
2. **grep 核对**（每阶段必跑）：
   - 旧令牌残留：`rg 'bg-primary|bg-card|menu-hover|text-primary' rs/ui/` → 期望 0 命中
   - 旧结构残留：`rg 'status-text|menu-title|menu_bar_group|menu-bar-group|menu-bar-item|status_bar|search_bar\.slint' rs/` → 期望 0 命中
   - 令牌引用完整性：`rg 'Theme\.' rs/ui/` 全部命中存在于 theme.slint 定义
3. **CI**：push 后检查 `rust-build.yml`（Windows：cargo build + cargo test --lib；Linux：xvfb GUI 集成测试）；失败按日志修复。
4. **手测**：`docs/manual-test-checklist.md` 新增 v2 清单（深浅主题全组件走查 / 汉堡全菜单动作 / 语言三切 / Toast 成功与错误 / Home/End/Ctrl+F / 右键菜单分隔线与删除红字 / 计数徽章增删刷新 / 空状态副文案 / 搜索空态）。
5. **渲染器观感比对（一次性，Windows 本机）**：以 `docs/prototype/gorun-ui-prototype.html` 为基准，对实际构建产物截图核对三处 software renderer 敏感点——① 弹层 drop-shadow 阴影边缘是否可接受（否则按 §6 降级为描边）；② 48px 图标 1:1 显示无缩放发虚；③ 窗口拖动/缩放流畅度。本环境不编译（仓库约定），比对由维护者在 Windows 本机或 CI 产物（`rust-build.yml` artifact）完成。

## 8. 文档同步

- `README.md`：UI 组件清单（分类侧边栏/项目网格/搜索/主题）补「工具行、汉堡菜单、Toast」描述。
- `AGENTS.md`：目录结构树同步（`toolbar.slint`、`toast.slint`，删 `menu_bar.slint`、`status_bar.slint`）；技术栈表"Slint 1.7+ (Skia 渲染)"修正为"software 渲染器"（与 `Cargo.toml` feature 一致，见 §4 T1.0）。
- `docs/ui-redesign-plan.md`：本文件；实施完成后追加「已实施」状态与各阶段实际 diff 摘要。

---

## 9. 实施记录（提交 `9415cf0`）

> 状态：**已实施**（2026-11-13）。静态审查（explore 子 agent）0 阻断；未在本环境编译（按仓库约定，CI 复核）。

| 阶段 | 落地摘要 |
|------|---------|
| T1 令牌 | `theme.slint` 全量新令牌（bg-0..3 / stroke-1,2 / text-1,2 / hover / accent* / danger* / spacing-panel / radius-xs/md/lg / font-size-xs/sm/md/lg / icon-size 48px）；全组件 `Theme.*` 引用改名，旧令牌 grep 0 残留 |
| T2 组件 | 图标无底板（Image 直接渲染）；网格 96×88；列表行 34px/图标 22px；选中 1px 描边；侧栏 180px + 左缘 3px 指示条 + 行尾计数徽章 + 新建分类 hover；菜单项 28px + `"-"` 分隔线；对话框 bg-2/radius-lg/标题 font-size-lg/取消 stroke-2 描边/确认 accent-fg（删除场景 danger 实底）；edit_dialog 接 Theme；geometry 菜单项高系数 22→28 |
| T3 结构 | `toolbar.slint`（搜索 30px + 视图/主题/汉堡 32×32 图标）替代 search_bar/menu_bar；`toast.slint` 替代 status_bar（右下角，2s/4s）；main_window 删 MenuBar/StatusBar，MenuPopup 右锚定 `max(8px, w-w-8px)`；handlers `on_hamburger_clicked`/`on_menu_popup_selected`（单一 23 项列表 + danger）；18 处 `set_status_text`→`show_toast`（ui_sync 单例 Timer）；tray_ui 删 menu-title/sb-switch 接线 |
| T4 交互 | Home/End（首尾导航）+ Ctrl+F（toolbar.focus-search）；右键菜单 danger 数组（项目删除/分类删除标红）；CategoryModel.count 填充（main/ui_sync/test 三处）；空状态副文案 `Tip_DropToAdd`（三语言） |

**验证**：旧令牌/旧结构 grep 0 残留；`Theme.*` 引用完整性 100%（约 100 处均在 theme.slint 定义）；菜单列表 23 项与 match 16 分支一一对应；danger 数组长度与 items 一致、无越界。

**说明**：① 项目/分类右键菜单的「分隔线」以「删除项标红」体现（保留原索引顺序，避免重排 on_menu_item_selected 分发，符合最小改动原则）；② `docs/prototype/`（HTML 原型）与本文档为参考文件，未纳入代码提交。
