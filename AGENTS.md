# Maye Nano - 项目上下文

## 项目概述

**Maye Nano** 是一款 Windows 平台的快速启动工具，专注于"快速启动"的纯粹体验。项目使用 **C++** 和 **ImGui** 开发，采用原生架构，无 WebView/Electron 依赖，具有毫秒级启动和极低资源占用的特点。

### 核心特性

- **极速启动**：100% 原生开发，毫秒级响应
- **低能耗**：深度优化底层架构，极低资源占用
- **插件系统**：支持搜索插件扩展
- **主题系统**：内置可视化主题编辑器
- **任务计划**：支持定时执行、开机自启、周期循环等触发模式
- **脚本支持**：内置脚本运行机制
- **多语言**：支持简体中文、繁体中文、英文

### 运行环境

- **操作系统**：Windows 10+
- **编写语言**：C++ / ImGui
- **软件大小**：约 3 MB+

---

## 目录结构

```
MayeNano/
├── README.md              # 项目主文档
├── AGENTS.md              # AI 代理上下文文件（本文件）
├── img/                   # 图片资源
│   ├── Logo.png           # 项目 Logo
│   ├── 01-06.png          # 功能截图
│   ├── ScrriptTest.gif    # 脚本功能演示
│   ├── vx-25H.jpg         # 微信群二维码
│   └── theme/             # 主题截图
├── lang/                  # 多语言文件
│   ├── zh-CN.json         # 简体中文
│   ├── zh-TW.json         # 繁体中文
│   └── en-US.json         # 英文
├── SDK/                   # 插件开发 SDK
│   ├── SDK_MN.hpp         # SDK 头文件
│   └── mn.cc.arae.Demo/   # 插件开发示例
│       ├── p_Demo.cpp     # 示例源码
│       ├── demo.png       # 示例图标
│       └── _testImgBlue.png
├── ToMayeNano/            # 数据迁移工具
│   ├── README.md          # 迁移工具说明
│   └── ToMayeNano.zip     # 迁移工具
└── .github/
    └── FUNDING.yml        # GitHub 赞助配置
```

---

## 插件开发

### SDK 概述

项目提供了插件开发 SDK（`SDK/SDK_MN.hpp`），目前支持 **搜索插件** 类型。

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

### 插件结构

插件需要实现以下核心组件：

1. **插件信息结构** `_MN_PLUGIN_INFO`
   - 名称、描述、版本、作者等信息
   - 事件通知函数指针 `fnNotify`

2. **搜索信息结构** `_MN_PLUGIN_SEARCH_INFO`
   - 搜索名称、描述、图标
   - 触发关键字或正则表达式
   - 搜索回调函数

3. **搜索结果结构** `_MN_PLUGIN_SEARCH_RESULT_ITEM`
   - 结果名称、图标、权重
   - 执行动作类型

### API 函数

| 函数 | 说明 |
|------|------|
| `GetVersion()` | 获取 Maye Nano 版本号 |
| `GetSelfId()` | 获取插件自身 ID |
| `GetSelfDir()` | 获取插件目录路径 |
| `SetCfgItem()` | 写入配置项 |
| `GetCfgItem()` | 读取配置项 |

### 示例代码

参考 `SDK/mn.cc.arae.Demo/p_Demo.cpp` 获取完整的搜索插件示例。

---

## 参数变量

Maye Nano 支持以下参数变量：

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

## 数据迁移

`ToMayeNano` 工具用于将旧版 Maye / Maye Lite 的数据库（JDB.json）转换为 Maye Nano 格式。

---

## 开发约定

### 编码规范

- **C++ 文件**：使用 UTF-8 编码，字符串使用 `u8` 前缀
- **JSON 文件**：使用 UTF-8 编码
- **命名规范**：驼峰命名法（camelCase）

### 项目主页

- 官方主页：https://mayenano.arae.cc/
- GitHub：https://github.com/25H/MayeNano
- 更新日志：https://soft.arae.cc/ChangeLog/MayeNano.txt

---

## 注意事项

1. **Windows 平台专用**：本项目仅支持 Windows 10 及以上系统
2. **原生开发**：不使用 WebView 或 Electron，保持轻量
3. **绿色软件**：不产生系统垃圾，可便携运行
4. **插件开发**：需要使用 Visual Studio 或兼容的 C++ 编译器，导出函数使用 `__declspec(dllexport)`
