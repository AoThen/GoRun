# GoRun - 项目上下文

## 项目概述

**GoRun** 是一款 Windows 平台的快速启动工具，专注于"快速启动"的纯粹体验。本项目为原版 GoRun 的开源重写版本，使用 **C++** 和 **ImGui** 开发，采用原生架构，无 WebView/Electron 依赖，具有毫秒级启动和极低资源占用的特点。

> **项目状态：** MVP 阶段 - 核心功能已实现，高级功能规划中
>
> **Rust 重写：** 正在进行中，位于 `rs/` 目录，与 C++ 代码并行存在

### 核心特性

- **极速启动**：100% 原生开发，毫秒级响应
- **低能耗**：深度优化底层架构，极低资源占用
- **现代化架构**：拥抱前沿技术栈，深度融合新系统特性

### 技术栈

#### C++ 原版（`src/`）

| 项目 | 选择 |
|-----|------|
| 语言 | C++17 |
| 构建 | CMake 3.16+ |
| 渲染 | DirectX 11 + ImGui (docking 分支) |
| 存储 | JSON (nlohmann/json v3.11.2) |
| 依赖管理 | CMake FetchContent |
| 平台 | Windows 10+ |

#### Rust 重写（`rs/`）

| 项目 | 选择 |
|-----|------|
| 语言 | Rust 2021 |
| 构建 | Cargo |
| UI | Slint 1.7+ (Skia 渲染) |
| 存储 | JSON (serde + serde_json) |
| 平台 | Windows 10+ |

---

## 当前功能

### 已实现（MVP）

- **窗口框架**：基于 DirectX 11 + ImGui 的主窗口
- **拖放添加**：支持拖放文件/快捷方式添加项目
- **分类管理**：创建、编辑、删除分类
- **项目管理**：添加、编辑、删除、拖拽排序
- **项目运行**：双击运行 / 管理员运行
- **全局快捷键**：通过快捷键唤醒/隐藏主窗口（默认 Ctrl+Alt+M）
- **托盘图标**：系统托盘图标，支持快捷操作
- **数据持久化**：JSON 格式存储配置和数据
- **图标缓存系统**：自动提取并缓存应用图标
- **主题系统**：基础主题框架

### 规划中

- 搜索功能
- 可视化主题编辑器
- 插件系统
- 任务计划（定时执行、开机自启、周期循环）
- 脚本支持
- 密码保护分类
- 列表/平铺视图切换

---

## 目录结构

```
GoRun/
├── CMakeLists.txt              # CMake 主配置
├── README.md                   # 项目主文档
├── AGENTS.md                   # AI 代理上下文文件（本文件）
├── cmake/
│   └── FetchDependencies.cmake # 依赖获取脚本
├── src/                        # C++ 原版代码
│   ├── main.cpp                # 程序入口
│   ├── app/
│   │   ├── App.h/cpp           # 应用主类
│   │   └── Resource.h          # 资源定义
│   ├── ui/
│   │   ├── MainWindow.h/cpp    # 主窗口
│   │   ├── Theme.h/cpp         # 主题系统
│   │   ├── widgets/
│   │   │   ├── ItemGrid.h/cpp    # 项目网格
│   │   │   └── CategoryTab.h/cpp # 分类标签
│   │   └── dialogs/
│   │       └── EditDialog.h/cpp  # 编辑对话框
│   ├── core/
│   │   ├── Types.h             # 类型定义
│   │   ├── Config.h/cpp        # 配置管理
│   │   ├── Storage.h/cpp       # JSON 存储
│   │   ├── IconCache.h/cpp     # 图标缓存
│   │   ├── IconTextureManager.h/cpp # 图标纹理管理
│   │   ├── ItemManager.h/cpp   # 项目管理器
│   │   ├── Runner.h/cpp        # 程序启动器
│   │   └── HotkeyManager.h/cpp # 快捷键管理
│   ├── platform/
│   │   ├── Window.h/cpp        # Win32 窗口
│   │   ├── D3D11Renderer.h/cpp # DX11 渲染器
│   │   └── TrayIcon.h/cpp      # 托盘图标
│   └── utils/
│       ├── StringUtils.h/cpp   # 字符串工具
│       └── PathUtils.h/cpp     # 路径工具
├── rs/                         # Rust 重写版本
│   ├── Cargo.toml              # Cargo 配置
│   ├── build.rs                # Slint 编译脚本
│   ├── src/
│   │   ├── main.rs             # 程序入口
│   │   ├── lib.rs              # 库导出
│   │   ├── config.rs           # 配置管理
│   │   ├── hotkey.rs           # 快捷键管理
│   │   ├── icon_cache.rs       # 图标缓存
│   │   ├── item_manager.rs     # 项目管理器
│   │   ├── localization.rs     # 多语言支持
│   │   ├── model.rs            # 数据模型
│   │   ├── runner.rs           # 程序启动器
│   │   ├── storage.rs          # JSON 存储
│   │   ├── tray.rs             # 托盘图标
│   │   └── platform/           # 平台相关（Windows）
│   │       ├── mod.rs          # 平台模块入口
│   │       └── dragdrop.rs     # OLE 拖放实现
│   ├── ui/
│   │   ├── main_window.slint   # 主窗口 UI
│   │   └── edit_dialog.slint   # 编辑对话框 UI
│   └── tests/
│       ├── gui_integration_test.rs  # GUI 集成测试
│       └── storage_test.rs     # 存储单元测试
├── res/
│   ├── resource.rc             # Windows 资源
│   └── icons/
│       └── app.ico             # 应用图标
├── lang/                       # 多语言文件
│   ├── zh-CN.json              # 简体中文（默认）
│   ├── zh-TW.json              # 繁体中文
│   └── en-US.json              # 英文
├── img/                        # 图片资源
│   ├── Logo.png                # 项目 Logo
│   ├── 01-06.png               # 功能截图
│   ├── ScrriptTest.gif         # 脚本功能演示
│   └── theme/                  # 主题截图
├── SDK/                        # 插件开发 SDK（规划中）
│   ├── SDK_MN.hpp              # SDK 头文件
│   └── mn.cc.arae.Demo/        # 插件开发示例
│       ├── p_Demo.cpp          # 示例源码
│       ├── demo.png            # 示例图标
│       └── _testImgBlue.png
├── docs/
│   └── superpowers/
│       ├── plans/              # 实现计划
│       └── specs/              # 设计规格
└── .github/
    ├── FUNDING.yml             # GitHub 赞助配置
    └── workflows/
        ├── build.yml           # C++ CI/CD 构建配置
        └── rust-build.yml      # Rust CI/CD 构建配置
```

---

## 构建指南

### 环境要求

- Visual Studio 2019+ 或 MSVC Build Tools
- CMake 3.16+

### 构建步骤

```bash
# 生成项目文件
cmake -B build -A x64

# 编译（Release 模式）
cmake --build build --config Release

# 编译（Debug 模式）
cmake --build build --config Debug
```

### 依赖项

项目使用 CMake FetchContent 自动下载以下依赖：

| 依赖 | 版本 | 用途 |
|-----|------|-----|
| ImGui | docking 分支 | UI 框架 |
| nlohmann/json | v3.11.2 | JSON 解析 |

---

## Rust 重写版本构建

### 环境要求

- Rust 1.70+ (2021 edition)
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
|-----|------|-----|
| slint | 1.7+ | UI 框架 (Skia 渲染) |
| serde | 1.x | 序列化 |
| serde_json | 1.x | JSON 解析 |
| dirs | 5.x | 系统目录 |
| image | 0.25 | 图像处理 |
| raw-window-handle | 0.6 | 窗口句柄 |
| windows | 0.57 | Windows API 绑定 |
| tray-icon | 0.14 | 系统托盘 |

---

## 数据存储

### 配置文件

**路径**：`%EXEDIR%/config.json`（可执行文件所在目录）

首次运行时自动创建目录和默认配置文件。

### 图标缓存

**路径**：`%EXEDIR%/icons/`

**缓存策略**：
- 添加项目时自动提取图标并缓存
- 文件命名格式：`item_<timestamp>_<random>.png`
- 删除项目时同步删除缓存图标

### JSON 数据结构

```json
{
    "version": "1.0.0",
    "categories": [
        {
            "id": "cat_<timestamp>_<random>",
            "name": "分类名称",
            "sortOrder": 0,
            "viewType": 0,
            "iconSize": 48
        }
    ],
    "items": [
        {
            "id": "item_<timestamp>_<random>",
            "name": "显示名称",
            "target": "目标路径",
            "arguments": "",
            "workingDir": "",
            "iconPath": "",
            "iconIndex": 0,
            "runAsAdmin": false,
            "runCount": 0,
            "keywords": "",
            "remark": "",
            "categoryId": "cat_<timestamp>_<random>",
            "sortOrder": 0
        }
    ],
    "config": {
        "globalHotkey": "Ctrl+Alt+M",
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
| `%so-url%` | 搜索参数 URL 编码 | 2.8.0.0+ |

---

## 多语言支持

语言文件位于 `lang/` 目录，使用 JSON 格式存储：

- `zh-CN.json` - 简体中文（默认）
- `zh-TW.json` - 繁体中文
- `en-US.json` - 英文

### 语言文件结构

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

项目提供了插件开发 SDK（`SDK/SDK_MN.hpp`），计划支持搜索插件扩展。

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

> **注意**：插件系统尚未实现，SDK 文档仅供规划参考。完整示例见 `SDK/mn.cc.arae.Demo/p_Demo.cpp`。

---

## 核心组件架构

### 应用主类（App）

```cpp
class App {
public:
    bool Initialize(HINSTANCE hInstance);
    int Run();
    void Shutdown();
    void Quit();
    
    static App* Get();
    ItemManager* GetItemManager();
    Config* GetConfig();
    MainWindow* GetMainWindow();
    IconTextureManager* GetIconTextureManager();
};
```

### 数据模型

```cpp
// 分类
struct Category {
    std::wstring id;           // 格式: cat_<timestamp>_<random>
    std::wstring name;
    int sortOrder = 0;
    ViewType viewType = ViewType::Icon;
    int iconSize = 48;
};

// 快捷项
struct Item {
    std::wstring id;           // 格式: item_<timestamp>_<random>
    std::wstring name;
    std::wstring target;       // 目标路径
    std::wstring arguments;    // 启动参数
    std::wstring workingDir;   // 工作目录
    std::wstring iconPath;
    int iconIndex = 0;
    bool runAsAdmin = false;
    int runCount = 0;
    std::wstring keywords;
    std::wstring remark;
    std::wstring categoryId;
    int sortOrder = 0;
};
```

### 组件职责

- **Storage**：JSON 数据读写，UTF-8 编码存储
- **Config**：应用配置管理（窗口位置、快捷键等）
- **IconCache**：图标提取与缓存管理
- **IconTextureManager**：ImGui 纹理管理
- **ItemManager**：分类和项目的内存状态管理
- **Runner**：程序启动执行，错误处理
- **HotkeyManager**：全局快捷键注册与处理
- **Window**：Win32 窗口封装
- **D3D11Renderer**：DirectX 11 渲染器
- **MainWindow**：ImGui 主界面渲染
- **TrayIcon**：系统托盘图标

---

## 开发约定

### 编码规范

- **C++ 标准**：C++17
- **字符编码**：
  - 源文件：UTF-8
  - 字符串字面量：使用 `u8` 前缀
  - 内部处理：`std::wstring`（Windows API）
  - 存储/ImGui：UTF-8 (`std::string`)
- **命名规范**：
  - 类名：PascalCase
  - 函数/方法：PascalCase
  - 成员变量：m_camelCase（前缀 m_）
  - 局部变量：camelCase
  - 常量：UPPER_CASE

### 字符串转换

```cpp
// wstring <-> UTF-8（ImGui 使用 UTF-8）
namespace StringUtils {
    std::string WStringToUtf8(const std::wstring& wstr);
    std::wstring Utf8ToWString(const std::string& str);
}
```

### 内存管理

- 使用 `std::unique_ptr` 管理组件生命周期
- 依赖注入模式（通过 `Initialize()` 传入依赖）
- 单例模式（`App::Get()`）

---

## CI/CD

### GitHub Actions

- **触发条件**：push/PR 到 main 分支
- **构建矩阵**：Release 和 Debug 配置
- **产物保留**：30 天
- **构建路径**：`build/{Release|Debug}/GoRun.exe`

#### C++ 构建（`.github/workflows/build.yml`）

| 项目 | 说明 |
|------|------|
| 触发路径 | `src/**`、`cmake/**`、`CMakeLists.txt`、`res/**` |
| Runner | `windows-latest` |
| 矩阵 | Release / Debug 双配置 |
| 产物 | `GoRun-{Release,Debug}` 各 30 天 |

#### Rust 构建（`.github/workflows/rust-build.yml`）

| 项目 | 说明 |
|------|------|
| 触发路径 | `rs/**` |
| Runner | `windows-latest` + `ubuntu-latest` |
| Windows 任务 | `cargo build` + `cargo test --lib` + 产物上传 |
| Linux 任务 | xvfb + GUI 集成测试 (`gui_integration_test.rs`) |
| 产物 | `GoRun-rs-release` 30 天 |

---

## 项目链接

- **当前仓库**：https://github.com/AoThen/GoRun
- **原版仓库**：https://github.com/25H/GoRun
- **官方网站**：https://GoRun.arae.cc/
- **更新日志**：https://soft.arae.cc/ChangeLog/GoRun.txt

---

## 注意事项

1. **Windows 平台专用**：本项目仅支持 Windows 10 及以上系统
2. **原生开发**：不使用 WebView 或 Electron，保持轻量
3. **绿色软件**：不产生系统垃圾，可便携运行
4. **数据安全**：每次操作后立即保存，确保数据不丢失
5. **MVP 阶段**：部分高级功能正在规划中，详见"规划中"章节
6. **插件开发**：需要使用 Visual Studio 或兼容的 C++ 编译器，导出函数使用 `__declspec(dllexport)`
7. **Rust 重写**：正在进行中，位于 `rs/` 目录，与 C++ 代码并行存在

---

## 致谢

感谢 [25H](https://github.com/25H) 创建的原版 GoRun 项目，为本开源重写版本提供了设计灵感和参考。