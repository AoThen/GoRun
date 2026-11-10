# 关键位置日志补充实现计划

> **面向 AI 代理的工作者：** 必需严格按照任务编号串行执行，每个任务完成后报告结果再继续下一个。不得并行操作。

**目标：** 在 GoRun 核心流程的关键位置补充日志，覆盖 P0（静默失败风险）和 P1（数据状态变更）场景

**架构：** 在现有 Logger 框架基础上，为 7 个文件添加 17 处日志调用，仅需添加 `#include "utils/Logger.h"` 和 LOG_xxx 宏调用

**涉及文件：**
- 修改：`src/core/Runner.cpp`, `src/core/Storage.cpp`, `src/platform/Window.cpp`, `src/platform/TrayIcon.cpp`, `src/core/ItemManager.cpp`, `src/core/Config.cpp`, `src/platform/D3D11Renderer.cpp`

---

### 任务 1：Runner.cpp — 添加运行失败日志（P0）

**文件：** 修改 `src/core/Runner.cpp`

- [ ] **步骤 1：添加头文件**

在 `#include "utils/PathUtils.h"` 之后添加：

```cpp
#include "utils/Logger.h"
```

- [ ] **步骤 2：Run() 文件未找到时记录 ERROR**

在第 24 行 `errorMessage` 赋值后添加：

```cpp
    LOG_ERRORW(L"Runner::Run failed: " + result.errorMessage);
```

即：

```cpp
    if (!isUrl && !PathUtils::Exists(item.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到: " + item.target;
        LOG_ERRORW(L"Runner::Run failed: " + result.errorMessage);
        return result;
    }
```

- [ ] **步骤 3：Run() ShellExecuteExW 失败时记录 ERROR**

在第 42 行 `return result;` 之前添加：

```cpp
        LOG_ERRORW(L"Runner::Run ShellExecuteExW failed: " + result.errorMessage);
```

即：

```cpp
    if (!ShellExecuteExW(&sei)) {
        unsigned long err = GetLastError();
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err) + L" (" + item.target + L")";
        LOG_ERRORW(L"Runner::Run ShellExecuteExW failed: " + result.errorMessage);
        if (m_runCallback) m_runCallback(item, false);
        return result;
    }
```

- [ ] **步骤 4：RunAsAdmin() 文件未找到时记录 ERROR**

在第 65 行 `return result;` 之前添加：

```cpp
    LOG_ERRORW(L"Runner::RunAsAdmin failed: file not found - " + item.target);
```

- [ ] **步骤 5：RunAsAdmin() ShellExecuteExW 失败时记录 ERROR**

在第 84 行（用户取消）添加：

```cpp
            LOG_ERRORW(L"Runner::RunAsAdmin cancelled by user: " + item.target);
```

在第 91 行 `return result;` 前添加：

```cpp
        LOG_ERRORW(L"Runner::RunAsAdmin failed: " + result.errorMessage);
```

---

### 任务 2：Storage.cpp — 添加存储写入失败日志（P0）

**文件：** 修改 `src/core/Storage.cpp`（已包含 Logger.h）

- [ ] **步骤 1：SaveToFile() 临时文件写入失败时记录 ERROR**

在第 83 行 `return false;` 之前添加：

```cpp
        LOG_ERROR("Storage::SaveToFile: failed to open temp file");
```

- [ ] **步骤 2：SaveToFile() MoveFileExW 失败时记录 ERROR**

在第 92 行 `return false;` 之前添加：

```cpp
        LOG_ERROR("Storage::SaveToFile: failed to rename temp file");
```

- [ ] **步骤 3：RotateBackups() 备份轮转失败时记录 ERROR**

目前 RotateBackups() 不检查 MoveFileExW/CopyFileW 的返回值，需要添加检查：
在第 28 行添加：

```cpp
    if (PathUtils::Exists(bak2)) {
        if (!MoveFileExW(bak2.c_str(), bak3.c_str(), MOVEFILE_REPLACE_EXISTING)) {
            LOG_ERROR("Storage::RotateBackups: failed to rotate bak2 -> bak3");
        }
    }
```

在第 32 行添加：

```cpp
    if (PathUtils::Exists(bak1)) {
        if (!MoveFileExW(bak1.c_str(), bak2.c_str(), MOVEFILE_REPLACE_EXISTING)) {
            LOG_ERROR("Storage::RotateBackups: failed to rotate bak1 -> bak2");
        }
    }
```

在第 36 行添加：

```cpp
    if (PathUtils::Exists(m_path)) {
        if (!CopyFileW(m_path.c_str(), bak1.c_str(), FALSE)) {
            LOG_ERROR("Storage::RotateBackups: failed to copy current -> bak1");
            return false;
        }
    }
```

注意：修改后 RotateBackups() 函数需要调整以支持 `return false;`。当前返回类型是 `bool`，但最后是 `return true;`，需要在 CopyFileW 失败时返回 false。

---

### 任务 3：Window.cpp — 添加窗口创建失败日志（P0）

**文件：** 修改 `src/platform/Window.cpp`

- [ ] **步骤 1：添加头文件**

在已有的 include 末尾添加：

```cpp
#include "utils/Logger.h"
```

- [ ] **步骤 2：CreateWindowExW 失败时记录 ERROR**

在第 47 行之后（m_hwnd 赋值完成），添加失败日志：

```cpp
    if (!m_hwnd) {
        LOG_ERRORW(L"Window::Create: CreateWindowExW failed");
        return false;
    }
    
    // 启用 DWM 窗口阴影效果
    ...
```

---

### 任务 4：TrayIcon.cpp — 添加托盘图标失败日志（P0）

**文件：** 修改 `src/platform/TrayIcon.cpp`

- [ ] **步骤 1：添加头文件**

在已有的 include 末尾添加：

```cpp
#include "utils/Logger.h"
```

- [ ] **步骤 2：LoadIcon 失败时记录 ERROR**

在第 16 行之后（LoadIcon 失败分支内）添加：

```cpp
    if (!hIcon) {
        LOG_ERRORW(L"TrayIcon::Create: failed to load app icon, using default");
        hIcon = LoadIcon(nullptr, IDI_APPLICATION);
    }
```

- [ ] **步骤 3：Shell_NotifyIconW 失败时记录 ERROR**

在第 29 行之后添加：

```cpp
    if (!m_created) {
        LOG_ERRORW(L"TrayIcon::Create: Shell_NotifyIconW failed");
    }
```

---

### 任务 5：ItemManager.cpp — 添加数据操作日志（P1）

**文件：** 修改 `src/core/ItemManager.cpp`

- [ ] **步骤 1：添加头文件**

在 `#include "utils/StringUtils.h"` 之后添加：

```cpp
#include "utils/Logger.h"
```

- [ ] **步骤 2：AddCategory()/DeleteCategory() 添加 INFO 日志**

在 `AddCategory()` 末尾（第 51 行 `m_storage->AddCategory(category);` 之后）添加：

```cpp
    LOG_INFOW(L"ItemManager::AddCategory: " + category.name);
```

在 `DeleteCategory()` 末尾（第 78 行之后）添加：

```cpp
    LOG_INFOW(L"ItemManager::DeleteCategory: " + id);
```

- [ ] **步骤 3：AddItem()/DeleteItem() 添加 INFO 日志**

在 `AddItem()` 末尾（第 111 行之后）添加：

```cpp
    LOG_INFOW(L"ItemManager::AddItem: " + item.name);
```

在 `DeleteItem()` 中第 157 行（`if (item)` 分支内）添加：

```cpp
    LOG_INFOW(L"ItemManager::DeleteItem: " + item->name);
```

- [ ] **步骤 4：ResolveShortcut() COM 失败添加 ERROR 日志**

在各处提前 return 前添加日志：

第 236 行（CoCreateInstance 失败）：

```cpp
        if (FAILED(hr)) {
            LOG_ERRORW(L"ItemManager::ResolveShortcut: CoCreateInstance failed");
            if (needUninitialize) CoUninitialize();
            return info;
        }
```

第 241 行（QueryInterface 失败）：

```cpp
        if (FAILED(hr)) {
            LOG_ERRORW(L"ItemManager::ResolveShortcut: QueryInterface failed");
            pShellLink->Release();
            if (needUninitialize) CoUninitialize();
            return info;
        }
```

第 248 行（Load 失败）：

```cpp
        if (FAILED(hr)) {
            LOG_ERRORW(L"ItemManager::ResolveShortcut: IPersistFile::Load failed");
            pPersistFile->Release();
            pShellLink->Release();
            if (needUninitialize) CoUninitialize();
            return info;
        }
```

最后在返回前添加（快捷方式解析成功但目标为空）：

```cpp
    if (!info.target.empty()) {
        LOG_INFOW(L"ItemManager::ResolveShortcut: resolved " + lnkPath + L" -> " + info.target);
    } else {
        LOG_ERRORW(L"ItemManager::ResolveShortcut: failed to resolve " + lnkPath);
    }
```

---

### 任务 6：Config.cpp — 添加注册表操作失败日志（P1）

**文件：** 修改 `src/core/Config.cpp`

- [ ] **步骤 1：添加头文件**

在已有的 include 之后添加：

```cpp
#include "utils/Logger.h"
```

- [ ] **步骤 2：UpdateRegistryAutoStart() 添加 ERROR 日志**

在第 127-132 行（RegSetValueExW）后将 `return result == ERROR_SUCCESS;` 改为：

```cpp
            bool regSuccess = (result == ERROR_SUCCESS);
            if (!regSuccess) {
                LOG_ERRORW(L"Config::UpdateRegistryAutoStart: RegSetValueExW failed");
            }
            RegCloseKey(hKey);
            return regSuccess;
```

第 136-141 行（RegDeleteValueW）同理：

```cpp
            bool regSuccess = (result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND);
            if (!regSuccess) {
                LOG_ERRORW(L"Config::UpdateRegistryAutoStart: RegDeleteValueW failed");
            }
            RegCloseKey(hKey);
            return regSuccess;
```

第 122 行（RegOpenKeyExW 失败添加）：

```cpp
        if (RegOpenKeyExW(HKEY_CURRENT_USER, runKeyPath, 0, KEY_WRITE, &hKey) != ERROR_SUCCESS) {
            LOG_ERRORW(L"Config::UpdateRegistryAutoStart: failed to open registry key");
            return false;
        }
```

---

### 任务 7：D3D11Renderer.cpp — 添加 D3D 初始化失败日志（P1）

**文件：** 修改 `src/platform/D3D11Renderer.cpp`（已包含 Logger.h）

- [ ] **步骤 1：D3D11CreateDeviceAndSwapChain 失败记录 ERROR**

在第 148 行 `if (FAILED(hr)) return false;` 改为：

```cpp
    if (FAILED(hr)) {
        LOG_ERROR("D3D11Renderer::Initialize: D3D11CreateDeviceAndSwapChain failed");
        return false;
    }
```

---

### 任务 8：验证构建

- [ ] **步骤 1：检查测试文件是否需修改**

检查 `tests/CMakeLists.txt`：
- `Config.cpp` 和 `Storage.cpp` 已在测试构建中
- 这两个文件添加的 Logger.h 头文件不引入额外链接依赖（Logger.cpp 已在测试构建中）
- **结论：测试文件无需修改**

- [ ] **步骤 2：编译验证**

```bash
cmake -B build -A x64
cmake --build build --config Release
```

预期：无编译错误，`GoRun.exe` 生成成功
