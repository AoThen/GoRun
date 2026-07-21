#include "app/App.h"
#include "core/Storage.h"
#include "core/IconCache.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/HotkeyManager.h"
#include "core/Runner.h"
#include "platform/Window.h"
#include "platform/D3D11Renderer.h"
#include "ui/MainWindow.h"

#ifdef _WIN32
#include <Windows.h>

#pragma comment(linker, "/SUBSYSTEM:WINDOWS")

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE, LPSTR, int) {
    mn::App app;
    
    if (!app.Initialize(hInstance)) {
        return -1;
    }
    
    int result = app.Run();
    app.Shutdown();
    
    return result;
}
#else
int main() {
    return 0;
}
#endif
