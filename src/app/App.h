#pragma once

#include <memory>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class Storage;
class IconCache;
class IconTextureManager;
class ItemManager;
class Config;
class HotkeyManager;
class Runner;
class Window;
class D3D11Renderer;
class MainWindow;
class TrayIcon;

class App {
public:
    App();  // 显式声明构造函数
#ifdef _WIN32
    bool Initialize(HINSTANCE hInstance);
    HINSTANCE GetInstance() const { return m_hInstance; }
#endif
    ~App();
    
    int Run();
    void Shutdown();
    void Quit();
    
    static App* Get();
    ItemManager* GetItemManager();
    Config* GetConfig();
    MainWindow* GetMainWindow();
    IconTextureManager* GetIconTextureManager();

private:
    void HandleHotkey(int id);
    void SaveWindowPosition();
    void ToggleWindow();
    
    static App* s_instance;
    
#ifdef _WIN32
    HINSTANCE m_hInstance = nullptr;
    HANDLE m_hMutex = nullptr;
#endif
    
    std::unique_ptr<Storage> m_storage;
    std::unique_ptr<IconCache> m_iconCache;
    std::unique_ptr<IconTextureManager> m_iconTextureManager;
    std::unique_ptr<ItemManager> m_itemManager;
    std::unique_ptr<Config> m_config;
    std::unique_ptr<HotkeyManager> m_hotkeyManager;
    std::unique_ptr<Runner> m_runner;
    std::unique_ptr<Window> m_window;
    std::unique_ptr<D3D11Renderer> m_renderer;
    std::unique_ptr<MainWindow> m_mainWindow;
    std::unique_ptr<TrayIcon> m_trayIcon;
};

} // namespace mn
