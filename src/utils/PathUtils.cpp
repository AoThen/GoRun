#include "PathUtils.h"
#include <filesystem>

#ifdef _WIN32
#include <Windows.h>
#include <ShlObj.h>
#endif

namespace mn::PathUtils {

std::wstring GetAppDataPath() {
#ifdef _WIN32
    wchar_t path[MAX_PATH] = {};
    HRESULT hr = SHGetFolderPathW(nullptr, CSIDL_APPDATA, nullptr, 0, path);
    if (FAILED(hr)) {
        return L"C:\\GoRun";  // 回退到默认路径
    }
    return std::wstring(path) + L"\\GoRun";
#else
    return L"/tmp/GoRun";
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

std::wstring ToRelative(const std::wstring& path) {
    std::filesystem::path p(path);
    std::filesystem::path base(GetExeDir());
    try {
        return std::filesystem::relative(p, base).wstring();
    } catch (...) {
        return path;
    }
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

std::wstring GetFileName(const std::wstring& path) {
    try {
        return std::filesystem::path(path).filename().wstring();
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
