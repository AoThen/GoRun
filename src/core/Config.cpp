#include "Config.h"
#include "Storage.h"
#include "utils/Logger.h"

#ifdef _WIN32
#include <Windows.h>
#include <ShlObj.h>
#endif

namespace mn {

void Config::Initialize(Storage* storage) {
    m_storage = storage;
}

std::wstring Config::GetGlobalHotkey() const {
    return m_storage ? m_storage->GetConfig("globalHotkey", L"Ctrl+Alt+Z") : L"Ctrl+Alt+Z";
}

void Config::SetGlobalHotkey(const std::wstring& hotkey) {
    if (m_storage) m_storage->SetConfig("globalHotkey", hotkey);
}

static int ClampToScreenX(int pos) {
    int minX = GetSystemMetrics(SM_XVIRTUALSCREEN);
    int maxX = minX + GetSystemMetrics(SM_CXVIRTUALSCREEN);
    if (pos < minX - 100 || pos > maxX - 100) return 100;
    return pos;
}

static int ClampToScreenY(int pos) {
    int minY = GetSystemMetrics(SM_YVIRTUALSCREEN);
    int maxY = minY + GetSystemMetrics(SM_CYVIRTUALSCREEN);
    if (pos < minY - 100 || pos > maxY - 100) return 100;
    return pos;
}

int Config::GetWindowX() const {
    if (!m_storage) return 100;
    try {
        int x = std::stoi(m_storage->GetConfig("windowX", L"100"));
        return ClampToScreenX(x);
    } catch (...) {
        return 100;
    }
}

int Config::GetWindowY() const {
    if (!m_storage) return 100;
    try {
        int y = std::stoi(m_storage->GetConfig("windowY", L"100"));
        return ClampToScreenY(y);
    } catch (...) {
        return 100;
    }
}

int Config::GetWindowWidth() const {
    if (!m_storage) return 800;
    try {
        return std::stoi(m_storage->GetConfig("windowWidth", L"800"));
    } catch (...) {
        return 800;
    }
}

int Config::GetWindowHeight() const {
    if (!m_storage) return 600;
    try {
        return std::stoi(m_storage->GetConfig("windowHeight", L"600"));
    } catch (...) {
        return 600;
    }
}

void Config::SetWindowPosition(int x, int y) {
    if (m_storage) {
        m_storage->SetConfig("windowX", std::to_wstring(x));
        m_storage->SetConfig("windowY", std::to_wstring(y));
    }
}

void Config::SetWindowSize(int width, int height) {
    if (m_storage) {
        m_storage->SetConfig("windowWidth", std::to_wstring(width));
        m_storage->SetConfig("windowHeight", std::to_wstring(height));
    }
}

bool Config::GetAutoStart() const {
    if (!m_storage) return false;
    return m_storage->GetConfig("autoStart", L"false") == L"true";
}

void Config::SetAutoStart(bool enabled) {
    if (m_storage) {
        m_storage->SetConfig("autoStart", enabled ? L"true" : L"false");
    }
    // 同步更新注册表
    UpdateRegistryAutoStart(enabled);
}

bool Config::IsAutoStartEnabled() const {
#ifdef _WIN32
    HKEY hKey;
    const wchar_t* runKeyPath = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    
    if (RegOpenKeyExW(HKEY_CURRENT_USER, runKeyPath, 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        wchar_t value[MAX_PATH] = {0};
        DWORD size = sizeof(value);
        DWORD type = 0;
        
        LONG result = RegQueryValueExW(hKey, L"GoRun", nullptr, &type, 
            reinterpret_cast<LPBYTE>(value), &size);
        RegCloseKey(hKey);
        
        return result == ERROR_SUCCESS;
    }
#endif
    return false;
}

bool Config::UpdateRegistryAutoStart(bool enabled) {
#ifdef _WIN32
    HKEY hKey;
    const wchar_t* runKeyPath = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    
    if (enabled) {
        // 添加开机自启
        if (RegOpenKeyExW(HKEY_CURRENT_USER, runKeyPath, 0, KEY_WRITE, &hKey) != ERROR_SUCCESS) {
            LOG_ERRORW(L"Config::UpdateRegistryAutoStart: failed to open registry key");
            return false;
        }
        wchar_t exePath[MAX_PATH] = {0};
        GetModuleFileNameW(nullptr, exePath, MAX_PATH);
        
        LONG result = RegSetValueExW(hKey, L"GoRun", 0, REG_SZ,
            reinterpret_cast<const BYTE*>(exePath),
            static_cast<DWORD>((wcslen(exePath) + 1) * sizeof(wchar_t)));
        bool regSuccess = (result == ERROR_SUCCESS);
        if (!regSuccess) {
            LOG_ERRORW(L"Config::UpdateRegistryAutoStart: RegSetValueExW failed");
        }
        RegCloseKey(hKey);
        return regSuccess;
    } else {
        // 移除开机自启
        if (RegOpenKeyExW(HKEY_CURRENT_USER, runKeyPath, 0, KEY_WRITE, &hKey) != ERROR_SUCCESS) {
            LOG_ERRORW(L"Config::UpdateRegistryAutoStart: failed to open registry key");
            return false;
        }
        LONG result = RegDeleteValueW(hKey, L"GoRun");
        bool regSuccess = (result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND);
        if (!regSuccess) {
            LOG_ERRORW(L"Config::UpdateRegistryAutoStart: RegDeleteValueW failed");
        }
        RegCloseKey(hKey);
        return regSuccess;
    }
#endif
    return false;
}

int Config::GetTheme() const {
    if (!m_storage) return 0;
    try {
        return std::stoi(m_storage->GetConfig("theme", L"0"));
    } catch (...) {
        return 0;
    }
}

void Config::SetTheme(int theme) {
    if (m_storage) {
        m_storage->SetConfig("theme", std::to_wstring(theme));
    }
}

} // namespace mn
