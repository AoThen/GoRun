# Maye Nano MVP 设计规格

## 概述

Maye Nano 是一款 Windows 平台的快速启动工具，使用 C++ 和 ImGui 开发。本文档定义 MVP（最小可行产品）的设计规格，包含核心启动功能。

## 技术栈

| 项目 | 选择 |
|-----|------|
| 语言 | C++17 |
| 构建 | CMake |
| 渲染 | DirectX 11 + ImGui (docking) |
| 存储 | JSON (nlohmann/json) |
| 依赖管理 | FetchContent |
| 平台 | Windows 10+ |

## MVP 功能范围

1. **窗口框架** - 基于 DirectX 11 + ImGui 的主窗口
2. **拖放添加** - 支持拖放文件/快捷方式添加项目
3. **分类管理** - 创建、编辑、删除分类
4. **项目运行** - 双击运行、管理员运行
5. **全局快捷键** - 通过快捷键唤醒/隐藏主窗口

## 项目结构

```
MayeNano/
├── CMakeLists.txt
├── cmake/
│   └── FetchDependencies.cmake
├── src/
│   ├── main.cpp
│   ├── app/
│   │   ├── App.h
│   │   ├── App.cpp
│   │   └── Resource.h
│   ├── ui/
│   │   ├── MainWindow.h/cpp
│   │   ├── widgets/
│   │   │   ├── ItemGrid.h/cpp
│   │   │   └── CategoryTab.h/cpp
│   │   └── dialogs/
│   │       └── EditDialog.h/cpp
│   ├── core/
│   │   ├── Config.h/cpp
│   │   ├── Storage.h/cpp
│   │   ├── Item.h/cpp
│   │   ├── Category.h/cpp
│   │   ├── ItemManager.h/cpp
│   │   ├── Runner.h/cpp
│   │   └── HotkeyManager.h/cpp
│   ├── platform/
│   │   ├── Window.h/cpp
│   │   ├── D3D11Renderer.h/cpp
│   │   ├── DragDrop.h/cpp
│   │   └── ShellExecute.h/cpp
│   └── utils/
│       ├── PathUtils.h/cpp
│       └── StringUtils.h/cpp
├── res/
│   └── icons/
├── lang/                   # 已存在
├── img/                    # 已存在
└── SDK/                    # 已存在
```

## 数据模型

### Item（快捷项）

```cpp
struct Item {
    std::wstring id;           // 唯一标识
    std::wstring name;         // 显示名称
    std::wstring target;       // 目标路径
    std::wstring arguments;    // 启动参数
    std::wstring workingDir;   // 工作目录
    std::wstring iconPath;     // 图标路径
    int iconIndex = 0;         // 图标索引
    bool runAsAdmin = false;   // 以管理员运行
    int runCount = 0;          // 运行次数
    std::wstring keywords;     // 搜索关键词
    std::wstring remark;       // 备注
    std::wstring categoryId;   // 所属分类 ID
    int sortOrder = 0;         // 排序顺序
};
```

### Category（分类）

```cpp
struct Category {
    std::wstring id;           // 唯一标识
    std::wstring name;         // 分类名称
    int sortOrder = 0;         // 排序顺序
    bool isPasswordProtected = false;
    ViewType viewType = ViewType::Icon;
    int iconSize = 48;
};
```

### JSON 存储格式

```json
{
    "version": "1.0.0",
    "categories": [
        {
            "id": "cat_001",
            "name": "常用工具",
            "sortOrder": 0
        }
    ],
    "items": [
        {
            "id": "item_001",
            "name": "记事本",
            "target": "notepad.exe",
            "categoryId": "cat_001"
        }
    ],
    "config": {
        "language": "zh-CN",
        "globalHotkey": "Ctrl+Alt+M"
    }
}
```

## 核心组件

### Storage（存储引擎）

负责 JSON 数据的读写操作。

```cpp
class Storage {
public:
    bool Load(const std::wstring& path);
    bool Save(const std::wstring& path);
    
    std::vector<Category> GetCategories();
    std::vector<Item> GetItems(const std::wstring& categoryId = L"");
    Item GetItem(const std::wstring& id);
    
    bool AddCategory(const Category& category);
    bool UpdateCategory(const Category& category);
    bool DeleteCategory(const std::wstring& id);
    
    bool AddItem(const Item& item);
    bool UpdateItem(const Item& item);
    bool DeleteItem(const std::wstring& id);
    
    std::wstring GetConfig(const std::string& key, const std::wstring& defaultVal = L"");
    bool SetConfig(const std::string& key, const std::wstring& value);
};
```

### ItemManager（项目管理器）

管理分类和项目的内存状态，响应 UI 操作。

```cpp
class ItemManager {
public:
    void Initialize(Storage* storage);
    
    std::vector<Category>& GetCategories();
    Category* GetCategory(const std::wstring& id);
    void AddCategory(Category category);
    void DeleteCategory(const std::wstring& id);
    
    std::vector<Item>& GetItems(const std::wstring& categoryId);
    Item* GetItem(const std::wstring& id);
    void AddItem(Item item);
    void MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId);
    void DeleteItem(const std::wstring& id);
    
    void HandleDrop(const std::vector<std::wstring>& files, const std::wstring& categoryId);
};
```

### Runner（程序启动器）

执行项目启动操作。

```cpp
class Runner {
public:
    bool Run(const Item& item);
    bool RunAsAdmin(const Item& item);
    bool RunMinimized(const Item& item);
};
```

### HotkeyManager（全局快捷键）

注册和处理 Windows 全局快捷键。

```cpp
class HotkeyManager {
public:
    bool RegisterGlobalHotkey(int id, UINT modifiers, UINT vk);
    bool UnregisterGlobalHotkey(int id);
    void ProcessHotkey(WPARAM wParam);
    
    using HotkeyCallback = std::function<void(int id)>;
    void SetCallback(HotkeyCallback callback);
};
```

## 平台层

### Window（窗口封装）

封装 Win32 窗口创建和消息处理。

```cpp
class Window {
public:
    bool Create(const std::wstring& title, int width, int height);
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    
    LRESULT WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    void EnableDragDrop();
    
    HWND GetHandle() const;
    
    void OnDropFiles(std::function<void(const std::vector<std::wstring>&)> callback);
    void OnHotkey(std::function<void(int id)> callback);
    void OnResize(std::function<void(int w, int h)> callback);
};
```

### D3D11Renderer（渲染器）

管理 DirectX 11 设备和 ImGui 渲染。

```cpp
class D3D11Renderer {
public:
    bool Initialize(HWND hwnd, int width, int height);
    void Shutdown();
    void NewFrame();
    void Render();
    void Resize(int width, int height);
    
    ImGuiContext* GetContext();
};
```

## UI 层

### MainWindow（主窗口）

渲染主界面，包括分类标签和项目网格。

```cpp
class MainWindow {
public:
    void Initialize(ItemManager* itemManager, Config* config);
    void Render();
    
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    
    void SetCurrentCategory(const std::wstring& categoryId);

private:
    void RenderCategoryTabs();
    void RenderItemGrid();
    void RenderContextMenu();
    void HandleDragDrop();
};
```

## 应用主流程

```cpp
class App {
public:
    bool Initialize(HINSTANCE hInstance);
    int Run();
    void Shutdown();
    
    static App* Get();
    ItemManager* GetItemManager();
    Config* GetConfig();
    MainWindow* GetMainWindow();

private:
    void MainLoop();
    void HandleHotkey(int id);
};
```

**启动流程：**

1. `App::Initialize()` - 初始化各组件
2. `Storage::Load()` - 加载数据文件
3. `HotkeyManager::RegisterGlobalHotkey()` - 注册全局快捷键
4. `App::Run()` - 进入主消息循环
5. 收到快捷键 → `MainWindow::Toggle()` 显示/隐藏窗口
6. 用户操作 → `ItemManager` 更新数据 → `Storage::Save()` 保存

## 依赖

| 依赖 | 版本 | 用途 |
|-----|------|-----|
| ImGui | docking 分支 | UI 框架 |
| nlohmann/json | v3.11.2 | JSON 解析 |

## 构建配置

```cmake
cmake_minimum_required(VERSION 3.16)
project(MayeNano VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_GENERATOR_PLATFORM x64)

include(cmake/FetchDependencies.cmake)

file(GLOB_RECURSE SOURCES "src/*.cpp")
file(GLOB_RECURSE HEADERS "src/*.h")

add_executable(MayeNano WIN32 ${SOURCES} ${HEADERS})

target_link_libraries(MayeNano
    PRIVATE imgui nlohmann_json d3d11 dxgi shell32 ole32)

target_include_directories(MayeNano PRIVATE "${CMAKE_SOURCE_DIR}/src")
target_sources(MayeNano PRIVATE "res/resource.rc")

set_target_properties(MayeNano PROPERTIES WIN32_EXECUTABLE TRUE)
```

## 后续扩展

MVP 完成后可扩展的功能：

1. 搜索功能
2. 主题系统
3. 插件系统
4. 任务计划
5. 脚本支持
6. 托盘图标
