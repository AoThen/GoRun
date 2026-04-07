#include "App.h"
#include "core/Storage.h"
#include "core/IconCache.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/HotkeyManager.h"
#include "core/Runner.h"
#include "platform/Window.h"
#include "platform/D3D11Renderer.h"
#include "ui/MainWindow.h"
#include "utils/PathUtils.h"
#include <imgui.h>

namespace mn {

App* App::s_instance = nullptr;

App* App::Get() {
    return s_instance;
}

#ifdef _WIN32
bool App::Initialize(HINSTANCE hInstance) {
    s_instance = this;
    
    m_storage = std::make_unique<Storage>();
    m_iconCache = std::make_unique<IconCache>();
    m_itemManager = std::make_unique<ItemManager>();
    m_config = std::make_unique<Config>();
    m_hotkeyManager = std::make_unique<HotkeyManager>();
    m_runner = std::make_unique<Runner>();
    m_window = std::make_unique<Window>();
    m_renderer = std::make_unique<D3D11Renderer>();
    m_mainWindow = std::make_unique<MainWindow>();
    
    std::wstring appDataPath = PathUtils::GetAppDataPath();
    PathUtils::EnsureDirectory(appDataPath);
    
    std::wstring iconsPath = appDataPath + L"\\icons";
    PathUtils::EnsureDirectory(iconsPath);
    
    m_iconCache->Initialize(iconsPath);
    
    std::wstring configPath = appDataPath + L"\\config.json";
    if (PathUtils::Exists(configPath)) {
        m_storage->Load(configPath);
    }
    
    m_config->Initialize(m_storage.get());
    m_itemManager->Initialize(m_storage.get(), m_iconCache.get());
    
    int x = m_config->GetWindowX();
    int y = m_config->GetWindowY();
    int width = m_config->GetWindowWidth();
    int height = m_config->GetWindowHeight();
    
    if (!m_window->Create(L"Maye Nano", width, height, x, y)) {
        return false;
    }
    
    if (!m_renderer->Initialize(m_window->GetHandle(), width, height)) {
        return false;
    }
    
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
    
    m_hotkeyManager->SetCallback([this](int id) {
        HandleHotkey(id);
    });
    
    std::wstring hotkey = m_config->GetGlobalHotkey();
    unsigned int modifiers, vk;
    if (m_hotkeyManager->ParseHotkeyString(hotkey, modifiers, vk)) {
        m_hotkeyManager->RegisterGlobalHotkey(1, modifiers, vk);
    }
    
    m_mainWindow->Initialize(m_itemManager.get(), m_config.get(), m_runner.get());
    
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
                m_hotkeyManager->ProcessHotkey(msg.wParam);
            }
        } else {
            m_renderer->NewFrame();
            
            if (m_mainWindow->IsVisible()) {
                ImGui::SetNextWindowPos(ImVec2(0, 0));
                ImGui::SetNextWindowSize(ImGui::GetIO().DisplaySize);
                ImGui::Begin("Main", nullptr, 
                    ImGuiWindowFlags_NoTitleBar | 
                    ImGuiWindowFlags_NoResize | 
                    ImGuiWindowFlags_NoMove |
                    ImGuiWindowFlags_MenuBar);
                
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
    
    m_renderer->Shutdown();
    m_hotkeyManager->UnregisterGlobalHotkey(1);
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

void App::HandleHotkey(int id) {
    if (id == 1) {
        m_mainWindow->Toggle();
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
