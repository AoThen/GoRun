#include "Runner.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include "utils/Logger.h"
#include <cwctype>

#ifdef _WIN32
#include <Windows.h>
#include <ShellApi.h>
#endif

namespace mn {

static bool StartsWithIgnoreCase(const std::wstring& str, const std::wstring& prefix) {
    if (str.size() < prefix.size()) return false;
    for (size_t i = 0; i < prefix.size(); i++) {
        if (std::towlower(str[i]) != std::towlower(prefix[i])) return false;
    }
    return true;
}

static bool IsUrl(const std::wstring& target) {
    return StartsWithIgnoreCase(target, L"http://") ||
           StartsWithIgnoreCase(target, L"https://") ||
           StartsWithIgnoreCase(target, L"ftp://") ||
           StartsWithIgnoreCase(target, L"ftps://") ||
           StartsWithIgnoreCase(target, L"steam://") ||
           StartsWithIgnoreCase(target, L"mailto:");
}

std::wstring Runner::ExpandVariables(const std::wstring& input, const Item& item) {
    if (input.empty()) return input;
    
    std::wstring result = input;
    
    // 展开 Windows 环境变量 (如 %PATH%, %USERPROFILE% 等)
    DWORD bufLen = ExpandEnvironmentStringsW(result.c_str(), nullptr, 0);
    if (bufLen > 0) {
        std::wstring expanded(bufLen - 1, L'\0');
        if (ExpandEnvironmentStringsW(result.c_str(), &expanded[0], bufLen)) {
            result = expanded;
        }
    }
    
    // 展开 GoRun 内置变量
    // %mp% - 当前目录（项目目标的父目录）
    if (!item.target.empty()) {
        std::wstring parentDir = PathUtils::GetParentDir(item.target);
        std::wstring drive;
        if (item.target.size() >= 2 && item.target[1] == L':') {
            drive = item.target.substr(0, 2);
        }
        
        size_t pos;
        while ((pos = result.find(L"%mp%")) != std::wstring::npos) {
            result.replace(pos, 4, parentDir);
        }
        while ((pos = result.find(L"%mr%")) != std::wstring::npos) {
            result.replace(pos, 4, drive);
        }
    }
    
    // %so% - 搜索参数
    if (!m_searchQuery.empty()) {
        size_t pos;
        while ((pos = result.find(L"%so%")) != std::wstring::npos) {
            result.replace(pos, 4, m_searchQuery);
        }
        // %so-url% - URL 编码的搜索参数
        std::wstring urlEncoded;
        for (wchar_t ch : m_searchQuery) {
            if (ch >= L'a' && ch <= L'z') urlEncoded += ch;
            else if (ch >= L'A' && ch <= L'Z') urlEncoded += ch;
            else if (ch >= L'0' && ch <= L'9') urlEncoded += ch;
            else if (ch == L'-' || ch == L'_' || ch == L'.' || ch == L'~') urlEncoded += ch;
            else if (ch == L' ') urlEncoded += L'+';
            else {
                wchar_t buf[8];
                _snwprintf(buf, 8, L"%%%02X", (unsigned)ch);
                urlEncoded += buf;
            }
        }
        size_t pos2;
        while ((pos2 = result.find(L"%so-url%")) != std::wstring::npos) {
            result.replace(pos2, 8, urlEncoded);
        }
    }
    
    return result;
}

RunResult Runner::Run(const Item& item) {
    RunResult result;

#ifdef _WIN32
    bool isUrl = IsUrl(item.target);
    
    // 展开变量
    Item expandedItem = item;
    expandedItem.target = ExpandVariables(item.target, item);
    expandedItem.arguments = ExpandVariables(item.arguments, item);
    expandedItem.workingDir = ExpandVariables(item.workingDir, item);
    
    // URL 不需要检查文件存在性
    if (!isUrl && !PathUtils::Exists(expandedItem.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到: " + item.target;
        LOG_ERRORW(L"Runner::Run failed: " + result.errorMessage);
        return result;
    }
    
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpFile = expandedItem.target.c_str();
    sei.lpParameters = expandedItem.arguments.empty() ? nullptr : expandedItem.arguments.c_str();
    sei.lpDirectory = expandedItem.workingDir.empty() ? nullptr : expandedItem.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        unsigned long err = GetLastError();
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err) + L" (" + item.target + L")";
        LOG_ERRORW(L"Runner::Run ShellExecuteExW failed: " + result.errorMessage);
        if (m_runCallback) m_runCallback(item, false);
        return result;
    }
    
    // 关闭进程句柄，避免资源泄漏
    if (sei.hProcess) {
        CloseHandle(sei.hProcess);
    }
    
    result.success = true;
    if (m_runCallback) m_runCallback(item, true);
#else
    result.success = false;
    result.errorMessage = L"仅支持 Windows 平台";
#endif
    return result;
}

RunResult Runner::RunAsAdmin(const Item& item) {
    RunResult result;

    bool isUrl = IsUrl(item.target);
    
    // 展开变量
    Item expandedItem = item;
    expandedItem.target = ExpandVariables(item.target, item);
    expandedItem.arguments = ExpandVariables(item.arguments, item);
    expandedItem.workingDir = ExpandVariables(item.workingDir, item);
    
    if (!isUrl && !PathUtils::Exists(expandedItem.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到";
        LOG_ERRORW(L"Runner::RunAsAdmin failed: file not found - " + item.target);
        return result;
    }
    
#ifdef _WIN32
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = L"runas";
    sei.lpFile = expandedItem.target.c_str();
    sei.lpParameters = expandedItem.arguments.empty() ? nullptr : expandedItem.arguments.c_str();
    sei.lpDirectory = expandedItem.workingDir.empty() ? nullptr : expandedItem.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        unsigned long err = GetLastError();
        if (err == ERROR_CANCELLED) {
            result.success = false;
            result.error = RunError::AccessDenied;
            result.errorMessage = L"用户取消了提权请求";
            LOG_ERRORW(L"Runner::RunAsAdmin cancelled by user: " + item.target);
            if (m_runCallback) m_runCallback(item, false);
            return result;
        }
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err);
        LOG_ERRORW(L"Runner::RunAsAdmin failed: " + result.errorMessage);
        if (m_runCallback) m_runCallback(item, false);
        return result;
    }
    
    // 关闭进程句柄，避免资源泄漏
    if (sei.hProcess) {
        CloseHandle(sei.hProcess);
    }
    
    result.success = true;
    if (m_runCallback) m_runCallback(item, true);
#else
    result.success = false;
    result.errorMessage = L"仅支持 Windows 平台";
#endif
    return result;
}

RunError Runner::MapError(unsigned long errorCode) {
#ifdef _WIN32
    switch (errorCode) {
        case ERROR_FILE_NOT_FOUND: return RunError::FileNotFound;
        case ERROR_PATH_NOT_FOUND: return RunError::PathNotFound;
        case ERROR_ACCESS_DENIED: return RunError::AccessDenied;
        case ERROR_NOT_ENOUGH_MEMORY: return RunError::OutOfMemory;
        case ERROR_DLL_NOT_FOUND: return RunError::DllNotFound;
        default: return RunError::Unknown;
    }
#else
    return RunError::Unknown;
#endif
}

std::wstring Runner::GetErrorMessage(unsigned long errorCode) {
#ifdef _WIN32
    switch (errorCode) {
        case ERROR_FILE_NOT_FOUND: return L"文件未找到";
        case ERROR_PATH_NOT_FOUND: return L"路径未找到";
        case ERROR_ACCESS_DENIED: return L"拒绝访问";
        case ERROR_NOT_ENOUGH_MEMORY: return L"内存不足";
        case ERROR_DLL_NOT_FOUND: return L"动态链接库未找到";
        default: return L"未知错误";
    }
#else
    return L"未知错误";
#endif
}

} // namespace mn
