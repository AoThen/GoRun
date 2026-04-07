#pragma once

#include <memory>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class Storage;
class IconCache;
class ItemManager;
class Config;
class HotkeyManager;
class Runner;
class Window;
class D3D11Renderer;
class MainWindow;

class App {
public:
#ifdef _WIN32
    bool Initialize(HINSTANCE hInstance);
#endif
    ~App();
    
    int Run();
    void Shutdown();
    
    static App* Get();
    ItemManager* GetItemManager();
    Config* GetConfig();
    MainWindow* GetMainWindow();

private:
    void HandleHotkey(int id);
    void SaveWindowPosition();
    
    static App* s_instance;
    
    std::unique_ptr<Storage> m_storage;
    std::unique_ptr<IconCache> m_iconCache;
    std::unique_ptr<ItemManager> m_itemManager;
    std::unique_ptr<Config> m_config;
    std::unique_ptr<HotkeyManager> m_hotkeyManager;
    std::unique_ptr<Runner> m_runner;
    std::unique_ptr<Window> m_window;
    std::unique_ptr<D3D11Renderer> m_renderer;
    std::unique_ptr<MainWindow> m_mainWindow;
};

} // namespace mn
