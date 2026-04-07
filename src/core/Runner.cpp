#include "Runner.h"
#include "utils/PathUtils.h"

#ifdef _WIN32
#include <Windows.h>
#include <ShellApi.h>
#endif

namespace mn {

RunResult Runner::Run(const Item& item) {
    RunResult result;
    
    if (!PathUtils::Exists(item.target)) {
        result.success = false;
        result.error = RunError::FileNotFound;
        result.errorMessage = L"文件未找到";
        return result;
    }
    
#ifdef _WIN32
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpFile = item.target.c_str();
    sei.lpParameters = item.arguments.empty() ? nullptr : item.arguments.c_str();
    sei.lpDirectory = item.workingDir.empty() ? nullptr : item.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        unsigned long err = GetLastError();
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err);
        return result;
    }
    
    result.success = true;
#else
    result.success = false;
    result.errorMessage = L"仅支持 Windows 平台";
#endif
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
    
#ifdef _WIN32
    SHELLEXECUTEINFOW sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFOW);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = L"runas";
    sei.lpFile = item.target.c_str();
    sei.lpParameters = item.arguments.empty() ? nullptr : item.arguments.c_str();
    sei.lpDirectory = item.workingDir.empty() ? nullptr : item.workingDir.c_str();
    sei.nShow = SW_SHOWNORMAL;
    
    if (!ShellExecuteExW(&sei)) {
        unsigned long err = GetLastError();
        if (err == ERROR_CANCELLED) {
            result.success = true;
            return result;
        }
        result.success = false;
        result.error = MapError(err);
        result.errorMessage = GetErrorMessage(err);
        return result;
    }
    
    result.success = true;
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
