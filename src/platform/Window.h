#pragma once

#include <string>
#include <functional>
#include <vector>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class Window {
public:
    bool Create(const std::wstring& title, int width, int height, int x, int y);
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    
    void SetPosition(int x, int y);
    void SetSize(int width, int height);
    void GetPosition(int& x, int& y);
    void GetSize(int& width, int& height);
    
#ifdef _WIN32
    HWND GetHandle() const { return m_hwnd; }
#endif
    
    void EnableDragDrop();
    
    using DropFilesCallback = std::function<void(const std::vector<std::wstring>&)>;
    using HotkeyCallback = std::function<void(int id)>;
    using ResizeCallback = std::function<void(int w, int h)>;
    using MoveCallback = std::function<void(int x, int y)>;
    using TrayCallback = std::function<void(WPARAM wParam, LPARAM lParam)>;
    
    void OnDropFiles(DropFilesCallback cb) { m_dropFilesCallback = cb; }
    void OnHotkey(HotkeyCallback cb) { m_hotkeyCallback = cb; }
    void OnResize(ResizeCallback cb) { m_resizeCallback = cb; }
    void OnMove(MoveCallback cb) { m_moveCallback = cb; }
    void OnTrayMessage(TrayCallback cb) { m_trayCallback = cb; }

private:
#ifdef _WIN32
    static LRESULT CALLBACK StaticWndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    LRESULT WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    HWND m_hwnd = nullptr;
#endif
    bool m_visible = false;
    
    DropFilesCallback m_dropFilesCallback;
    HotkeyCallback m_hotkeyCallback;
    ResizeCallback m_resizeCallback;
    MoveCallback m_moveCallback;
    TrayCallback m_trayCallback;
};

} // namespace mn
