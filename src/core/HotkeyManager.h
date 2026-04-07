#pragma once

#include <functional>
#include <string>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class HotkeyManager {
public:
    bool RegisterGlobalHotkey(int id, unsigned int modifiers, unsigned int vk);
    bool UnregisterGlobalHotkey(int id);
    void ProcessHotkey(unsigned long wParam);
    
    bool ParseHotkeyString(const std::wstring& hotkey, unsigned int& modifiers, unsigned int& vk);
    
    using HotkeyCallback = std::function<void(int id)>;
    void SetCallback(HotkeyCallback callback);

private:
    HotkeyCallback m_callback;
};

} // namespace mn