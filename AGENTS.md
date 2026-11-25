# GoRun - 项目上下文

## 项目概述

**GoRun** 是一款 Windows 平台的快速启动工具，专注于"快速启动"的纯粹体验。本项目使用 **Rust** 和 **Slint** 开发，采用原生架构，无 WebView/Electron 依赖，具有毫秒级启动和极低资源占用的特点。

> **项目状态：** MVP 阶段 - 核心功能已实现，高级功能规划中
>
> **主线：** `rs/`（Rust + Slint）。配置目录统一为便携（exe 同级），仅当 exe 目录不可写时回退系统数据目录。

### 核心特性

- **极速启动**：100% 原生开发，毫秒级响应
- **低能耗**：深度优化底层架构，极低资源占用
- **现代化架构**：拥抱前沿技术栈，深度融合新系统特性

### 技术栈

#### Rust 主线（`rs/`）

| 项目 | 选择 |
|-----|------|
| 语言 | Rust 2021 |
| 构建 | Cargo |
| UI | Slint 1.18 (software 渲染器) |
| 存储 | JSON (serde + serde_json) |
| 平台 | Windows 10+ |

---

## 当前功能

### 已实现（MVP，Rust 主线）

> **主线为 `rs/`（Rust + Slint）**。以下按 Rust 主线实际实现编写。

- **窗口框架**：Slint 原生渲染（无 WebView/Electron），最小 360×240，几何记忆 + 离屏检测 + DPI 换算
- **拖放添加**：OLE IDropTarget（`RegisterDragDrop`），支持文件/快捷方式/`.url` 链接
- **分类管理**：创建、重命名、删除（级联，至少保留一个）；右键上移/下移 + 拖拽排序（F2-1）
- **项目管理**：添加、编辑、删除；单击运行/管理员运行（v2 单击即启动，400ms 内同项重复点击防抖）；右键上移/下移 + 拖拽排序 + 移动到分类；运行全部（背景右键）需两道确认防误批量启动，空分类直接提示
- **一键清理无效条目**：汉堡菜单入口（danger 标红）；后台线程扫描全部分类中"运行必失败"条目（空目标 / 非 URL 目标不存在，与运行期 EmptyTarget/FileNotFound 判定同构；`%so%`/`%so-url%` 搜索模板视为有效），确认框列数量+名称预览后批量删除，同步清理图标缓存并重注册条目热键；扫描经 mpsc+轮询 Timer 回送，不阻塞 UI
- **全局快捷键**：默认 Ctrl+/ 唤醒/隐藏；条目级全局热键（F8/4.1，每条目可配）
- **托盘图标**：显示/隐藏、开机自启（HKCU Run）、跟随鼠标、低驻留模式
- **数据持久化**：JSON 便携优先（exe 同级，回退 `%APPDATA%/GoRun`）；原子写（tmp+fsync+rename）+ 单份 `.bak1` 备份（仅结构性变更轮转，使用计数等高频写不冲刷）；单机信任模型（无完整性签名，损坏文件经 `.corrupt` 隔离 + 备份恢复）；加载/导入后孤儿条目（指向不存在分类）自动归首分类
- **图标缓存系统**：FNV-1a64 哈希命名，异步提取 + 内存缓存，`cachedIcon` 落盘跨进程恢复（键校验）；exe/dll 自定义图标源经异步提取兜底，损坏缓存自动重提取
- **主题系统**：深/浅色设计令牌（theme.slint 全局令牌，`set-dark` 切换）
- **搜索功能**：子串 + 拼音首字母/全拼（P1-9 缓存），按使用次数排序，300ms 防抖
- **图标/列表视图切换**：按分类 `view_type` 显示网格或列表行；**默认列表模式**（`view_type` 默认 `List`，存量配置保持原视图不变）；列表模式固定列宽 160px 横向自适应多列（行高 34px），图标模式 96×88 响应式网格
- **设置对话框（V1）**：背景图 png/jpg（最长边 2560 封顶）+ 暗化遮罩开关/浓度 + 日志档位（Info 默认/warn/debug）+ 光标离开自动隐藏开关（F7，运行时即时生效）
- **多语言**：固定简体中文（v2 决策，不再支持切换）

### 规划中

- 可视化主题编辑器
- 插件系统
- 任务计划（定时执行、周期循环）
- 脚本支持
- 密码保护分类

---

## 目录结构

```
GoRun/
├── README.md                   # 项目主文档
├── AGENTS.md                   # AI 代理上下文文件（本文件）
├── rs/                         # Rust 主线（Rust + Slint）
│   ├── Cargo.toml              # Cargo 配置
│   ├── Cargo.lock              # 依赖锁定
│   ├── build.rs                # Slint 编译脚本
│   ├── src/
│   │   ├── main.rs             # 薄入口（main + 模块声明）
│   │   ├── lib.rs              # 库导出
│   │   ├── app_state.rs        # AppState 状态容器
│   │   ├── clipboard.rs        # 剪贴板
│   │   ├── config.rs           # 配置管理
│   │   ├── drop_handler.rs     # OLE 拖放入口
│   │   ├── fileops.rs          # 文件对话框
│   │   ├── geometry.rs         # 窗口几何/定位
│   │   ├── handlers/           # 事件处理器
│   │   │   ├── mod.rs          # wire_handler 回调注册
│   │   │   ├── dialogs.rs      # 编辑对话框辅助
│   │   │   └── edit_dialog.rs # 新建项目对话框
│   │   ├── hotkey.rs           # 快捷键管理
│   │   ├── icon_cache.rs       # 图标缓存
│   │   ├── item_manager.rs     # 项目管理器
│   │   ├── localization.rs     # 本地化（仅简体中文，v2 固定）
│   │   ├── logger.rs           # 日志
│   │   ├── model.rs            # 数据模型
│   │   ├── platform/           # 平台相关（Windows）
│   │   │   ├── mod.rs          # 平台模块入口
│   │   │   └── dragdrop.rs     # OLE 拖放实现
│   │   ├── runner.rs           # 程序启动器
│   │   ├── storage.rs          # JSON 存储
│   │   ├── tray.rs             # 托盘图标
│   │   ├── tray_ui.rs          # 托盘 UI 动作桥接
│   │   ├── ui_sync.rs          # UI 模型同步/刷新
│   │   └── util.rs             # 工具函数
│   ├── ui/
│   │   ├── main_window.slint   # 主窗口 UI
│   │   ├── edit_dialog.slint   # 编辑对话框 UI
│   │   ├── theme.slint         # 主题设计令牌（深/浅色板）
│   │   ├── components/         # 复用组件（toolbar/toast/sidebar/item_grid/item_tile/context_menu/menu_popup；menu_bar/status_bar/search_bar 已删）
│   │   └── dialogs/            # 对话框组件（confirm/input/about）
│   └── tests/
│       ├── gui_integration_test.rs  # GUI 集成测试
│       └── storage_test.rs     # 存储单元测试
├── resources/
│   ├── icons/
│   │   └── app.ico             # 应用图标（build.rs 嵌入 exe）
│   └── lang/
│       └── zh-CN.json          # 简体中文（编译期嵌入）
└── .github/
    └── workflows/
        └── rust-build.yml      # Rust CI/CD 构建配置
```

---

## 构建指南

### 环境要求

- Rust 1.92+ (MSRV 由依赖链决定：slint 1.18 及 backend 依赖要求 rustc 1.92)
- Cargo
- Windows SDK (for windows crate)

### 构建步骤

```bash
cd rs

# Debug 构建
cargo build

# Release 构建
cargo build --release

# 运行单元测试
cargo test --lib

# 运行 GUI 集成测试（需要 xvfb 或显示服务器）
cargo test
```

### 依赖项

| 依赖 | 版本 | 用途 |
|-----|------|------|
| slint | 1.18.1 | UI 框架（software 渲染器） |
| serde | 1.x | 序列化 |
| serde_json | 1.x | JSON 解析 |
| dirs | 7.x | 系统目录 |
| image | 0.25 | 图像处理 |
| log | 0.4 | 日志（运行时档位可调：Info 默认/Warn/Debug，设置项 `log_level`；默认 Info 收敛隐私——Debug 档记录搜索词/路径需显式开启） |
| chrono | 0.4 | 日期时间 |
| pinyin | 0.10 | 中文拼音转换（搜索） |
| raw-window-handle | 0.6 | 窗口句柄 |
| windows | 0.62 | Windows API 绑定（含托盘实现） |
| i-slint-backend-testing | 1.18.1 | GUI 集成测试后端（可选，backend-testing feature） |

---

## 数据存储

### 配置文件

**路径**：`%EXEDIR%/config.json`（可执行文件所在目录，便携优先）；exe 目录不可写时回退系统数据目录（Windows `%APPDATA%/GoRun`）。

首次运行时自动创建目录和默认配置文件，并迁移旧版 `%APPDATA%/GoRun` 数据（只复制不删源）。迁移时若旧配置为**空占位配置**（有条目但所有条目 target 均为空，如历史遗留的 "New Item" 占位），视为无有效数据，跳过迁移走正常首启；JSON 解析失败则保守照旧迁移。

**备份策略**：单份覆盖式备份 `config.json.bak1`，仅在**结构性变更**（增删改条目/分类、导入）保存时轮转；使用计数（runCount）与窗口几何属高频非关键写，走 `save`/`save_soft` 不轮转，避免"运行→退出"高频模式冲刷备份、保住误操作后的恢复窗口。主配置 JSON 损坏且无可用备份时隔离为 `.corrupt.<ts>` 留证并启用默认配置。

**孤儿条目归置**：加载/导入的配置若有条目指向不存在的分类（手改 JSON、外部文件），启动与导入时自动归入首个分类（分类为空则先补默认分类），保证条目在侧栏/分类视图可见可管理。

**单机信任模型**：不做配置完整性签名（曾有的 HMAC 机制已移除——密钥可逆向、且隔离后静默自动导入使防护收益趋近于零）。防损坏靠原子写（tmp+fsync+rename）+ 备份 + 损坏隔离。

### 图标缓存

**路径**：`%EXEDIR%/icons/`（便携优先，不可写回退数据目录）

**缓存策略**（Rust 主线）：
- 添加项目时自动提取图标并缓存
- 文件命名格式：FNV-1a64 哈希十六进制 `.png`（键 = item id + iconPath + iconIndex + target，编辑图标/目标自动失效）
- **缓存路径持久化**：提取成功后写入 `item.cachedIcon` 并落盘；启动时据此恢复图标（`item_keys` 仅为内存映射，进程重启即失效）
- **恢复键校验**：`cachedIcon` 必须与当前条目重算的缓存键路径一致才采纳（手改 target/iconPath 后残留旧键路径会被拒绝，走重新提取自愈）
- **损坏缓存自愈**：缓存文件解码失败时自动触发重新提取覆盖（不静默降级永久空白）
- **原子写**：提取出的 PNG 经唯一 tmp（`<hash>.tmp.<pid>.<seq>`，对齐配置原子写）+ rename 落盘，并发提取线程互不干扰
- **会话级提取防抖**：同一条目每次会话仅自动入队提取一次（防提取失败/无源时重复 spawn）；提取成功回写、编辑条目、手动刷新图标时清除标记
- 删除项目/删除分类时同步删除缓存图标

### JSON 数据结构

```json
{
    "version": "1.0.0",
    "categories": [
        {
            "id": "cat_<timestamp>_<seq>",
            "name": "分类名称",
            "sortOrder": 0,
            "viewType": 1,
            "iconSize": 48
        }
    ],
    "items": [
        {
            "id": "item_<timestamp>_<seq>",
            "name": "显示名称",
            "target": "目标路径",
            "arguments": "",
            "workingDir": "",
            "iconPath": "",
            "cachedIcon": "",
            "iconIndex": 0,
            "runAsAdmin": false,
            "runCount": 0,
            "keywords": "",
            "remark": "",
            "categoryId": "cat_<timestamp>_<seq>",
            "sortOrder": 0,
            "hotkey": ""
        }
    ],
    "config": {
        "globalHotkey": "Ctrl+/",
        "windowX": 100,
        "windowY": 100,
        "windowWidth": 800,
        "windowHeight": 600
    }
}
```

---

## 参数变量

GoRun 支持 Windows 环境变量以及以下内置参数变量：

| 变量名 | 说明 | 支持版本 |
|-------|------|---------|
| `%mp%` | 当前目录 | 1.0.0.1+ |
| `%mr%` | 当前盘符 | 1.0.0.1+ |
| `%so%` | 搜索参数 | 2.8.0.0+ |
| `%so-url%` | 搜索参数 URL 编码（form 风格：空格编为 `+`，适合 query 段；用于 URL 路径段时语义与标准 percent-encoding 略有差异） | 2.8.0.0+ |

> 快捷键说明：`+` 是快捷键串的分隔符，`Ctrl++` 等含 `+` 键位的组合无法表达（已知限制，`parse_hotkey_string` 返回失败）。

---

## 文案（仅简体中文）

v2 决策：仅保留简体中文，不再支持多语言切换。文案文件为 `resources/lang/zh-CN.json`（编译期嵌入，`Localization::tr()` 取键值，缺键回退键名）。

### 文案文件结构

```json
{
    "LANG_NAME": "语言名称",
    "Tip_*": "提示信息",
    "Title_*": "对话框标题",
    "Btn_*": "按钮文本",
    "Menu_*": "菜单项",
    "BI_*": "内置项目名称",
    "STR_*": "通用字符串",
    "TS_*": "主题字符串",
    "Set_Tab_*": "设置标签页",
    "SSTR_*": "设置字符串",
    "ST_*": "设置提示",
    "SI_*": "设置选项"
}
```

---

## 插件系统（规划中）

插件系统规划中，支持搜索插件扩展，SDK 将在主线（`rs/`）内自行实现。

### 插件类型

| 类型常量 | 值 | 说明 |
|---------|---|------|
| `_MN_PLUGIN_TYPE_SEARCH` | 100 | 搜索插件 |

### 核心事件

| 事件常量 | 值 | 说明 |
|---------|---|------|
| `_MN_PLUGIN_EVENT_LOAD` | 1000 | 插件被载入 |
| `_MN_PLUGIN_EVENT_DESTROY` | 9999 | 插件销毁 |
| `_MN_PLUGIN_EVENT_ENABLE` | 2010 | 插件被启用 |
| `_MN_PLUGIN_EVENT_DISABLE` | 2020 | 插件被禁用 |
| `_MN_PLUGIN_EVENT_CONFIG_CHANGE` | 3000 | 配置被更改 |
| `_MN_PLUGIN_EVENT_SEARCH_INIT` | 5000 | 搜索初始化 |

### API 函数

| 函数 | 说明 |
|------|------|
| `GetVersion()` | 获取 GoRun 版本号 |
| `GetSelfId()` | 获取插件自身 ID |
| `GetSelfDir()` | 获取插件目录路径 |
| `SetCfgItem()` | 写入配置项 |
| `GetCfgItem()` | 读取配置项 |

> **注意**：插件系统尚未实现，SDK 将在主线（`rs/`）内自行实现。

---

## CI/CD

### GitHub Actions

#### Rust 构建（`.github/workflows/rust-build.yml`）

| 项目 | 说明 |
|------|------|
| 触发路径 | `rs/**`、`resources/icons/**`、`resources/lang/**`、`.github/workflows/rust-build.yml` |
| Runner | `windows-latest` + `ubuntu-latest` |
| Windows 构建任务 | `rust-build-windows`：clippy + debug/release 构建 + 单测 + 产物上传 |
| Windows 测试任务 | `rust-tests-windows`：testing 后端构建 + Windows 原生全量测试 |
| Linux 任务 | `rust-gui-tests-linux`：xvfb + GUI 集成测试 (`gui_integration_test.rs`) + 全量测试 |
| 发布任务 | `release-latest`：下载产物、zip 打包、force-update latest 标签 |
| 产物 | `GoRun-rs-release` 7 天 |

---

## 项目链接

- **当前仓库**：https://github.com/AoThen/GoRun
- **官方网站**：https://GoRun.arae.cc/
- **更新日志**：https://soft.arae.cc/ChangeLog/GoRun.txt

---

## 注意事项

1. **Windows 平台专用**：本项目仅支持 Windows 10 及以上系统
2. **原生开发**：不使用 WebView 或 Electron，保持轻量
3. **绿色软件**：不产生系统垃圾，可便携运行
4. **数据安全**：每次操作后立即保存，确保数据不丢失
5. **MVP 阶段**：部分高级功能正在规划中，详见"规划中"章节
6. **拖拽排序手势**（rs 主线）：条目区用 `Flickable` 且 `mouse-drag-pan-enabled: false`（鼠标拖动直通 `DragArea` 排序，触摸拖动与滚轮仍滚动）；侧栏用 `ScrollView`（默认同样行为）。`DragArea.data` 绑定 pure 回调即时构造载荷（索引字符串），消除"down 时 Rust 回写属性"的时序竞态
7. **对话框/设置界面**（rs 主线）：覆盖层组件（Confirm/Input/About/Settings/Toast）在 main_window 实例须显式 `width/height: 100%`，否则按 Slint 规则落在 (0,0) 且尺寸为 preferred（遮罩不铺满、不居中）；遮罩已加 TouchArea 吞点击防穿透（不点击关闭，Esc/按钮关闭）
