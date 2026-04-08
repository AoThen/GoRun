#include "Window.h"
#include "app/Resource.h"

#ifdef _WIN32
#include <Windows.h>
#include <ShellApi.h>
#include <dwmapi.h>
#pragma comment(lib, "dwmapi.lib")
#endif

namespace mn {

#ifdef _WIN32
static const wchar_t* CLASS_NAME = L"GoRunWindowClass";
#endif

bool Window::Create(const std::wstring& title, int width, int height, int x, int y) {
#ifdef _WIN32
    // 注册窗口类
    WNDCLASSEXW wc = {};
    wc.cbSize = sizeof(WNDCLASSEXW);
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = StaticWndProc;
    wc.hInstance = GetModuleHandle(nullptr);
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszClassName = CLASS_NAME;
    RegisterClassExW(&wc);
    
    // 创建窗口
    m_hwnd = CreateWindowExW(
        WS_EX_APPWINDOW,
        CLASS_NAME,
        title.c_str(),
        WS_OVERLAPPEDWINDOW,
        x, y, width, height,
        nullptr, nullptr,
        GetModuleHandle(nullptr),
        this
    );
    
    // 启用 DWM 窗口阴影效果
    if (m_hwnd) {
        // 设置窗口阴影边距（负值表示阴影延伸到窗口外）
        MARGINS margins = { -1, -1, -1, -1 };
        DwmExtendFrameIntoClientArea(m_hwnd, &margins);
    }
    
    return m_hwnd != nullptr;
#else
    return false;
#endif
}

void Window::Show() {
    m_visible = true;
#ifdef _WIN32
    ShowWindow(m_hwnd, SW_SHOW);
    SetForegroundWindow(m_hwnd);
#endif
}

void Window::Hide() {
    m_visible = false;
#ifdef _WIN32
    ShowWindow(m_hwnd, SW_HIDE);
#endif
}

void Window::Toggle() {
    if (m_visible) Hide();
    else Show();
}

bool Window::IsVisible() const {
    return m_visible;
}

void Window::SetPosition(int x, int y) {
#ifdef _WIN32
    SetWindowPos(m_hwnd, nullptr, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
#endif
}

void Window::SetSize(int width, int height) {
#ifdef _WIN32
    SetWindowPos(m_hwnd, nullptr, 0, 0, width, height, SWP_NOMOVE | SWP_NOZORDER);
#endif
}

void Window::GetPosition(int& x, int& y) {
#ifdef _WIN32
    RECT rect;
    GetWindowRect(m_hwnd, &rect);
    x = rect.left;
    y = rect.top;
#else
    x = y = 0;
#endif
}

void Window::GetSize(int& width, int& height) {
#ifdef _WIN32
    RECT rect;
    GetWindowRect(m_hwnd, &rect);
    width = rect.right - rect.left;
    height = rect.bottom - rect.top;
#else
    width = height = 0;
#endif
}

void Window::EnableDragDrop() {
#ifdef _WIN32
    DragAcceptFiles(m_hwnd, TRUE);
#endif
}

#ifdef _WIN32
LRESULT CALLBACK Window::StaticWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    Window* window = nullptr;
    if (msg == WM_NCCREATE) {
        CREATESTRUCT* cs = reinterpret_cast<CREATESTRUCT*>(lParam);
        window = reinterpret_cast<Window*>(cs->lpCreateParams);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(window));
    } else {
        window = reinterpret_cast<Window*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    }
    
    if (window) {
        return window->WndProc(hwnd, msg, wParam, lParam);
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}

LRESULT Window::WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
        case WM_HOTKEY:
            if (m_hotkeyCallback) {
                m_hotkeyCallback(static_cast<int>(wParam));
            }
            return 0;
            
        case WM_DROPFILES: {
            HDROP hDrop = reinterpret_cast<HDROP>(wParam);
            if (m_dropFilesCallback) {
                std::vector<std::wstring> files;
                UINT count = DragQueryFileW(hDrop, 0xFFFFFFFF, nullptr, 0);
                for (UINT i = 0; i < count; i++) {
                    wchar_t buffer[MAX_PATH];
                    DragQueryFileW(hDrop, i, buffer, MAX_PATH);
                    files.push_back(buffer);
                }
                m_dropFilesCallback(files);
            }
            DragFinish(hDrop);
            return 0;
        }
        
        case WM_SIZE:
            if (m_resizeCallback && wParam != SIZE_MINIMIZED) {
                m_resizeCallback(LOWORD(lParam), HIWORD(lParam));
            }
            return 0;
            
        case WM_MOVE:
            if (m_moveCallback) {
                m_moveCallback(LOWORD(lParam), HIWORD(lParam));
            }
            return 0;
        
        case WM_TRAYICON:
            if (m_trayCallback) {
                m_trayCallback(wParam, lParam);
            }
            return 0;
            
        case WM_CLOSE:
            Hide();
            return 0;
            
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}
#endif

} // namespace mn