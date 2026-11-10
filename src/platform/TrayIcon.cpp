#include "TrayIcon.h"
#include "app/Resource.h"
#include "utils/Logger.h"
#include "utils/StringUtils.h"
#include "core/Localization.h"

#ifdef _WIN32
#include <ShellApi.h>

namespace mn {

bool TrayIcon::Create(HWND hwnd, UINT messageId, const wchar_t* tooltip) {
    m_hwnd = hwnd;
    m_messageId = messageId;
    
    // 加载自定义图标
    HICON hIcon = LoadIcon(GetModuleHandle(nullptr), MAKEINTRESOURCE(IDI_APP_ICON));
    if (!hIcon) {
        LOG_ERRORW(L"TrayIcon::Create: failed to load app icon, using default");
        hIcon = LoadIcon(nullptr, IDI_APPLICATION);
    }
    
    NOTIFYICONDATAW nid = {};
    nid.cbSize = sizeof(NOTIFYICONDATAW);
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = messageId;
    nid.hIcon = hIcon;
    wcscpy_s(nid.szTip, tooltip);
    
    m_created = Shell_NotifyIconW(NIM_ADD, &nid) != FALSE;
    if (!m_created) {
        LOG_ERRORW(L"TrayIcon::Create: Shell_NotifyIconW failed");
    }
    return m_created;
}

void TrayIcon::Destroy() {
    if (!m_created) return;
    
    NOTIFYICONDATAW nid = {};
    nid.cbSize = sizeof(NOTIFYICONDATAW);
    nid.hWnd = m_hwnd;
    nid.uID = 1;
    
    Shell_NotifyIconW(NIM_DELETE, &nid);
    m_created = false;
}

void TrayIcon::HandleMessage(WPARAM wParam, LPARAM lParam) {
    if (wParam != 1) return;
    
    switch (LOWORD(lParam)) {
        case WM_LBUTTONDBLCLK:
            // 双击托盘图标
            if (m_showCallback) {
                m_showCallback();
            }
            break;
            
        case WM_RBUTTONUP:
            // 右键菜单
            ShowContextMenu();
            break;
    }
}

void TrayIcon::ShowContextMenu() {
    POINT pt;
    GetCursorPos(&pt);
    
    HMENU hMenu = CreatePopupMenu();
    AppendMenuW(hMenu, MF_STRING, ID_TRAY_SHOW, StringUtils::Utf8ToWString(TrUtf8("Tray_ShowHide")).c_str());
    AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(hMenu, MF_STRING, ID_TRAY_EXIT, StringUtils::Utf8ToWString(TrUtf8("Tray_Exit")).c_str());
    
    // 必须设置前台窗口，否则菜单可能不会正确关闭
    SetForegroundWindow(m_hwnd);
    
    int cmd = TrackPopupMenu(
        hMenu,
        TPM_RIGHTBUTTON | TPM_NONOTIFY | TPM_RETURNCMD,
        pt.x, pt.y,
        0, m_hwnd, nullptr
    );
    
    DestroyMenu(hMenu);
    
    switch (cmd) {
        case ID_TRAY_SHOW:
            if (m_showCallback) {
                m_showCallback();
            }
            break;
        case ID_TRAY_EXIT:
            if (m_exitCallback) {
                m_exitCallback();
            }
            break;
    }
}

} // namespace mn

#endif
