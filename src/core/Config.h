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
    
    // 开机自启
    bool GetAutoStart() const;
    void SetAutoStart(bool enabled);
    bool IsAutoStartEnabled() const;  // 检查注册表实际状态

private:
    Storage* m_storage = nullptr;
    bool UpdateRegistryAutoStart(bool enabled);
};

} // namespace mn
