#pragma once

#ifdef _WIN32
#include <Windows.h>
#include <functional>
#endif

namespace mn {

class TrayIcon {
public:
#ifdef _WIN32
    bool Create(HWND hwnd, UINT messageId, const wchar_t* tooltip);
    void Destroy();
    using ShowCallback = std::function<void()>;
    using ExitCallback = std::function<void()>;
    
    void OnShow(ShowCallback cb) { m_showCallback = cb; }
    void OnExit(ExitCallback cb) { m_exitCallback = cb; }
    
    void HandleMessage(WPARAM wParam, LPARAM lParam);
#endif

private:
#ifdef _WIN32
    void ShowContextMenu();
    
    HWND m_hwnd = nullptr;
    UINT m_messageId = 0;
    bool m_created = false;
    
    ShowCallback m_showCallback;
    ExitCallback m_exitCallback;
#endif
};

} // namespace mn
