# GoRun - 极速启动工具

**GoRun** 是一款专注于"快速启动"的纯粹工具。我们深知效率的本质是减法而非加法，因此摒弃一切冗余，仅保留最极致的启动体验。

> 本项目为 GoRun 的开源重写版本，使用 C++ 和 ImGui 开发，采用原生架构。

![GoRun Logo](img/Logo.png)

## 特色亮点

**现代化架构** - 拥抱前沿技术栈，以超前架构定义性能，深度融合新系统特性。

**极致的快** - 100% 原生开发，毫秒级启动、即时响应。无 WebView，无 Electron，性能与体验全面领先。

**低能耗** - 深度优化底层架构，实现极低资源占用，长时间运行依然轻盈高效。

## 已实现功能

### 核心功能
- 窗口框架（DirectX 11 + ImGui）
- 拖放添加文件/快捷方式
- 分类管理（创建、编辑、删除）
- 项目管理（添加、编辑、删除、拖拽排序）
- 双击运行 / 管理员运行
- 全局快捷键唤醒/隐藏主窗口
- 托盘图标

### 数据管理
- JSON 格式存储配置和数据
- 图标缓存系统
- 配置持久化（窗口位置、大小等）

### UI 组件
- 分类标签栏
- 项目网格视图
- 编辑对话框
- 主题系统基础框架

## 技术栈

| 项目 | 选择 |
|-----|------|
| 语言 | C++17 |
| 构建 | CMake |
| 渲染 | DirectX 11 + ImGui (docking) |
| 存储 | JSON (nlohmann/json) |
| 依赖管理 | FetchContent |
| 平台 | Windows 10+ |

## 构建

### 环境要求

- Visual Studio 2019+ 或 MSVC Build Tools
- CMake 3.16+

### 构建步骤

```bash
# 生成项目文件
cmake -B build -A x64

# 编译
cmake --build build --config Release
```

### 依赖项

项目使用 CMake FetchContent 自动下载以下依赖：

| 依赖 | 版本 | 用途 |
|-----|------|------|
| ImGui | docking 分支 | UI 框架 |
| nlohmann/json | v3.11.2 | JSON 解析 |

## 项目结构

```
GoRun/
├── CMakeLists.txt              # CMake 配置
├── cmake/
│   └── FetchDependencies.cmake # 依赖获取脚本
├── src/
│   ├── main.cpp                # 程序入口
│   ├── app/
│   │   ├── App.h/cpp           # 应用主类
│   │   └── Resource.h          # 资源定义
│   ├── ui/
│   │   ├── MainWindow.h/cpp    # 主窗口
│   │   ├── Theme.h/cpp         # 主题系统
│   │   ├── widgets/
│   │   │   ├── ItemGrid.h/cpp  # 项目网格
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
├── res/
│   ├── resource.rc             # Windows 资源
│   └── icons/
│       └── app.ico             # 应用图标
├── lang/                       # 多语言文件
│   ├── zh-CN.json
│   ├── zh-TW.json
│   └── en-US.json
├── img/                        # 图片资源
├── SDK/                        # 插件开发 SDK
└── ToGoRun/                 # 数据迁移工具
```

## 数据存储

**配置文件路径**：`%APPDATA%/GoRun/config.json`

首次运行时自动创建目录和默认配置文件。

**图标缓存**：`%APPDATA%/GoRun/icons/`

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

### 后续扩展
- [ ] 搜索功能
- [ ] 主题编辑器
- [ ] 插件系统
- [ ] 任务计划
- [ ] 脚本支持
- [ ] 密码保护分类
- [ ] 列表/平铺视图

## 截图

![GoRun](img/01.png)

<table>
  <tr>
    <td><img src="img/ScrriptTest.gif" alt="脚本测试"></td>
    <td><img src="img/02.png" alt="界面截图"></td>
    <td><img src="img/03.png" alt="界面截图"></td>
  </tr>
  <tr>
    <td><img src="img/04.png" alt="界面截图"></td>
    <td><img src="img/05.png" alt="界面截图"></td>
    <td><img src="img/06.png" alt="界面截图"></td>
  </tr>
</table>

**主题预览**

<table>
  <tr>
    <td><img src="img/theme/01.png" alt="主题"></td>
    <td><img src="img/theme/02.png" alt="主题"></td>
    <td><img src="img/theme/03.png" alt="主题"></td>
  </tr>
  <tr>
    <td><img src="img/theme/04.png" alt="主题"></td>
    <td><img src="img/theme/05.png" alt="主题"></td>
    <td><img src="img/theme/06.png" alt="主题"></td>
  </tr>
  <tr>
    <td><img src="img/theme/07.png" alt="主题"></td>
    <td><img src="img/theme/08.png" alt="主题"></td>
    <td><img src="img/theme/09.png" alt="主题"></td>
  </tr>
</table>

## 相关链接

- **官方网站**: https://GoRun.arae.cc/
- **GitHub**: https://github.com/AoThen/GoRun
- **原版仓库**: https://github.com/25H/GoRun
- **更新日志**: https://soft.arae.cc/ChangeLog/GoRun.txt

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

本项目采用 MIT 许可证。

## 致谢

感谢 [25H](https://github.com/25H) 创建的原版 GoRun 项目。

---

**备注**
- 名称：GoRun 快速启动工具
- 大小：约 3 MB+
- 语言：多语言
- 运行环境：Windows 10+
- 授权方式：免费软件
- 软件类别：桌面工具
- 编写语言：C++ / ImGui