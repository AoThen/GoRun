#include "PathUtils.h"
#include <filesystem>

#ifdef _WIN32
#include <Windows.h>
#include <ShlObj.h>
#endif

namespace mn::PathUtils {

std::wstring GetAppDataPath() {
#ifdef _WIN32
    // 便携化：使用程序所在目录下的 data 文件夹
    wchar_t exePath[MAX_PATH] = {};
    DWORD len = GetModuleFileNameW(nullptr, exePath, MAX_PATH);
    if (len == 0 || len >= MAX_PATH) {
        return L".\\data";  // 回退到当前目录
    }
    // 获取程序所在目录
    std::filesystem::path exeDir = std::filesystem::path(exePath).parent_path();
    return (exeDir / L"data").wstring();
#else
    return L"./data";
#endif
}

std::wstring GetExePath() {
#ifdef _WIN32
    wchar_t path[MAX_PATH] = {};
    DWORD len = GetModuleFileNameW(nullptr, path, MAX_PATH);
    if (len == 0 || len >= MAX_PATH) {
        return L"GoRun.exe";
    }
    return path;
#else
    return L"./GoRun";
#endif
}

std::wstring GetExeDir() {
    return GetParentDir(GetExePath());
}

std::wstring ToAbsolute(const std::wstring& path) {
#ifdef _WIN32
    wchar_t absPath[MAX_PATH] = {};
    DWORD len = GetFullPathNameW(path.c_str(), MAX_PATH, absPath, nullptr);
    if (len == 0 || len >= MAX_PATH) {
        return path;  // 失败时返回原路径
    }
    return absPath;
#else
    return std::filesystem::absolute(path).wstring();
#endif
}

bool Exists(const std::wstring& path) {
    try {
        return std::filesystem::exists(path);
    } catch (...) {
        return false;
    }
}

std::wstring GetParentDir(const std::wstring& path) {
    try {
        return std::filesystem::path(path).parent_path().wstring();
    } catch (...) {
        return L"";
    }
}

std::wstring GetFileBaseName(const std::wstring& path) {
    try {
        return std::filesystem::path(path).stem().wstring();
    } catch (...) {
        return L"";
    }
}

bool EnsureDirectory(const std::wstring& path) {
    try {
        return std::filesystem::create_directories(path);
    } catch (...) {
        return false;
    }
}

} // namespace mn::PathUtils
