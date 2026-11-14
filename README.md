# GoRun - 极速启动工具

**GoRun** 是一款专注于"快速启动"的纯粹工具。

## 特色亮点

**现代化架构** - 拥抱前沿技术栈，以超前架构定义性能，深度融合新系统特性。

**极致的快** - 100% 原生开发，毫秒级启动、即时响应。无 WebView，无 Electron，性能与体验全面领先。

**低能耗** - 深度优化底层架构，实现极低资源占用，长时间运行依然轻盈高效。

## 已实现功能

> 主线为 `rs/`（Rust + Slint）；`src/`（C++ + ImGui）冻结为维护模式。

### 核心功能
- 窗口框架（Slint 原生渲染，无 WebView/Electron）
- 拖放添加文件/快捷方式
- 分类管理（创建、编辑、删除；右键上移/下移排序）
- 项目管理（添加、编辑、删除；右键上移/下移排序、移动到分类）
- 双击运行 / 管理员运行
- 全局快捷键唤醒/隐藏主窗口（默认 Ctrl+Alt+M）
- 托盘图标（显示/隐藏、开机自启、跟随鼠标、语言切换）

### 数据管理
- JSON 格式存储配置和数据（便携优先，回退系统数据目录）
- 图标缓存系统（FNV-1a64 哈希命名，编辑自动失效）
- 配置持久化（窗口位置、大小、主题、语言、跟随鼠标等）
- 完整性校验（HMAC-SHA256，隔离外部/被篡改配置）

### UI 组件
- 分类侧边栏（含计数徽章、选中指示条）
- 项目网格/列表视图切换
- 工具行（搜索框 + 视图/主题/汉堡三图标按钮）
- 汉堡菜单（新建/删除/重命名/热键/视图/主题/语言/导入导出/关于/退出）
- Toast 提示（右下角，成功/错误，替代状态栏）
- 编辑对话框
- 主题系统（深色/浅色，设计令牌驱动）
- 搜索（子串 + 拼音首字母/全拼，按使用次数排序）
- 多语言（简体中文 / 繁体中文 / 英文）

## 技术栈

#### C++ 原版（`src/`，维护模式）

| 项目 | 选择 |
|-----|------|
| 语言 | C++17 |
| 构建 | CMake |
| 渲染 | DirectX 11 + ImGui (docking) |
| 存储 | JSON (nlohmann/json) |
| 依赖管理 | FetchContent |
| 平台 | Windows 10+ |

#### Rust 重写（`rs/`，主线）

| 项目 | 选择 |
|-----|------|
| 语言 | Rust 2021 |
| 构建 | Cargo |
| UI | Slint 1.7+ |
| 存储 | JSON (serde + serde_json) |
| 平台 | Windows 10+ |

## 构建

> 主线构建（`rs/`，Rust）。C++ 冻结版构建详见 `CMakeLists.txt` 与 `AGENTS.md`。

### 环境要求

- Rust 1.92+（MSRV 由依赖链决定：slint 1.17 及 backend 依赖要求 rustc 1.92）
- Cargo
- Windows SDK（for windows crate）

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
| slint | 1.7+ | UI 框架 |
| serde / serde_json | 1.x | JSON 序列化 |
| dirs | 5.x | 系统目录 |
| image | 0.25 | 图标处理 |
| pinyin | 0.10 | 中文拼音搜索 |
| windows | 0.57 | Windows API（仅 Windows 目标） |

## 项目结构

```
GoRun/
├── CMakeLists.txt              # CMake 主配置
├── AGENTS.md                   # AI 代理上下文
├── cmake/
│   └── FetchDependencies.cmake # 依赖获取脚本
├── rs/                         # Rust 重写版本（主线）
│   ├── Cargo.toml              # Cargo 配置
│   ├── src/                    # Rust 源码（config/storage/model/runner 等）
│   ├── ui/                     # Slint UI
│   └── tests/                  # 单元/GUI 集成测试
├── src/                        # C++ 原版代码（维护模式）
│   ├── main.cpp                # 程序入口
│   ├── app/                    # 应用主类
│   ├── ui/                     # 主窗口/主题/组件
│   ├── core/                   # 配置/存储/图标/启动器
│   ├── platform/               # Win32/DX11/托盘
│   └── utils/                  # 字符串/路径工具
├── resources/                  # 资源
│   ├── resource.rc             # Windows 资源
│   ├── icons/app.ico           # 应用图标
│   └── lang/                   # 多语言文件
├── tests/                      # C++ 单元测试
├── docs/                       # 文档
└── .github/
    └── workflows/              # CI/CD 构建配置
```

## 数据存储

> 主线（`rs/`，Rust）：**便携优先**——配置 `config.json` 与图标缓存 `icons/` 均位于可执行文件所在目录（`%EXEDIR%`）；仅当 exe 目录不可写时回退系统数据目录（Windows 为 `%LOCALAPPDATA%/GoRun`）。首次运行时自动创建目录和默认配置文件，并迁移旧版 `%APPDATA%/GoRun` 数据（只复制不删源）。

> C++ 冻结版（`src/`，维护模式）：配置路径为 `%APPDATA%/GoRun/config.json`，图标缓存为 `%APPDATA%/GoRun/icons/`。

## 参数变量

GoRun 支持 Windows 环境变量同时并内置参数变量：

| 变量名 | 说明 | 支持版本 |
|-------|------|---------|
| `%mp%` | 当前目录 | 1.0.0.1+ |
| `%mr%` | 当前盘符 | 1.0.0.1+ |
| `%so%` | 搜索参数 | 2.8.0.0+ |
| `%so-url%` | 搜索参数 URL 编码 | 2.8.0.0+ |

## 开发规划

### MVP 阶段（当前）
- [x] 窗口框架
- [x] 拖放添加
- [x] 分类管理
- [x] 项目运行
- [x] 全局快捷键
- [x] 托盘图标
- [x] 搜索功能（子串 + 拼音首字母/全拼，按使用次数排序）
- [x] 列表/平铺视图切换

### 后续扩展
- [ ] 主题编辑器
- [ ] 插件系统
- [ ] 任务计划
- [ ] 脚本支持
- [ ] 密码保护分类

## 截图

_（暂无截图，待补充）_


## 相关链接

- **GitHub**: https://github.com/AoThen/GoRun

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

本项目采用 MIT 许可证。


---

**备注**
- 名称：GoRun 快速启动工具
- 大小：约 3 MB+
- 语言：多语言
- 运行环境：Windows 10+
- 授权方式：免费软件
- 软件类别：桌面工具
- 编写语言：Rust / Slint（主线 `rs/`）；C++ / ImGui（冻结版 `src/`）