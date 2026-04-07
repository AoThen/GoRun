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

private:
    Storage* m_storage = nullptr;
};

} // namespace mn
