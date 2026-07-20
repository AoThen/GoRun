#include "HotkeyManager.h"
#include "utils/StringUtils.h"
#include <cwctype>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

bool HotkeyManager::RegisterGlobalHotkey(int id, unsigned int modifiers, unsigned int vk) {
#ifdef _WIN32
    return RegisterHotKey(nullptr, id, modifiers, vk) != FALSE;
#else
    return false;
#endif
}

bool HotkeyManager::UnregisterGlobalHotkey(int id) {
#ifdef _WIN32
    return UnregisterHotKey(nullptr, id) != FALSE;
#else
    return false;
#endif
}

void HotkeyManager::ProcessHotkey(unsigned long wParam) {
    if (m_callback) {
        m_callback(static_cast<int>(wParam));
    }
}

bool HotkeyManager::ParseHotkeyString(const std::wstring& hotkey, unsigned int& modifiers, unsigned int& vk) {
    modifiers = 0;
    vk = 0;
    
    auto parts = StringUtils::Split(hotkey, L'+');
    for (size_t i = 0; i < parts.size(); i++) {
        auto part = StringUtils::Trim(parts[i]);
        
        if (part == L"Ctrl" || part == L"ctrl") {
#ifdef _WIN32
            modifiers |= MOD_CONTROL;
#endif
        } else if (part == L"Alt" || part == L"alt") {
#ifdef _WIN32
            modifiers |= MOD_ALT;
#endif
        } else if (part == L"Shift" || part == L"shift") {
#ifdef _WIN32
            modifiers |= MOD_SHIFT;
#endif
        } else if (part == L"Win" || part == L"win") {
#ifdef _WIN32
            modifiers |= MOD_WIN;
#endif
        } else if (i == parts.size() - 1) {
            if (part.size() == 1) {
                wchar_t c = part[0];
                if (c >= L'a' && c <= L'z') {
                    vk = static_cast<unsigned int>(c - L'a' + 'A');
                } else if (c >= L'A' && c <= L'Z') {
                    vk = static_cast<unsigned int>(c);
                } else if (c >= L'0' && c <= L'9') {
                    vk = static_cast<unsigned int>(c);
                } else {
                    return false;
                }
            }
#ifdef _WIN32
            else if (part == L"Space") vk = VK_SPACE;
            else if (part == L"Tab") vk = VK_TAB;
            else if (part == L"Escape" || part == L"Esc") vk = VK_ESCAPE;
            else if (part == L"Enter") vk = VK_RETURN;
            else if (part == L"Backspace") vk = VK_BACK;
            else if (part == L"Delete" || part == L"Del") vk = VK_DELETE;
            else if (part == L"Insert" || part == L"Ins") vk = VK_INSERT;
            else if (part == L"Home") vk = VK_HOME;
            else if (part == L"End") vk = VK_END;
            else if (part == L"PageUp") vk = VK_PRIOR;
            else if (part == L"PageDown") vk = VK_NEXT;
            else if (part == L"PrintScreen") vk = VK_SNAPSHOT;
            else if (part == L"Pause") vk = VK_PAUSE;
            else if (part == L"NumLock") vk = VK_NUMLOCK;
            else if (part == L"CapsLock") vk = VK_CAPITAL;
            else if (part == L"ScrollLock") vk = VK_SCROLL;
            else if (part == L"F1") vk = VK_F1;
            else if (part == L"F2") vk = VK_F2;
            else if (part == L"F3") vk = VK_F3;
            else if (part == L"F4") vk = VK_F4;
            else if (part == L"F5") vk = VK_F5;
            else if (part == L"F6") vk = VK_F6;
            else if (part == L"F7") vk = VK_F7;
            else if (part == L"F8") vk = VK_F8;
            else if (part == L"F9") vk = VK_F9;
            else if (part == L"F10") vk = VK_F10;
            else if (part == L"F11") vk = VK_F11;
            else if (part == L"F12") vk = VK_F12;
#endif
            else return false;
        }
    }
    
    return modifiers != 0 && vk != 0;
}

void HotkeyManager::SetCallback(HotkeyCallback callback) {
    m_callback = callback;
}

} // namespace mn
