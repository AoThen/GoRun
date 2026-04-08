#include "App.h"
#include "Resource.h"
#include "core/Storage.h"
#include "core/IconCache.h"
#include "core/IconTextureManager.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/HotkeyManager.h"
#include "core/Runner.h"
#include "platform/Window.h"
#include "platform/D3D11Renderer.h"
#include "platform/TrayIcon.h"
#include "ui/MainWindow.h"
#include "utils/PathUtils.h"
#include <imgui.h>

namespace mn {

App::App() = default;

App::~App() {
    // 显式定义析构函数，确保 unique_ptr 在此处销毁
    // 此时 TrayIcon 等类型已是完整类型
}

App* App::s_instance = nullptr;

App* App::Get() {
    return s_instance;
}

#ifdef _WIN32

bool App::Initialize(HINSTANCE hInstance) {
    s_instance = this;
    m_hInstance = hInstance;
    
    // 设置 DPI 感知，确保在高 DPI 显示器上字体清晰
    // 使用 Per-Monitor DPI Awareness V2（Windows 10 1703+）
    typedef HRESULT(WINAPI* SetProcessDpiAwarenessContext_t)(DPI_AWARENESS_CONTEXT);
    HMODULE hUser32 = GetModuleHandleW(L"user32.dll");
    if (hUser32) {
        auto pSetProcessDpiAwarenessContext = 
            (SetProcessDpiAwarenessContext_t)GetProcAddress(hUser32, "SetProcessDpiAwarenessContext");
        if (pSetProcessDpiAwarenessContext) {
            pSetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }
    
    // 单实例检查
    HANDLE hMutex = CreateMutexW(nullptr, TRUE, L"GoRun_SingleInstance_Mutex");
    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        // 已有实例运行，尝试激活现有窗口
        HWND hwndExisting = FindWindowW(L"GoRunWindowClass", nullptr);
        if (hwndExisting) {
            ShowWindow(hwndExisting, SW_SHOW);
            SetForegroundWindow(hwndExisting);
        }
        return false;
    }
    m_hMutex = hMutex;
    
    m_storage = std::make_unique<Storage>();
    m_iconCache = std::make_unique<IconCache>();
    m_iconTextureManager = std::make_unique<IconTextureManager>();
    m_itemManager = std::make_unique<ItemManager>();
    m_config = std::make_unique<Config>();
    m_hotkeyManager = std::make_unique<HotkeyManager>();
    m_runner = std::make_unique<Runner>();
    m_window = std::make_unique<Window>();
    m_renderer = std::make_unique<D3D11Renderer>();
    m_mainWindow = std::make_unique<MainWindow>();
    m_trayIcon = std::make_unique<TrayIcon>();
    
    std::wstring appDataPath = PathUtils::GetAppDataPath();
    PathUtils::EnsureDirectory(appDataPath);
    
    std::wstring iconsPath = appDataPath + L"\\icons";
    PathUtils::EnsureDirectory(iconsPath);
    
    m_iconCache->Initialize(iconsPath);
    
    std::wstring configPath = appDataPath + L"\\config.json";
    m_storage->Initialize(configPath);
    if (PathUtils::Exists(configPath)) {
        m_storage->Load(configPath);
    }
    
    m_config->Initialize(m_storage.get());
    m_itemManager->Initialize(m_storage.get(), m_iconCache.get());
    
    int x = m_config->GetWindowX();
    int y = m_config->GetWindowY();
    int width = m_config->GetWindowWidth();
    int height = m_config->GetWindowHeight();
    
    if (!m_window->Create(L"GoRun", width, height, x, y)) {
        return false;
    }
    
    if (!m_renderer->Initialize(m_window->GetHandle(), width, height)) {
        return false;
    }
    
    // 初始化图标纹理管理器
    m_iconTextureManager->Initialize(m_renderer.get(), m_iconCache.get());
    
    m_window->EnableDragDrop();
    
    m_window->OnDropFiles([this](const std::vector<std::wstring>& files) {
        if (m_mainWindow->IsVisible() && !m_itemManager->GetCategories().empty()) {
            m_itemManager->HandleDrop(files, m_itemManager->GetCategories()[0].id);
        }
    });
    
    m_window->OnResize([this](int w, int h) {
        m_renderer->Resize(w, h);
        m_config->SetWindowSize(w, h);
    });
    
    m_window->OnMove([this](int x, int y) {
        m_config->SetWindowPosition(x, y);
    });
    
    m_window->OnDpiChanged([this](int dpi) {
        float newScale = dpi / 96.0f;
        m_renderer->UpdateDpiScale(newScale);
    });
    
    m_hotkeyManager->SetCallback([this](int id) {
        HandleHotkey(id);
    });
    
    std::wstring hotkey = m_config->GetGlobalHotkey();
    unsigned int modifiers, vk;
    if (m_hotkeyManager->ParseHotkeyString(hotkey, modifiers, vk)) {
        m_hotkeyManager->RegisterGlobalHotkey(1, modifiers, vk);
    }
    
    m_mainWindow->Initialize(m_itemManager.get(), m_config.get(), m_runner.get(), m_iconTextureManager.get());
    
    m_trayIcon->Create(m_window->GetHandle(), WM_TRAYICON, L"GoRun");
    m_trayIcon->OnShow([this]() {
        ToggleWindow();
    });
    m_trayIcon->OnExit([this]() {
        Quit();
    });
    
    m_window->OnTrayMessage([this](WPARAM wParam, LPARAM lParam) {
        m_trayIcon->HandleMessage(wParam, lParam);
    });
    
    m_window->Show();
    m_mainWindow->Show();
    
    return true;
}
#endif

int App::Run() {
#ifdef _WIN32
    MSG msg = {};
    
    while (msg.message != WM_QUIT) {
        if (PeekMessage(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
            
            if (msg.message == WM_HOTKEY) {
                m_hotkeyManager->ProcessHotkey(static_cast<int>(msg.wParam));
            }
        } else {
            m_renderer->NewFrame();
            
            if (m_mainWindow->IsVisible()) {
                ImGui::SetNextWindowPos(ImVec2(0, 0));
                ImGui::SetNextWindowSize(ImGui::GetIO().DisplaySize);
                ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 0.0f);
                ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f);
                ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(0, 0));
                ImGui::Begin("GoRun", nullptr, 
                    ImGuiWindowFlags_NoResize | 
                    ImGuiWindowFlags_NoMove |
                    ImGuiWindowFlags_NoCollapse |
                    ImGuiWindowFlags_MenuBar);
                ImGui::PopStyleVar(3);
                
                m_mainWindow->Render();
                
                ImGui::End();
            }
            
            m_renderer->Render();
        }
    }
    
    return static_cast<int>(msg.wParam);
#else
    return 0;
#endif
}

void App::Shutdown() {
    SaveWindowPosition();
    
    m_trayIcon->Destroy();
    m_renderer->Shutdown();
    m_hotkeyManager->UnregisterGlobalHotkey(1);
    
    // 释放单实例互斥体
    if (m_hMutex) {
        ReleaseMutex(m_hMutex);
        CloseHandle(m_hMutex);
        m_hMutex = nullptr;
    }
}

void App::Quit() {
#ifdef _WIN32
    PostQuitMessage(0);
#endif
}

ItemManager* App::GetItemManager() {
    return m_itemManager.get();
}

Config* App::GetConfig() {
    return m_config.get();
}

MainWindow* App::GetMainWindow() {
    return m_mainWindow.get();
}

IconTextureManager* App::GetIconTextureManager() {
    return m_iconTextureManager.get();
}

void App::HandleHotkey(int id) {
    if (id == 1) {
        ToggleWindow();
    }
}

void App::ToggleWindow() {
    if (m_window->IsVisible()) {
        m_window->Hide();
        m_mainWindow->Hide();
    } else {
        m_window->Show();
        m_mainWindow->Show();
    }
}

void App::SaveWindowPosition() {
    int x, y, w, h;
    m_window->GetPosition(x, y);
    m_window->GetSize(w, h);
    m_config->SetWindowPosition(x, y);
    m_config->SetWindowSize(w, h);
}

} // namespace mn