#pragma once

#include <string>
#include <vector>
#include <functional>
#include <chrono>
#include <random>
#include <unordered_map>

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
