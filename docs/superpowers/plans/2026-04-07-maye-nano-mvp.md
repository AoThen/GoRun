# Maye Nano MVP 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 构建 Windows 快速启动工具的 MVP 版本，支持窗口框架、拖放添加、分类管理、项目运行、全局快捷键。

**架构：** 分层架构 - 平台层（Win32/DX11）→ 核心层（数据/业务）→ UI层（ImGui）→ 应用层。依赖注入，单向依赖。

**技术栈：** C++17 / CMake / DirectX 11 / ImGui (docking) / nlohmann/json

---

## 文件结构

```
src/
├── main.cpp                        # 程序入口
├── app/
│   ├── App.h/cpp                   # 应用主类
│   └── Resource.h                  # 资源定义
├── ui/
│   ├── MainWindow.h/cpp            # 主窗口
│   ├── widgets/
│   │   ├── ItemGrid.h/cpp          # 项目网格
│   │   └── CategoryTab.h/cpp       # 分类标签
│   └── dialogs/
│       └── EditDialog.h/cpp        # 编辑对话框
├── core/
│   ├── Types.h                     # 类型定义（枚举、结构体）
│   ├── Config.h/cpp                # 配置管理
│   ├── Storage.h/cpp               # JSON 存储
│   ├── IconCache.h/cpp             # 图标缓存
│   ├── ItemManager.h/cpp           # 项目管理器
│   ├── Runner.h/cpp                # 程序启动器
│   └── HotkeyManager.h/cpp         # 快捷键管理
├── platform/
│   ├── Window.h/cpp                # Win32 窗口
│   ├── D3D11Renderer.h/cpp         # DX11 渲染器
│   └── DragDrop.h/cpp              # 拖放处理
└── utils/
    ├── StringUtils.h/cpp           # 字符串工具
    └── PathUtils.h/cpp             # 路径工具
```

---

## 阶段一：项目搭建

### 任务 1：CMake 构建配置

**文件：**
- 创建：`CMakeLists.txt`
- 创建：`cmake/FetchDependencies.cmake`

- [ ] **步骤 1：创建 cmake 目录和依赖配置**

```cmake
# cmake/FetchDependencies.cmake
include(FetchContent)

# ImGui
FetchContent_Declare(
    imgui
    GIT_REPOSITORY https://github.com/ocornut/imgui.git
    GIT_TAG docking
)

# nlohmann/json
FetchContent_Declare(
    nlohmann_json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG v3.11.2
)

FetchContent_MakeAvailable(imgui nlohmann_json)

# ImGui 源文件
set(IMGUI_SOURCES
    ${imgui_SOURCE_DIR}/imgui.cpp
    ${imgui_SOURCE_DIR}/imgui_draw.cpp
    ${imgui_SOURCE_DIR}/imgui_tables.cpp
    ${imgui_SOURCE_DIR}/imgui_widgets.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_win32.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_dx11.cpp
)

add_library(imgui STATIC ${IMGUI_SOURCES})
target_include_directories(imgui PUBLIC 
    ${imgui_SOURCE_DIR} 
    ${imgui_SOURCE_DIR}/backends
)
```

- [ ] **步骤 2：创建主 CMakeLists.txt**

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.16)
project(MayeNano VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(cmake/FetchDependencies.cmake)

file(GLOB_RECURSE SOURCES "src/*.cpp")
file(GLOB_RECURSE HEADERS "src/*.h")

add_executable(MayeNano WIN32 ${SOURCES} ${HEADERS})

target_link_libraries(MayeNano
    PRIVATE imgui nlohmann_json d3d11 dxgi shell32 ole32)

target_include_directories(MayeNano PRIVATE "${CMAKE_SOURCE_DIR}/src")
```

- [ ] **步骤 3：创建目录结构**

```bash
mkdir -p src/app src/ui/widgets src/ui/dialogs src/core src/platform src/utils res/icons
```

- [ ] **步骤 4：Commit**

```bash
git add CMakeLists.txt cmake/
git commit -m "chore: setup CMake build configuration"
```

---

### 任务 2：类型定义

**文件：**
- 创建：`src/core/Types.h`

- [ ] **步骤 1：创建类型定义头文件**

```cpp
// src/core/Types.h
#pragma once

#include <string>
#include <vector>
#include <functional>
#include <chrono>
#include <random>

namespace mn {

// 视图类型
enum class ViewType {
    Icon = 0
};

// 运行错误类型
enum class RunError {
    None = 0,
    FileNotFound,
    PathNotFound,
    AccessDenied,
    OutOfMemory,
    DllNotFound,
    Unknown
};

// 分类
struct Category {
    std::wstring id;
    std::wstring name;
    int sortOrder = 0;
    ViewType viewType = ViewType::Icon;
    int iconSize = 48;
};

// 快捷项
struct Item {
    std::wstring id;
    std::wstring name;
    std::wstring target;
    std::wstring arguments;
    std::wstring workingDir;
    std::wstring iconPath;
    int iconIndex = 0;
    bool runAsAdmin = false;
    int runCount = 0;
    std::wstring keywords;
    std::wstring remark;
    std::wstring categoryId;
    int sortOrder = 0;
};

// 运行结果
struct RunResult {
    bool success = false;
    RunError error = RunError::None;
    std::wstring errorMessage;
};

// ID 生成器
inline std::wstring GenerateId(const std::wstring& prefix) {
    auto now = std::chrono::system_clock::now().time_since_epoch();
    auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(now).count();
    std::random_device rd;
    int random = rd() % 10000;
    return prefix + L"_" + std::to_wstring(ms) + L"_" + std::to_wstring(random);
}

} // namespace mn
```

- [ ] **步骤 2：Commit**

```bash
git add src/core/Types.h
git commit -m "feat: add core type definitions"
```

---

## 阶段二：工具类

### 任务 3：字符串工具

**文件：**
- 创建：`src/utils/StringUtils.h`
- 创建：`src/utils/StringUtils.cpp`

- [ ] **步骤 1：创建 StringUtils.h**

```cpp
// src/utils/StringUtils.h
#pragma once

#include <string>
#include <vector>

namespace mn::StringUtils {

std::string WStringToUtf8(const std::wstring& wstr);
std::wstring Utf8ToWString(const std::string& str);
std::vector<std::wstring> Split(const std::wstring& str, wchar_t delimiter);
std::wstring Trim(const std::wstring& str);
std::wstring ToLower(const std::wstring& str);

} // namespace mn::StringUtils
```

- [ ] **步骤 2：创建 StringUtils.cpp**

```cpp
// src/utils/StringUtils.cpp
#include "StringUtils.h"
#include <algorithm>
#include <Windows.h>

namespace mn::StringUtils {

std::string WStringToUtf8(const std::wstring& wstr) {
    if (wstr.empty()) return {};
    int size = WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), -1, nullptr, 0, nullptr, nullptr);
    std::string result(size - 1, 0);
    WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), -1, result.data(), size, nullptr, nullptr);
    return result;
}

std::wstring Utf8ToWString(const std::string& str) {
    if (str.empty()) return {};
    int size = MultiByteToWideChar(CP_UTF8, 0, str.c_str(), -1, nullptr, 0);
    std::wstring result(size - 1, 0);
    MultiByteToWideChar(CP_UTF8, 0, str.c_str(), -1, result.data(), size);
    return result;
}

std::vector<std::wstring> Split(const std::wstring& str, wchar_t delimiter) {
    std::vector<std::wstring> tokens;
    size_t start = 0;
    size_t end = str.find(delimiter);
    while (end != std::wstring::npos) {
        tokens.push_back(str.substr(start, end - start));
        start = end + 1;
        end = str.find(delimiter, start);
    }
    tokens.push_back(str.substr(start));
    return tokens;
}

std::wstring Trim(const std::wstring& str) {
    auto start = str.find_first_not_of(L" \t\r\n");
    if (start == std::wstring::npos) return {};
    auto end = str.find_last_not_of(L" \t\r\n");
    return str.substr(start, end - start + 1);
}

std::wstring ToLower(const std::wstring& str) {
    std::wstring result = str;
    std::transform(result.begin(), result.end(), result.begin(), ::towlower);
    return result;
}

} // namespace mn::StringUtils
```

- [ ] **步骤 3：Commit**

```bash
git add src/utils/StringUtils.h src/utils/StringUtils.cpp
git commit -m "feat: add StringUtils utilities"
```

---

### 任务 4：路径工具

**文件：**
- 创建：`src/utils/PathUtils.h`
- 创建：`src/utils/PathUtils.cpp`

- [ ] **步骤 1：创建 PathUtils.h**

```cpp
// src/utils/PathUtils.h
#pragma once

#include <string>

namespace mn::PathUtils {

std::wstring GetAppDataPath();
std::wstring GetExePath();
std::wstring GetExeDir();
std::wstring ToAbsolute(const std::wstring& path);
std::wstring ToRelative(const std::wstring& path);
bool Exists(const std::wstring& path);
std::wstring GetParentDir(const std::wstring& path);
std::wstring GetFileName(const std::wstring& path);
bool EnsureDirectory(const std::wstring& path);

} // namespace mn::PathUtils
```

- [ ] **步骤 2：创建 PathUtils.cpp**

```cpp
// src/utils/PathUtils.cpp
#include "PathUtils.h"
#include <filesystem>
#include <Windows.h>
#include <ShlObj.h>

namespace mn::PathUtils {

std::wstring GetAppDataPath() {
    wchar_t path[MAX_PATH];
    SHGetFolderPathW(nullptr, CSIDL_APPDATA, nullptr, 0, path);
    return std::wstring(path) + L"\\MayeNano";
}

std::wstring GetExePath() {
    wchar_t path[MAX_PATH];
    GetModuleFileNameW(nullptr, path, MAX_PATH);
    return path;
}

std::wstring GetExeDir() {
    return GetParentDir(GetExePath());
}

std::wstring ToAbsolute(const std::wstring& path) {
    wchar_t absPath[MAX_PATH];
    GetFullPathNameW(path.c_str(), MAX_PATH, absPath, nullptr);
    return absPath;
}

std::wstring ToRelative(const std::wstring& path) {
    std::filesystem::path p(path);
    std::filesystem::path base(GetExeDir());
    return std::filesystem::relative(p, base).wstring();
}

bool Exists(const std::wstring& path) {
    return std::filesystem::exists(path);
}

std::wstring GetParentDir(const std::wstring& path) {
    return std::filesystem::path(path).parent_path().wstring();
}

std::wstring GetFileName(const std::wstring& path) {
    return std::filesystem::path(path).filename().wstring();
}

bool EnsureDirectory(const std::wstring& path) {
    return std::filesystem::create_directories(path);
}

} // namespace mn::PathUtils
```

- [ ] **步骤 3：Commit**

```bash
git add src/utils/PathUtils.h src/utils/PathUtils.cpp
git commit -m "feat: add PathUtils utilities"
```

---

## 阶段三：平台层

### 任务 5：Win32 窗口

**文件：**
- 创建：`src/platform/Window.h`
- 创建：`src/platform/Window.cpp`

- [ ] **步骤 1：创建 Window.h**

```cpp
// src/platform/Window.h
#pragma once

#include <string>
#include <functional>
#include <vector>
#include <Windows.h>

namespace mn {

class Window {
public:
    bool Create(const std::wstring& title, int width, int height, int x, int y);
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    
    void SetPosition(int x, int y);
    void SetSize(int width, int height);
    void GetPosition(int& x, int& y);
    void GetSize(int& width, int& height);
    
    HWND GetHandle() const { return m_hwnd; }
    
    void EnableDragDrop();
    
    // 事件回调
    using DropFilesCallback = std::function<void(const std::vector<std::wstring>&)>;
    using HotkeyCallback = std::function<void(int id)>;
    using ResizeCallback = std::function<void(int w, int h)>;
    using MoveCallback = std::function<void(int x, int y)>;
    
    void OnDropFiles(DropFilesCallback cb) { m_dropFilesCallback = cb; }
    void OnHotkey(HotkeyCallback cb) { m_hotkeyCallback = cb; }
    void OnResize(ResizeCallback cb) { m_resizeCallback = cb; }
    void OnMove(MoveCallback cb) { m_moveCallback = cb; }

private:
    static LRESULT CALLBACK StaticWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    LRESULT WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    
    HWND m_hwnd = nullptr;
    bool m_visible = false;
    
    DropFilesCallback m_dropFilesCallback;
    HotkeyCallback m_hotkeyCallback;
    ResizeCallback m_resizeCallback;
    MoveCallback m_moveCallback;
};

} // namespace mn
```

- [ ] **步骤 2：创建 Window.cpp**

```cpp
// src/platform/Window.cpp
#include "Window.h"
#include <ShellApi.h>

namespace mn {

static const wchar_t* CLASS_NAME = L"MayeNanoWindowClass";

bool Window::Create(const std::wstring& title, int width, int height, int x, int y) {
    // 注册窗口类
    WNDCLASSEXW wc = {};
    wc.cbSize = sizeof(WNDCLASSEXW);
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = StaticWndProc;
    wc.hInstance = GetModuleHandle(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = CLASS_NAME;
    RegisterClassExW(&wc);
    
    // 创建窗口
    m_hwnd = CreateWindowExW(
        WS_EX_APPWINDOW,
        CLASS_NAME,
        title.c_str(),
        WS_OVERLAPPEDWINDOW,
        x, y, width, height,
        nullptr, nullptr,
        GetModuleHandle(nullptr),
        this
    );
    
    return m_hwnd != nullptr;
}

void Window::Show() {
    m_visible = true;
    ShowWindow(m_hwnd, SW_SHOW);
    SetForegroundWindow(m_hwnd);
}

void Window::Hide() {
    m_visible = false;
    ShowWindow(m_hwnd, SW_HIDE);
}

void Window::Toggle() {
    if (m_visible) Hide();
    else Show();
}

bool Window::IsVisible() const {
    return m_visible;
}

void Window::SetPosition(int x, int y) {
    SetWindowPos(m_hwnd, nullptr, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

void Window::SetSize(int width, int height) {
    SetWindowPos(m_hwnd, nullptr, 0, 0, width, height, SWP_NOMOVE | SWP_NOZORDER);
}

void Window::GetPosition(int& x, int& y) {
    RECT rect;
    GetWindowRect(m_hwnd, &rect);
    x = rect.left;
    y = rect.top;
}

void Window::GetSize(int& width, int& height) {
    RECT rect;
    GetWindowRect(m_hwnd, &rect);
    width = rect.right - rect.left;
    height = rect.bottom - rect.top;
}

void Window::EnableDragDrop() {
    DragAcceptFiles(m_hwnd, TRUE);
}

LRESULT CALLBACK Window::StaticWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    Window* window = nullptr;
    if (msg == WM_NCCREATE) {
        CREATESTRUCT* cs = reinterpret_cast<CREATESTRUCT*>(lParam);
        window = reinterpret_cast<Window*>(cs->lpCreateParams);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(window));
    } else {
        window = reinterpret_cast<Window*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    }
    
    if (window) {
        return window->WndProc(hwnd, msg, wParam, lParam);
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}

LRESULT Window::WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
        case WM_HOTKEY:
            if (m_hotkeyCallback) {
                m_hotkeyCallback(static_cast<int>(wParam));
            }
            return 0;
            
        case WM_DROPFILES: {
            HDROP hDrop = reinterpret_cast<HDROP>(wParam);
            if (m_dropFilesCallback) {
                std::vector<std::wstring> files;
                UINT count = DragQueryFileW(hDrop, 0xFFFFFFFF, nullptr, 0);
                for (UINT i = 0; i < count; i++) {
                    wchar_t buffer[MAX_PATH];
                    DragQueryFileW(hDrop, i, buffer, MAX_PATH);
                    files.push_back(buffer);
                }
                m_dropFilesCallback(files);
            }
            DragFinish(hDrop);
            return 0;
        }
        
        case WM_SIZE:
            if (m_resizeCallback && wParam != SIZE_MINIMIZED) {
                m_resizeCallback(LOWORD(lParam), HIWORD(lParam));
            }
            return 0;
            
        case WM_MOVE:
            if (m_moveCallback) {
                m_moveCallback(LOWORD(lParam), HIWORD(lParam));
            }
            return 0;
            
        case WM_CLOSE:
            Hide();
            return 0;
            
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/platform/Window.h src/platform/Window.cpp
git commit -m "feat: add Win32 Window wrapper"
```

---

### 任务 6：DirectX 11 渲染器

**文件：**
- 创建：`src/platform/D3D11Renderer.h`
- 创建：`src/platform/D3D11Renderer.cpp`

- [ ] **步骤 1：创建 D3D11Renderer.h**

```cpp
// src/platform/D3D11Renderer.h
#pragma once

#include <d3d11.h>
#include <Windows.h>

struct ImGuiContext;

namespace mn {

class D3D11Renderer {
public:
    bool Initialize(HWND hwnd, int width, int height);
    void Shutdown();
    void NewFrame();
    void Render();
    void Resize(int width, int height);
    
    ImGuiContext* GetContext() { return m_imguiContext; }

private:
    ID3D11Device* m_device = nullptr;
    ID3D11DeviceContext* m_context = nullptr;
    IDXGISwapChain* m_swapChain = nullptr;
    ID3D11RenderTargetView* m_rtv = nullptr;
    ImGuiContext* m_imguiContext = nullptr;
};

} // namespace mn
```

- [ ] **步骤 2：创建 D3D11Renderer.cpp**

```cpp
// src/platform/D3D11Renderer.cpp
#include "D3D11Renderer.h"
#include <imgui.h>
#include <imgui_impl_win32.h>
#include <imgui_impl_dx11.h>

namespace mn {

bool D3D11Renderer::Initialize(HWND hwnd, int width, int height) {
    // 创建设备与交换链
    DXGI_SWAP_CHAIN_DESC scd = {};
    scd.BufferCount = 2;
    scd.BufferDesc.Width = width;
    scd.BufferDesc.Height = height;
    scd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    scd.BufferDesc.RefreshRate.Numerator = 60;
    scd.BufferDesc.RefreshRate.Denominator = 1;
    scd.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;
    scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    scd.OutputWindow = hwnd;
    scd.SampleDesc.Count = 1;
    scd.SampleDesc.Quality = 0;
    scd.Windowed = TRUE;
    scd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
    
    UINT createFlags = 0;
    // createFlags |= D3D11_CREATE_DEVICE_DEBUG;
    
    D3D_FEATURE_LEVEL featureLevel;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(
        nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
        createFlags, nullptr, 0,
        D3D11_SDK_VERSION, &scd,
        &m_swapChain, &m_device,
        &featureLevel, &m_context
    );
    
    if (FAILED(hr)) return false;
    
    // 创建渲染目标视图
    ID3D11Texture2D* pBackBuffer;
    m_swapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    m_device->CreateRenderTargetView(pBackBuffer, nullptr, &m_rtv);
    pBackBuffer->Release();
    
    // 初始化 ImGui
    m_imguiContext = ImGui::CreateContext();
    ImGui::SetCurrentContext(m_imguiContext);
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    
    ImGui_ImplWin32_Init(hwnd);
    ImGui_ImplDX11_Init(m_device, m_context);
    
    return true;
}

void D3D11Renderer::Shutdown() {
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext(m_imguiContext);
    
    if (m_rtv) m_rtv->Release();
    if (m_swapChain) m_swapChain->Release();
    if (m_context) m_context->Release();
    if (m_device) m_device->Release();
}

void D3D11Renderer::NewFrame() {
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui_ImplDX11_NewFrame();
    ImGui_ImplWin32_NewFrame();
    ImGui::NewFrame();
}

void D3D11Renderer::Render() {
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui::Render();
    
    float clear_color[4] = { 0.1f, 0.1f, 0.12f, 1.0f };
    m_context->OMSetRenderTargets(1, &m_rtv, nullptr);
    m_context->ClearRenderTargetView(m_rtv, clear_color);
    
    ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
    m_swapChain->Present(1, 0);
}

void D3D11Renderer::Resize(int width, int height) {
    if (m_rtv) {
        m_rtv->Release();
        m_rtv = nullptr;
    }
    
    m_swapChain->ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, 0);
    
    ID3D11Texture2D* pBackBuffer;
    m_swapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    m_device->CreateRenderTargetView(pBackBuffer, nullptr, &m_rtv);
    pBackBuffer->Release();
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/platform/D3D11Renderer.h src/platform/D3D11Renderer.cpp
git commit -m "feat: add D3D11Renderer with ImGui integration"
```

---

### 任务 7：拖放处理

**文件：**
- 创建：`src/platform/DragDrop.h`
- 创建：`src/platform/DragDrop.cpp`

- [ ] **步骤 1：创建 DragDrop.h**

```cpp
// src/platform/DragDrop.h
#pragma once

#include <string>
#include <vector>
#include <Windows.h>

namespace mn {

class DragDrop {
public:
    void Initialize(HWND hwnd);
    std::vector<std::wstring> GetDroppedFiles(HDROP hDrop);
};

} // namespace mn
```

- [ ] **步骤 2：创建 DragDrop.cpp**

```cpp
// src/platform/DragDrop.cpp
#include "DragDrop.h"

namespace mn {

void DragDrop::Initialize(HWND hwnd) {
    DragAcceptFiles(hwnd, TRUE);
}

std::vector<std::wstring> DragDrop::GetDroppedFiles(HDROP hDrop) {
    std::vector<std::wstring> files;
    UINT count = DragQueryFileW(hDrop, 0xFFFFFFFF, nullptr, 0);
    
    for (UINT i = 0; i < count; i++) {
        wchar_t buffer[MAX_PATH];
        DragQueryFileW(hDrop, i, buffer, MAX_PATH);
        files.push_back(buffer);
    }
    
    DragFinish(hDrop);
    return files;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/platform/DragDrop.h src/platform/DragDrop.cpp
git commit -m "feat: add DragDrop handler"
```

---

## 阶段四：核心组件

### 任务 8：JSON 存储

**文件：**
- 创建：`src/core/Storage.h`
- 创建：`src/core/Storage.cpp`

- [ ] **步骤 1：创建 Storage.h**

```cpp
// src/core/Storage.h
#pragma once

#include "Types.h"
#include <string>
#include <vector>

namespace mn {

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
    
    std::vector<std::wstring> GetItemIdsByCategory(const std::wstring& categoryId);
    
    std::wstring GetConfig(const std::string& key, const std::wstring& defaultVal = L"");
    bool SetConfig(const std::string& key, const std::wstring& value);

private:
    std::wstring m_path;
    std::vector<Category> m_categories;
    std::vector<Item> m_items;
    std::unordered_map<std::string, std::wstring> m_config;
};

} // namespace mn
```

- [ ] **步骤 2：创建 Storage.cpp**

```cpp
// src/core/Storage.cpp
#include "Storage.h"
#include "utils/StringUtils.h"
#include <nlohmann/json.hpp>
#include <fstream>

namespace mn {

using json = nlohmann::json;

bool Storage::Load(const std::wstring& path) {
    m_path = path;
    
    std::ifstream file(path);
    if (!file.is_open()) return false;
    
    try {
        json j;
        file >> j;
        
        // 加载分类
        if (j.contains("categories")) {
            for (const auto& cat : j["categories"]) {
                Category c;
                c.id = StringUtils::Utf8ToWString(cat.value("id", ""));
                c.name = StringUtils::Utf8ToWString(cat.value("name", ""));
                c.sortOrder = cat.value("sortOrder", 0);
                c.viewType = static_cast<ViewType>(cat.value("viewType", 0));
                c.iconSize = cat.value("iconSize", 48);
                m_categories.push_back(c);
            }
        }
        
        // 加载项目
        if (j.contains("items")) {
            for (const auto& item : j["items"]) {
                Item i;
                i.id = StringUtils::Utf8ToWString(item.value("id", ""));
                i.name = StringUtils::Utf8ToWString(item.value("name", ""));
                i.target = StringUtils::Utf8ToWString(item.value("target", ""));
                i.arguments = StringUtils::Utf8ToWString(item.value("arguments", ""));
                i.workingDir = StringUtils::Utf8ToWString(item.value("workingDir", ""));
                i.iconPath = StringUtils::Utf8ToWString(item.value("iconPath", ""));
                i.iconIndex = item.value("iconIndex", 0);
                i.runAsAdmin = item.value("runAsAdmin", false);
                i.runCount = item.value("runCount", 0);
                i.keywords = StringUtils::Utf8ToWString(item.value("keywords", ""));
                i.remark = StringUtils::Utf8ToWString(item.value("remark", ""));
                i.categoryId = StringUtils::Utf8ToWString(item.value("categoryId", ""));
                i.sortOrder = item.value("sortOrder", 0);
                m_items.push_back(i);
            }
        }
        
        // 加载配置
        if (j.contains("config")) {
            for (auto& [key, val] : j["config"].items()) {
                m_config[key] = StringUtils::Utf8ToWString(val.get<std::string>());
            }
        }
        
        return true;
    } catch (...) {
        return false;
    }
}

bool Storage::Save(const std::wstring& path) {
    json j;
    j["version"] = "1.0.0";
    
    // 保存分类
    for (const auto& cat : m_categories) {
        json c;
        c["id"] = StringUtils::WStringToUtf8(cat.id);
        c["name"] = StringUtils::WStringToUtf8(cat.name);
        c["sortOrder"] = cat.sortOrder;
        c["viewType"] = static_cast<int>(cat.viewType);
        c["iconSize"] = cat.iconSize;
        j["categories"].push_back(c);
    }
    
    // 保存项目
    for (const auto& item : m_items) {
        json i;
        i["id"] = StringUtils::WStringToUtf8(item.id);
        i["name"] = StringUtils::WStringToUtf8(item.name);
        i["target"] = StringUtils::WStringToUtf8(item.target);
        i["arguments"] = StringUtils::WStringToUtf8(item.arguments);
        i["workingDir"] = StringUtils::WStringToUtf8(item.workingDir);
        i["iconPath"] = StringUtils::WStringToUtf8(item.iconPath);
        i["iconIndex"] = item.iconIndex;
        i["runAsAdmin"] = item.runAsAdmin;
        i["runCount"] = item.runCount;
        i["keywords"] = StringUtils::WStringToUtf8(item.keywords);
        i["remark"] = StringUtils::WStringToUtf8(item.remark);
        i["categoryId"] = StringUtils::WStringToUtf8(item.categoryId);
        i["sortOrder"] = item.sortOrder;
        j["items"].push_back(i);
    }
    
    // 保存配置
    for (const auto& [key, val] : m_config) {
        j["config"][key] = StringUtils::WStringToUtf8(val);
    }
    
    std::ofstream file(path);
    if (!file.is_open()) return false;
    
    file << j.dump(4);
    return true;
}

std::vector<Category> Storage::GetCategories() {
    return m_categories;
}

std::vector<Item> Storage::GetItems(const std::wstring& categoryId) {
    if (categoryId.empty()) return m_items;
    
    std::vector<Item> result;
    for (const auto& item : m_items) {
        if (item.categoryId == categoryId) {
            result.push_back(item);
        }
    }
    return result;
}

Item Storage::GetItem(const std::wstring& id) {
    for (const auto& item : m_items) {
        if (item.id == id) return item;
    }
    return {};
}

bool Storage::AddCategory(const Category& category) {
    m_categories.push_back(category);
    return Save(m_path);
}

bool Storage::UpdateCategory(const Category& category) {
    for (auto& cat : m_categories) {
        if (cat.id == category.id) {
            cat = category;
            return Save(m_path);
        }
    }
    return false;
}

bool Storage::DeleteCategory(const std::wstring& id) {
    // 删除分类下的所有项目
    auto itemIds = GetItemIdsByCategory(id);
    for (const auto& itemId : itemIds) {
        DeleteItem(itemId);
    }
    
    // 删除分类
    m_categories.erase(
        std::remove_if(m_categories.begin(), m_categories.end(),
            [&](const Category& c) { return c.id == id; }),
        m_categories.end()
    );
    
    return Save(m_path);
}

bool Storage::AddItem(const Item& item) {
    m_items.push_back(item);
    return Save(m_path);
}

bool Storage::UpdateItem(const Item& item) {
    for (auto& i : m_items) {
        if (i.id == item.id) {
            i = item;
            return Save(m_path);
        }
    }
    return false;
}

bool Storage::DeleteItem(const std::wstring& id) {
    m_items.erase(
        std::remove_if(m_items.begin(), m_items.end(),
            [&](const Item& i) { return i.id == id; }),
        m_items.end()
    );
    return Save(m_path);
}

std::vector<std::wstring> Storage::GetItemIdsByCategory(const std::wstring& categoryId) {
    std::vector<std::wstring> ids;
    for (const auto& item : m_items) {
        if (item.categoryId == categoryId) {
            ids.push_back(item.id);
        }
    }
    return ids;
}

std::wstring Storage::GetConfig(const std::string& key, const std::wstring& defaultVal) {
    auto it = m_config.find(key);
    if (it != m_config.end()) return it->second;
    return defaultVal;
}

bool Storage::SetConfig(const std::string& key, const std::wstring& value) {
    m_config[key] = value;
    return Save(m_path);
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/Storage.h src/core/Storage.cpp
git commit -m "feat: add JSON Storage engine"
```

---

### 任务 9：配置管理

**文件：**
- 创建：`src/core/Config.h`
- 创建：`src/core/Config.cpp`

- [ ] **步骤 1：创建 Config.h**

```cpp
// src/core/Config.h
#pragma once

#include <string>

namespace mn {

class Storage;

class Config {
public:
    void Initialize(Storage* storage);
    
    std::wstring GetGlobalHotkey() const;
    void SetGlobalHotkey(const std::wstring& hotkey);
    
    int GetWindowX() const;
    int GetWindowY() const;
    int GetWindowWidth() const;
    int GetWindowHeight() const;
    void SetWindowPosition(int x, int y);
    void SetWindowSize(int width, int height);
    
    void Save();

private:
    Storage* m_storage = nullptr;
};

} // namespace mn
```

- [ ] **步骤 2：创建 Config.cpp**

```cpp
// src/core/Config.cpp
#include "Config.h"
#include "Storage.h"

namespace mn {

void Config::Initialize(Storage* storage) {
    m_storage = storage;
}

std::wstring Config::GetGlobalHotkey() const {
    return m_storage->GetConfig("globalHotkey", L"Ctrl+Alt+M");
}

void Config::SetGlobalHotkey(const std::wstring& hotkey) {
    m_storage->SetConfig("globalHotkey", hotkey);
}

int Config::GetWindowX() const {
    return std::stoi(m_storage->GetConfig("windowX", L"100"));
}

int Config::GetWindowY() const {
    return std::stoi(m_storage->GetConfig("windowY", L"100"));
}

int Config::GetWindowWidth() const {
    return std::stoi(m_storage->GetConfig("windowWidth", L"800"));
}

int Config::GetWindowHeight() const {
    return std::stoi(m_storage->GetConfig("windowHeight", L"600"));
}

void Config::SetWindowPosition(int x, int y) {
    m_storage->SetConfig("windowX", std::to_wstring(x));
    m_storage->SetConfig("windowY", std::to_wstring(y));
}

void Config::SetWindowSize(int width, int height) {
    m_storage->SetConfig("windowWidth", std::to_wstring(width));
    m_storage->SetConfig("windowHeight", std::to_wstring(height));
}

void Config::Save() {
    // Config changes are auto-saved via Storage::SetConfig
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/Config.h src/core/Config.cpp
git commit -m "feat: add Config manager"
```

---

### 任务 10：图标缓存

**文件：**
- 创建：`src/core/IconCache.h`
- 创建：`src/core/IconCache.cpp`

- [ ] **步骤 1：创建 IconCache.h**

```cpp
// src/core/IconCache.h
#pragma once

#include "Types.h"
#include <string>
#include <unordered_map>
#include <Windows.h>

namespace mn {

class IconCache {
public:
    void Initialize(const std::wstring& cacheDir);
    
    std::wstring GetIconPath(const Item& item);
    void RefreshIcon(const Item& item);
    void DeleteCache(const std::wstring& itemId);

private:
    HICON ExtractIconFromTarget(const std::wstring& target, int iconIndex);
    bool SaveIconToFile(HICON hIcon, const std::wstring& path);
    
    std::wstring m_cacheDir;
    std::unordered_map<std::wstring, std::wstring> m_cache;
};

} // namespace mn
```

- [ ] **步骤 2：创建 IconCache.cpp**

```cpp
// src/core/IconCache.cpp
#include "IconCache.h"
#include "utils/PathUtils.h"
#include <ShellApi.h>
#include <olectl.h>

namespace mn {

void IconCache::Initialize(const std::wstring& cacheDir) {
    m_cacheDir = cacheDir;
    PathUtils::EnsureDirectory(cacheDir);
}

std::wstring IconCache::GetIconPath(const Item& item) {
    // 如果已有缓存路径，直接返回
    std::wstring cachePath = m_cacheDir + L"\\" + item.id + L".png";
    if (PathUtils::Exists(cachePath)) {
        return cachePath;
    }
    
    // 提取并缓存图标
    RefreshIcon(item);
    return cachePath;
}

void IconCache::RefreshIcon(const Item& item) {
    std::wstring cachePath = m_cacheDir + L"\\" + item.id + L".png";
    
    HICON hIcon = ExtractIconFromTarget(item.target, item.iconIndex);
    if (hIcon) {
        SaveIconToFile(hIcon, cachePath);
        DestroyIcon(hIcon);
    }
}

void IconCache::DeleteCache(const std::wstring& itemId) {
    std::wstring cachePath = m_cacheDir + L"\\" + itemId + L".png";
    if (PathUtils::Exists(cachePath)) {
        DeleteFileW(cachePath.c_str());
    }
}

HICON IconCache::ExtractIconFromTarget(const std::wstring& target, int iconIndex) {
    HICON hIcon = nullptr;
    
    // 使用 SHGetFileInfo 提取图标
    SHFILEINFOW sfi = {};
    DWORD_PTR result = SHGetFileInfoW(
        target.c_str(),
        0,
        &sfi,
        sizeof(sfi),
        SHGFI_ICON | SHGFI_LARGEICON
    );
    
    if (result && sfi.hIcon) {
        hIcon = sfi.hIcon;
    }
    
    // 如果失败，尝试 ExtractIcon
    if (!hIcon) {
        hIcon = ExtractIconW(GetModuleHandle(nullptr), target.c_str(), iconIndex);
    }
    
    return hIcon;
}

bool IconCache::SaveIconToFile(HICON hIcon, const std::wstring& path) {
    // 简化实现：使用 GDI+ 保存图标
    // 这里使用一个简化的方法，实际项目中可能需要完整的 GDI+ 实现
    
    ICONINFO iconInfo;
    if (!GetIconInfo(hIcon, &iconInfo)) return false;
    
    BITMAP bm;
    GetObject(iconInfo.hbmColor, sizeof(BITMAP), &bm);
    
    // 创建位图文件头
    BITMAPFILEHEADER bfh = {};
    bfh.bfType = 0x4D42; // "BM"
    bfh.bfOffBits = sizeof(BITMAPFILEHEADER) + sizeof(BITMAPINFOHEADER);
    bfh.bfSize = bfh.bfOffBits + bm.bmWidthBytes * bm.bmHeight;
    
    BITMAPINFOHEADER bih = {};
    bih.biSize = sizeof(BITMAPINFOHEADER);
    bih.biWidth = bm.bmWidth;
    bih.biHeight = bm.bmHeight;
    bih.biPlanes = 1;
    bih.biBitCount = 32;
    bih.biCompression = BI_RGB;
    
    // 保存到文件
    HANDLE hFile = CreateFileW(path.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (hFile == INVALID_HANDLE_VALUE) {
        DeleteObject(iconInfo.hbmColor);
        DeleteObject(iconInfo.hbmMask);
        return false;
    }
    
    DWORD written;
    WriteFile(hFile, &bfh, sizeof(bfh), &written, nullptr);
    WriteFile(hFile, &bih, sizeof(bih), &written, nullptr);
    
    // 写入像素数据
    HDC hdc = GetDC(nullptr);
    HDC hdcMem = CreateCompatibleDC(hdc);
    HBITMAP hbmOld = (HBITMAP)SelectObject(hdcMem, iconInfo.hbmColor);
    
    std::vector<BYTE> pixels(bm.bmWidthBytes * bm.bmHeight);
    GetDIBits(hdcMem, iconInfo.hbmColor, 0, bm.bmHeight, pixels.data(), (BITMAPINFO*)&bih, DIB_RGB_COLORS);
    WriteFile(hFile, pixels.data(), pixels.size(), &written, nullptr);
    
    SelectObject(hdcMem, hbmOld);
    DeleteDC(hdcMem);
    ReleaseDC(nullptr, hdc);
    CloseHandle(hFile);
    
    DeleteObject(iconInfo.hbmColor);
    DeleteObject(iconInfo.hbmMask);
    
    return true;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/IconCache.h src/core/IconCache.cpp
git commit -m "feat: add IconCache system"
```

---

### 任务 11：项目启动器

**文件：**
- 创建：`src/core/Runner.h`
- 创建：`src/core/Runner.cpp`

- [ ] **步骤 1：创建 Runner.h**

```cpp
// src/core/Runner.h
#pragma once

#include "Types.h"

namespace mn {

class Runner {
public:
    RunResult Run(const Item& item);
    RunResult RunAsAdmin(const Item& item);

private:
    std::wstring GetErrorMessage(DWORD errorCode);
    RunError MapError(DWORD errorCode);
};

} // namespace mn
```

- [ ] **步骤 2：创建 Runner.cpp**

```cpp
// src/core/Runner.cpp
#include "Runner.h"
#include "utils/PathUtils.h"
#include <ShellApi.h>

namespace mn {

RunResult Runner::Run(const Item& item) {
    RunResult result;
    
    // 检查文件是否存在
    if (!PathUtils::Exists(item.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到";
        return result;
    }
    
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpFile = item.target.c_str();
    sei.lpParameters = item.arguments.empty() ? nullptr : item.arguments.c_str();
    sei.lpDirectory = item.workingDir.empty() ? nullptr : item.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        DWORD err = GetLastError();
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err);
        return result;
    }
    
    result.success = true;
    return result;
}

RunResult Runner::RunAsAdmin(const Item& item) {
    RunResult result;
    
    if (!PathUtils::Exists(item.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到";
        return result;
    }
    
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = L"runas";
    sei.lpFile = item.target.c_str();
    sei.lpParameters = item.arguments.empty() ? nullptr : item.arguments.c_str();
    sei.lpDirectory = item.workingDir.empty() ? nullptr : item.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        DWORD err = GetLastError();
        if (err == ERROR_CANCELLED) {
            // 用户取消了 UAC 提示
            result.success = true;
            return result;
        }
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err);
        return result;
    }
    
    result.success = true;
    return result;
}

RunError Runner::MapError(DWORD errorCode) {
    switch (errorCode) {
        case ERROR_FILE_NOT_FOUND: return RunError::FileNotFound;
        case ERROR_PATH_NOT_FOUND: return RunError::PathNotFound;
        case ERROR_ACCESS_DENIED: return RunError::AccessDenied;
        case ERROR_NOT_ENOUGH_MEMORY: return RunError::OutOfMemory;
        case ERROR_DLL_NOT_FOUND: return RunError::DllNotFound;
        default: return RunError::Unknown;
    }
}

std::wstring Runner::GetErrorMessage(DWORD errorCode) {
    switch (errorCode) {
        case ERROR_FILE_NOT_FOUND: return L"文件未找到";
        case ERROR_PATH_NOT_FOUND: return L"路径未找到";
        case ERROR_ACCESS_DENIED: return L"拒绝访问";
        case ERROR_NOT_ENOUGH_MEMORY: return L"内存不足";
        case ERROR_DLL_NOT_FOUND: return L"动态链接库未找到";
        default: return L"未知错误";
    }
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/Runner.h src/core/Runner.cpp
git commit -m "feat: add Runner with error handling"
```

---

### 任务 12：快捷键管理器

**文件：**
- 创建：`src/core/HotkeyManager.h`
- 创建：`src/core/HotkeyManager.cpp`

- [ ] **步骤 1：创建 HotkeyManager.h**

```cpp
// src/core/HotkeyManager.h
#pragma once

#include <functional>
#include <Windows.h>

namespace mn {

class HotkeyManager {
public:
    bool RegisterGlobalHotkey(int id, UINT modifiers, UINT vk);
    bool UnregisterGlobalHotkey(int id);
    void ProcessHotkey(WPARAM wParam);
    
    bool ParseHotkeyString(const std::wstring& hotkey, UINT& modifiers, UINT& vk);
    
    using HotkeyCallback = std::function<void(int id)>;
    void SetCallback(HotkeyCallback callback);

private:
    HotkeyCallback m_callback;
};

} // namespace mn
```

- [ ] **步骤 2：创建 HotkeyManager.cpp**

```cpp
// src/core/HotkeyManager.cpp
#include "HotkeyManager.h"
#include "utils/StringUtils.h"

namespace mn {

bool HotkeyManager::RegisterGlobalHotkey(int id, UINT modifiers, UINT vk) {
    return RegisterHotKey(nullptr, id, modifiers, vk) != FALSE;
}

bool HotkeyManager::UnregisterGlobalHotkey(int id) {
    return UnregisterHotKey(nullptr, id) != FALSE;
}

void HotkeyManager::ProcessHotkey(WPARAM wParam) {
    if (m_callback) {
        m_callback(static_cast<int>(wParam));
    }
}

bool HotkeyManager::ParseHotkeyString(const std::wstring& hotkey, UINT& modifiers, UINT& vk) {
    modifiers = 0;
    vk = 0;
    
    auto parts = StringUtils::Split(hotkey, L'+');
    for (size_t i = 0; i < parts.size(); i++) {
        auto part = StringUtils::Trim(parts[i]);
        
        if (part == L"Ctrl" || part == L"ctrl") {
            modifiers |= MOD_CONTROL;
        } else if (part == L"Alt" || part == L"alt") {
            modifiers |= MOD_ALT;
        } else if (part == L"Shift" || part == L"shift") {
            modifiers |= MOD_SHIFT;
        } else if (part == L"Win" || part == L"win") {
            modifiers |= MOD_WIN;
        } else if (i == parts.size() - 1) {
            // 最后一个部分是按键
            if (part.size() == 1) {
                vk = toupper(part[0]);
            } else if (part == L"F1") vk = VK_F1;
            else if (part == L"F2") vk = VK_F2;
            else if (part == L"F3") vk = VK_F3;
            else if (part == L"F4") vk = VK_F4;
            else if (part == L"F5") vk = VK_F5;
            else if (part == L"F6") vk = VK_F6;
            else if (part == L"F7") vk = VK_F7;
            else if (part == L"F8") vk = VK_F8;
            else if (part == L"F9") vk = VK_F9;
            else if (part == L"F10") vk = VK_F10;
            else if (part == L"F11") vk = VK_F11;
            else if (part == L"F12") vk = VK_F12;
            else return false;
        }
    }
    
    return modifiers != 0 && vk != 0;
}

void HotkeyManager::SetCallback(HotkeyCallback callback) {
    m_callback = callback;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/HotkeyManager.h src/core/HotkeyManager.cpp
git commit -m "feat: add HotkeyManager"
```

---

### 任务 13：项目管理器

**文件：**
- 创建：`src/core/ItemManager.h`
- 创建：`src/core/ItemManager.cpp`

- [ ] **步骤 1：创建 ItemManager.h**

```cpp
// src/core/ItemManager.h
#pragma once

#include "Types.h"
#include <vector>
#include <unordered_map>

namespace mn {

class Storage;
class IconCache;

class ItemManager {
public:
    void Initialize(Storage* storage, IconCache* iconCache);
    
    std::vector<Category>& GetCategories();
    Category* GetCategory(const std::wstring& id);
    void AddCategory(Category category);
    void DeleteCategory(const std::wstring& id);
    
    std::vector<Item>& GetItems(const std::wstring& categoryId);
    Item* GetItem(const std::wstring& id);
    void AddItem(Item item);
    void MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId);
    void DeleteItem(const std::wstring& id);
    
    void RefreshItemIcon(const std::wstring& itemId);
    void HandleDrop(const std::vector<std::wstring>& files, const std::wstring& categoryId);

private:
    bool ResolveShortcut(const std::wstring& lnkPath, std::wstring& target, std::wstring& workingDir);
    
    Storage* m_storage = nullptr;
    IconCache* m_iconCache = nullptr;
    std::vector<Category> m_categories;
    std::unordered_map<std::wstring, std::vector<Item>> m_itemsByCategory;
    std::vector<Item> m_allItems;
};

} // namespace mn
```

- [ ] **步骤 2：创建 ItemManager.cpp**

```cpp
// src/core/ItemManager.cpp
#include "ItemManager.h"
#include "Storage.h"
#include "IconCache.h"
#include "utils/PathUtils.h"
#include <ShObjIdl.h>
#include <ShlObj.h>

namespace mn {

void ItemManager::Initialize(Storage* storage, IconCache* iconCache) {
    m_storage = storage;
    m_iconCache = iconCache;
    
    m_categories = storage->GetCategories();
    m_allItems = storage->GetItems();
    
    // 按分类组织项目
    for (const auto& item : m_allItems) {
        m_itemsByCategory[item.categoryId].push_back(item);
    }
    
    // 确保至少有一个分类
    if (m_categories.empty()) {
        Category defaultCat;
        defaultCat.id = GenerateId(L"cat");
        defaultCat.name = L"默认";
        AddCategory(defaultCat);
    }
}

std::vector<Category>& ItemManager::GetCategories() {
    return m_categories;
}

Category* ItemManager::GetCategory(const std::wstring& id) {
    for (auto& cat : m_categories) {
        if (cat.id == id) return &cat;
    }
    return nullptr;
}

void ItemManager::AddCategory(Category category) {
    if (category.id.empty()) {
        category.id = GenerateId(L"cat");
    }
    m_categories.push_back(category);
    m_storage->AddCategory(category);
}

void ItemManager::DeleteCategory(const std::wstring& id) {
    // 删除分类下的所有项目
    auto& items = m_itemsByCategory[id];
    for (const auto& item : items) {
        m_iconCache->DeleteCache(item.id);
    }
    m_itemsByCategory.erase(id);
    
    // 从存储中删除
    m_storage->DeleteCategory(id);
    
    // 从内存中删除
    m_categories.erase(
        std::remove_if(m_categories.begin(), m_categories.end(),
            [&](const Category& c) { return c.id == id; }),
        m_categories.end()
    );
}

std::vector<Item>& ItemManager::GetItems(const std::wstring& categoryId) {
    return m_itemsByCategory[categoryId];
}

Item* ItemManager::GetItem(const std::wstring& id) {
    for (auto& [catId, items] : m_itemsByCategory) {
        for (auto& item : items) {
            if (item.id == id) return &item;
        }
    }
    return nullptr;
}

void ItemManager::AddItem(Item item) {
    if (item.id.empty()) {
        item.id = GenerateId(L"item");
    }
    
    // 提取图标
    m_iconCache->GetIconPath(item);
    
    m_itemsByCategory[item.categoryId].push_back(item);
    m_allItems.push_back(item);
    m_storage->AddItem(item);
}

void ItemManager::MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId) {
    Item* item = GetItem(itemId);
    if (item) {
        std::wstring oldCatId = item->categoryId;
        item->categoryId = targetCategoryId;
        
        // 更新内存
        auto& oldItems = m_itemsByCategory[oldCatId];
        oldItems.erase(
            std::remove_if(oldItems.begin(), oldItems.end(),
                [&](const Item& i) { return i.id == itemId; }),
            oldItems.end()
        );
        m_itemsByCategory[targetCategoryId].push_back(*item);
        
        // 更新存储
        m_storage->UpdateItem(*item);
    }
}

void ItemManager::DeleteItem(const std::wstring& id) {
    Item* item = GetItem(id);
    if (item) {
        std::wstring catId = item->categoryId;
        
        // 删除图标缓存
        m_iconCache->DeleteCache(id);
        
        // 从内存删除
        auto& items = m_itemsByCategory[catId];
        items.erase(
            std::remove_if(items.begin(), items.end(),
                [&](const Item& i) { return i.id == id; }),
            items.end()
        );
        
        m_allItems.erase(
            std::remove_if(m_allItems.begin(), m_allItems.end(),
                [&](const Item& i) { return i.id == id; }),
            m_allItems.end()
        );
        
        // 从存储删除
        m_storage->DeleteItem(id);
    }
}

void ItemManager::RefreshItemIcon(const std::wstring& itemId) {
    Item* item = GetItem(itemId);
    if (item && m_iconCache) {
        m_iconCache->RefreshIcon(*item);
    }
}

void ItemManager::HandleDrop(const std::vector<std::wstring>& files, const std::wstring& categoryId) {
    for (const auto& file : files) {
        Item item;
        item.id = GenerateId(L"item");
        item.categoryId = categoryId;
        item.name = PathUtils::GetFileName(file);
        
        // 处理快捷方式
        if (file.size() > 4 && file.substr(file.size() - 4) == L".lnk") {
            std::wstring target, workingDir;
            if (ResolveShortcut(file, target, workingDir)) {
                item.target = target;
                item.workingDir = workingDir;
                item.name = PathUtils::GetFileName(target);
                // 移除扩展名
                size_t dotPos = item.name.rfind(L'.');
                if (dotPos != std::wstring::npos) {
                    item.name = item.name.substr(0, dotPos);
                }
            } else {
                item.target = file;
            }
        } else {
            item.target = file;
            item.workingDir = PathUtils::GetParentDir(file);
        }
        
        AddItem(item);
    }
}

bool ItemManager::ResolveShortcut(const std::wstring& lnkPath, std::wstring& target, std::wstring& workingDir) {
    HRESULT hr = CoInitialize(nullptr);
    if (FAILED(hr)) return false;
    
    IShellLinkW* pShellLink = nullptr;
    hr = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER, IID_IShellLinkW, (void**)&pShellLink);
    
    if (SUCCEEDED(hr)) {
        IPersistFile* pPersistFile = nullptr;
        hr = pShellLink->QueryInterface(IID_IPersistFile, (void**)&pPersistFile);
        
        if (SUCCEEDED(hr)) {
            hr = pPersistFile->Load(lnkPath.c_str(), STGM_READ);
            
            if (SUCCEEDED(hr)) {
                wchar_t szTarget[MAX_PATH];
                wchar_t szWorkingDir[MAX_PATH];
                
                hr = pShellLink->GetPath(szTarget, MAX_PATH, nullptr, 0);
                if (SUCCEEDED(hr)) {
                    target = szTarget;
                }
                
                hr = pShellLink->GetWorkingDirectory(szWorkingDir, MAX_PATH);
                if (SUCCEEDED(hr)) {
                    workingDir = szWorkingDir;
                }
            }
            
            pPersistFile->Release();
        }
        
        pShellLink->Release();
    }
    
    CoUninitialize();
    return SUCCEEDED(hr) && !target.empty();
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/core/ItemManager.h src/core/ItemManager.cpp
git commit -m "feat: add ItemManager with drag-drop support"
```

---

## 阶段五：UI 层

### 任务 14：分类标签组件

**文件：**
- 创建：`src/ui/widgets/CategoryTab.h`
- 创建：`src/ui/widgets/CategoryTab.cpp`

- [ ] **步骤 1：创建 CategoryTab.h**

```cpp
// src/ui/widgets/CategoryTab.h
#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>

namespace mn {

class CategoryTab {
public:
    void SetCategories(std::vector<Category>* categories);
    void Render();
    
    void OnCategoryChanged(std::function<void(const std::wstring& id)> callback);
    
    void SetCurrentCategory(const std::wstring& id);
    std::wstring GetCurrentCategory() const;

private:
    std::vector<Category>* m_categories = nullptr;
    std::wstring m_currentId;
    std::function<void(const std::wstring&)> m_onChanged;
};

} // namespace mn
```

- [ ] **步骤 2：创建 CategoryTab.cpp**

```cpp
// src/ui/widgets/CategoryTab.cpp
#include "CategoryTab.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void CategoryTab::SetCategories(std::vector<Category>* categories) {
    m_categories = categories;
    if (categories && !categories->empty() && m_currentId.empty()) {
        m_currentId = (*categories)[0].id;
    }
}

void CategoryTab::Render() {
    if (!m_categories) return;
    
    ImGui::BeginChild("Categories", ImVec2(150, 0), true);
    
    for (const auto& cat : *m_categories) {
        bool selected = (cat.id == m_currentId);
        std::string name = StringUtils::WStringToUtf8(cat.name);
        
        if (ImGui::Selectable(name.c_str(), selected)) {
            m_currentId = cat.id;
            if (m_onChanged) {
                m_onChanged(cat.id);
            }
        }
    }
    
    ImGui::EndChild();
}

void CategoryTab::OnCategoryChanged(std::function<void(const std::wstring&)> callback) {
    m_onChanged = callback;
}

void CategoryTab::SetCurrentCategory(const std::wstring& id) {
    m_currentId = id;
}

std::wstring CategoryTab::GetCurrentCategory() const {
    return m_currentId;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/ui/widgets/CategoryTab.h src/ui/widgets/CategoryTab.cpp
git commit -m "feat: add CategoryTab widget"
```

---

### 任务 15：项目网格组件

**文件：**
- 创建：`src/ui/widgets/ItemGrid.h`
- 创建：`src/ui/widgets/ItemGrid.cpp`

- [ ] **步骤 1：创建 ItemGrid.h**

```cpp
// src/ui/widgets/ItemGrid.h
#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>

namespace mn {

class ItemGrid {
public:
    void SetItems(std::vector<Item>* items);
    void Render();
    
    void OnItemDoubleClicked(std::function<void(const Item&)> callback);
    void OnItemRightClicked(std::function<void(const Item&)> callback);

private:
    std::vector<Item>* m_items = nullptr;
    int m_selectedIndex = -1;
    std::function<void(const Item&)> m_onDoubleClick;
    std::function<void(const Item&)> m_onRightClick;
};

} // namespace mn
```

- [ ] **步骤 2：创建 ItemGrid.cpp**

```cpp
// src/ui/widgets/ItemGrid.cpp
#include "ItemGrid.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void ItemGrid::SetItems(std::vector<Item>* items) {
    m_items = items;
    m_selectedIndex = -1;
}

void ItemGrid::Render() {
    if (!m_items) return;
    
    ImGui::BeginChild("Items", ImVec2(0, 0), true);
    
    int index = 0;
    for (const auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        // 图标按钮
        if (ImGui::Button(name.c_str(), ImVec2(80, 80))) {
            m_selectedIndex = index;
        }
        
        // 双击检测
        if (ImGui::IsItemHovered() && ImGui::IsMouseDoubleClicked(0)) {
            if (m_onDoubleClick) {
                m_onDoubleClick(item);
            }
        }
        
        // 右键菜单
        if (ImGui::BeginPopupContextItem()) {
            if (ImGui::MenuItem("运行")) {
                if (m_onDoubleClick) {
                    m_onDoubleClick(item);
                }
            }
            if (ImGui::MenuItem("以管理员运行")) {
                // TODO: 实现管理员运行
            }
            if (ImGui::MenuItem("编辑")) {
                // TODO: 打开编辑对话框
            }
            if (ImGui::MenuItem("删除")) {
                // TODO: 删除项目
            }
            ImGui::EndPopup();
        }
        
        // 同一行排列，每行 4 个
        if ((index + 1) % 4 != 0) {
            ImGui::SameLine();
        }
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
}

void ItemGrid::OnItemDoubleClicked(std::function<void(const Item&)> callback) {
    m_onDoubleClick = callback;
}

void ItemGrid::OnItemRightClicked(std::function<void(const Item&)> callback) {
    m_onRightClick = callback;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/ui/widgets/ItemGrid.h src/ui/widgets/ItemGrid.cpp
git commit -m "feat: add ItemGrid widget"
```

---

### 任务 16：编辑对话框

**文件：**
- 创建：`src/ui/dialogs/EditDialog.h`
- 创建：`src/ui/dialogs/EditDialog.cpp`

- [ ] **步骤 1：创建 EditDialog.h**

```cpp
// src/ui/dialogs/EditDialog.h
#pragma once

#include "core/Types.h"
#include <functional>
#include <string>

namespace mn {

class EditDialog {
public:
    void Show(Item* item);
    void Hide();
    bool IsVisible() const;
    void Render();
    
    void OnSave(std::function<void(const Item&)> callback);

private:
    void LoadFromItem();
    void SaveToItem();
    
    Item* m_item = nullptr;
    bool m_visible = false;
    
    std::string m_nameBuf;
    std::string m_targetBuf;
    std::string m_argsBuf;
    std::string m_workingDirBuf;
    bool m_runAsAdmin = false;
    
    std::function<void(const Item&)> m_onSave;
};

} // namespace mn
```

- [ ] **步骤 2：创建 EditDialog.cpp**

```cpp
// src/ui/dialogs/EditDialog.cpp
#include "EditDialog.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void EditDialog::Show(Item* item) {
    m_item = item;
    m_visible = true;
    LoadFromItem();
}

void EditDialog::Hide() {
    m_visible = false;
    m_item = nullptr;
}

bool EditDialog::IsVisible() const {
    return m_visible;
}

void EditDialog::Render() {
    if (!m_visible || !m_item) return;
    
    ImGui::OpenPopup("编辑项目");
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(400, 300));
    
    if (ImGui::BeginPopupModal("编辑项目", nullptr, ImGuiWindowFlags_NoResize)) {
        // 名称
        char nameBuf[256];
        strncpy(nameBuf, m_nameBuf.c_str(), sizeof(nameBuf) - 1);
        if (ImGui::InputText("名称", nameBuf, sizeof(nameBuf))) {
            m_nameBuf = nameBuf;
        }
        
        // 目标
        char targetBuf[1024];
        strncpy(targetBuf, m_targetBuf.c_str(), sizeof(targetBuf) - 1);
        if (ImGui::InputText("目标", targetBuf, sizeof(targetBuf))) {
            m_targetBuf = targetBuf;
        }
        
        // 参数
        char argsBuf[512];
        strncpy(argsBuf, m_argsBuf.c_str(), sizeof(argsBuf) - 1);
        if (ImGui::InputText("参数", argsBuf, sizeof(argsBuf))) {
            m_argsBuf = argsBuf;
        }
        
        // 工作目录
        char workDirBuf[1024];
        strncpy(workDirBuf, m_workingDirBuf.c_str(), sizeof(workDirBuf) - 1);
        if (ImGui::InputText("工作目录", workDirBuf, sizeof(workDirBuf))) {
            m_workingDirBuf = workDirBuf;
        }
        
        // 管理员权限
        ImGui::Checkbox("以管理员运行", &m_runAsAdmin);
        
        ImGui::Separator();
        
        // 按钮
        if (ImGui::Button("保存", ImVec2(120, 0))) {
            SaveToItem();
            Hide();
            if (m_onSave) {
                m_onSave(*m_item);
            }
        }
        ImGui::SameLine();
        if (ImGui::Button("取消", ImVec2(120, 0))) {
            Hide();
        }
        
        ImGui::EndPopup();
    }
}

void EditDialog::LoadFromItem() {
    if (!m_item) return;
    
    m_nameBuf = StringUtils::WStringToUtf8(m_item->name);
    m_targetBuf = StringUtils::WStringToUtf8(m_item->target);
    m_argsBuf = StringUtils::WStringToUtf8(m_item->arguments);
    m_workingDirBuf = StringUtils::WStringToUtf8(m_item->workingDir);
    m_runAsAdmin = m_item->runAsAdmin;
}

void EditDialog::SaveToItem() {
    if (!m_item) return;
    
    m_item->name = StringUtils::Utf8ToWString(m_nameBuf);
    m_item->target = StringUtils::Utf8ToWString(m_targetBuf);
    m_item->arguments = StringUtils::Utf8ToWString(m_argsBuf);
    m_item->workingDir = StringUtils::Utf8ToWString(m_workingDirBuf);
    m_item->runAsAdmin = m_runAsAdmin;
}

void EditDialog::OnSave(std::function<void(const Item&)> callback) {
    m_onSave = callback;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/ui/dialogs/EditDialog.h src/ui/dialogs/EditDialog.cpp
git commit -m "feat: add EditDialog"
```

---

### 任务 17：主窗口

**文件：**
- 创建：`src/ui/MainWindow.h`
- 创建：`src/ui/MainWindow.cpp`

- [ ] **步骤 1：创建 MainWindow.h**

```cpp
// src/ui/MainWindow.h
#pragma once

#include "widgets/CategoryTab.h"
#include "widgets/ItemGrid.h"
#include "dialogs/EditDialog.h"
#include "core/Types.h"
#include <memory>

namespace mn {

class ItemManager;
class Config;
class Runner;

class MainWindow {
public:
    void Initialize(ItemManager* itemManager, Config* config, Runner* runner);
    void Render();
    
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    
    void SetCurrentCategory(const std::wstring& categoryId);

private:
    void RenderMenuBar();
    void RenderContextMenu();
    void ShowError(const std::wstring& message);
    
    ItemManager* m_itemManager = nullptr;
    Config* m_config = nullptr;
    Runner* m_runner = nullptr;
    
    CategoryTab m_categoryTab;
    ItemGrid m_itemGrid;
    EditDialog m_editDialog;
    
    std::wstring m_currentCategoryId;
    bool m_visible = false;
    int m_selectedItemIndex = -1;
    
    std::wstring m_errorMessage;
    bool m_showError = false;
};

} // namespace mn
```

- [ ] **步骤 2：创建 MainWindow.cpp**

```cpp
// src/ui/MainWindow.cpp
#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    // 设置分类变更回调
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
    });
    
    // 设置双击回调
    m_itemGrid.OnItemDoubleClicked([this](const Item& item) {
        RunResult result = m_runner->Run(item);
        if (!result.success) {
            ShowError(result.errorMessage);
        }
    });
    
    // 设置保存回调
    m_editDialog.OnSave([this](const Item& item) {
        // 刷新显示
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    });
    
    // 设置默认分类
    auto& categories = itemManager->GetCategories();
    if (!categories.empty()) {
        m_currentCategoryId = categories[0].id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    }
}

void MainWindow::Render() {
    if (!m_visible) return;
    
    RenderMenuBar();
    
    // 主布局
    ImGui::BeginChild("MainLayout", ImVec2(0, 0), false);
    
    // 左侧分类
    ImGui::BeginChild("Left", ImVec2(150, 0), true);
    m_categoryTab.Render();
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    // 右侧项目
    ImGui::BeginChild("Right", ImVec2(0, 0), true);
    m_itemGrid.Render();
    ImGui::EndChild();
    
    ImGui::EndChild();
    
    // 编辑对话框
    m_editDialog.Render();
    
    // 错误提示
    if (m_showError) {
        ImGui::OpenPopup("错误");
        if (ImGui::BeginPopupModal("错误", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button("确定")) {
                m_showError = false;
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        }
    }
}

void MainWindow::RenderMenuBar() {
    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu("文件")) {
            if (ImGui::MenuItem("新建分类")) {
                Category cat;
                cat.id = GenerateId(L"cat");
                cat.name = L"新分类";
                m_itemManager->AddCategory(cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu("设置")) {
            // TODO: 设置对话框
            ImGui::EndMenu();
        }
        ImGui::EndMenuBar();
    }
}

void MainWindow::Show() {
    m_visible = true;
}

void MainWindow::Hide() {
    m_visible = false;
}

void MainWindow::Toggle() {
    m_visible = !m_visible;
}

bool MainWindow::IsVisible() const {
    return m_visible;
}

void MainWindow::SetCurrentCategory(const std::wstring& categoryId) {
    m_currentCategoryId = categoryId;
    m_categoryTab.SetCurrentCategory(categoryId);
    m_itemGrid.SetItems(&m_itemManager->GetItems(categoryId));
}

void MainWindow::ShowError(const std::wstring& message) {
    m_errorMessage = message;
    m_showError = true;
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/ui/MainWindow.h src/ui/MainWindow.cpp
git commit -m "feat: add MainWindow UI"
```

---

## 阶段六：应用入口

### 任务 18：应用主类

**文件：**
- 创建：`src/app/App.h`
- 创建：`src/app/App.cpp`

- [ ] **步骤 1：创建 App.h**

```cpp
// src/app/App.h
#pragma once

#include <memory>
#include <Windows.h>

namespace mn {

class Storage;
class IconCache;
class ItemManager;
class Config;
class HotkeyManager;
class Runner;
class Window;
class D3D11Renderer;
class MainWindow;

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
    void SaveWindowPosition();
    
    static App* s_instance;
    
    std::unique_ptr<Storage> m_storage;
    std::unique_ptr<IconCache> m_iconCache;
    std::unique_ptr<ItemManager> m_itemManager;
    std::unique_ptr<Config> m_config;
    std::unique_ptr<HotkeyManager> m_hotkeyManager;
    std::unique_ptr<Runner> m_runner;
    std::unique_ptr<Window> m_window;
    std::unique_ptr<D3D11Renderer> m_renderer;
    std::unique_ptr<MainWindow> m_mainWindow;
};

} // namespace mn
```

- [ ] **步骤 2：创建 App.cpp**

```cpp
// src/app/App.cpp
#include "App.h"
#include "core/Storage.h"
#include "core/IconCache.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/HotkeyManager.h"
#include "core/Runner.h"
#include "platform/Window.h"
#include "platform/D3D11Renderer.h"
#include "ui/MainWindow.h"
#include "utils/PathUtils.h"
#include <imgui.h>

namespace mn {

App* App::s_instance = nullptr;

App* App::Get() {
    return s_instance;
}

bool App::Initialize(HINSTANCE hInstance) {
    s_instance = this;
    
    // 创建组件
    m_storage = std::make_unique<Storage>();
    m_iconCache = std::make_unique<IconCache>();
    m_itemManager = std::make_unique<ItemManager>();
    m_config = std::make_unique<Config>();
    m_hotkeyManager = std::make_unique<HotkeyManager>();
    m_runner = std::make_unique<Runner>();
    m_window = std::make_unique<Window>();
    m_renderer = std::make_unique<D3D11Renderer>();
    m_mainWindow = std::make_unique<MainWindow>();
    
    // 初始化数据目录
    std::wstring appDataPath = PathUtils::GetAppDataPath();
    PathUtils::EnsureDirectory(appDataPath);
    
    std::wstring iconsPath = appDataPath + L"\\icons";
    PathUtils::EnsureDirectory(iconsPath);
    
    // 初始化图标缓存
    m_iconCache->Initialize(iconsPath);
    
    // 加载或创建配置文件
    std::wstring configPath = appDataPath + L"\\config.json";
    if (PathUtils::Exists(configPath)) {
        m_storage->Load(configPath);
    }
    
    // 初始化配置
    m_config->Initialize(m_storage.get());
    
    // 初始化项目管理器
    m_itemManager->Initialize(m_storage.get(), m_iconCache.get());
    
    // 创建窗口
    int x = m_config->GetWindowX();
    int y = m_config->GetWindowY();
    int width = m_config->GetWindowWidth();
    int height = m_config->GetWindowHeight();
    
    if (!m_window->Create(L"Maye Nano", width, height, x, y)) {
        return false;
    }
    
    // 初始化渲染器
    if (!m_renderer->Initialize(m_window->GetHandle(), width, height)) {
        return false;
    }
    
    // 启用拖放
    m_window->EnableDragDrop();
    
    // 设置拖放回调
    m_window->OnDropFiles([this](const std::vector<std::wstring>& files) {
        m_itemManager->HandleDrop(files, m_mainWindow->IsVisible() ? 
            m_itemManager->GetCategories()[0].id : L"");
    });
    
    // 设置窗口大小回调
    m_window->OnResize([this](int w, int h) {
        m_renderer->Resize(w, h);
        m_config->SetWindowSize(w, h);
    });
    
    // 设置窗口移动回调
    m_window->OnMove([this](int x, int y) {
        m_config->SetWindowPosition(x, y);
    });
    
    // 设置快捷键回调
    m_hotkeyManager->SetCallback([this](int id) {
        HandleHotkey(id);
    });
    
    // 注册全局快捷键
    std::wstring hotkey = m_config->GetGlobalHotkey();
    UINT modifiers, vk;
    if (m_hotkeyManager->ParseHotkeyString(hotkey, modifiers, vk)) {
        m_hotkeyManager->RegisterGlobalHotkey(1, modifiers, vk);
    }
    
    // 初始化 UI
    m_mainWindow->Initialize(m_itemManager.get(), m_config.get(), m_runner.get());
    
    return true;
}

int App::Run() {
    MSG msg = {};
    
    while (msg.message != WM_QUIT) {
        if (PeekMessage(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
            
            // 处理快捷键
            if (msg.message == WM_HOTKEY) {
                m_hotkeyManager->ProcessHotkey(msg.wParam);
            }
        } else {
            // 渲染
            m_renderer->NewFrame();
            
            if (m_mainWindow->IsVisible()) {
                ImGui::SetNextWindowPos(ImVec2(0, 0));
                ImGui::SetNextWindowSize(ImGui::GetIO().DisplaySize);
                ImGui::Begin("Main", nullptr, 
                    ImGuiWindowFlags_NoTitleBar | 
                    ImGuiWindowFlags_NoResize | 
                    ImGuiWindowFlags_NoMove |
                    ImGuiWindowFlags_MenuBar);
                
                m_mainWindow->Render();
                
                ImGui::End();
            }
            
            m_renderer->Render();
        }
    }
    
    return static_cast<int>(msg.wParam);
}

void App::Shutdown() {
    SaveWindowPosition();
    
    m_renderer->Shutdown();
    m_hotkeyManager->UnregisterGlobalHotkey(1);
}

ItemManager* App::GetItemManager() {
    return m_itemManager.get();
}

Config* App::GetConfig() {
    return m_config.get();
}

MainWindow* App::GetMainWindow() {
    return m_mainWindow.get();
}

void App::HandleHotkey(int id) {
    if (id == 1) {
        m_mainWindow->Toggle();
    }
}

void App::SaveWindowPosition() {
    int x, y, w, h;
    m_window->GetPosition(x, y);
    m_window->GetSize(w, h);
    m_config->SetWindowPosition(x, y);
    m_config->SetWindowSize(w, h);
}

} // namespace mn
```

- [ ] **步骤 3：Commit**

```bash
git add src/app/App.h src/app/App.cpp
git commit -m "feat: add App main class"
```

---

### 任务 19：程序入口

**文件：**
- 创建：`src/main.cpp`
- 创建：`src/app/Resource.h`

- [ ] **步骤 1：创建 main.cpp**

```cpp
// src/main.cpp
#include "app/App.h"
#include <Windows.h>

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    mn::App app;
    
    if (!app.Initialize(hInstance)) {
        return -1;
    }
    
    int result = app.Run();
    app.Shutdown();
    
    return result;
}
```

- [ ] **步骤 2：创建 Resource.h**

```cpp
// src/app/Resource.h
#pragma once

#define IDI_APP_ICON 101
```

- [ ] **步骤 3：Commit**

```bash
git add src/main.cpp src/app/Resource.h
git commit -m "feat: add program entry point"
```

---

### 任务 20：验证构建

- [ ] **步骤 1：创建资源文件目录结构**

```bash
mkdir -p res/icons
```

- [ ] **步骤 2：尝试构建项目**

```bash
cmake -B build -A x64
cmake --build build --config Release
```

预期：构建成功，生成 MayeNano.exe

- [ ] **步骤 3：最终 Commit**

```bash
git add .
git commit -m "feat: complete Maye Nano MVP implementation"
```

---

## 完成检查清单

- [ ] 项目可以成功构建
- [ ] 运行时创建配置文件目录
- [ ] 全局快捷键可以唤醒/隐藏窗口
- [ ] 可以拖放文件添加项目
- [ ] 可以创建/删除分类
- [ ] 可以双击运行项目
- [ ] 窗口位置/大小可以保存和恢复
