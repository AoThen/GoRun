#include "app/App.h"

#ifdef _WIN32
#include <Windows.h>

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
