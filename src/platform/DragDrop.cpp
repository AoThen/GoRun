#include "DragDrop.h"

#ifdef _WIN32
#include <Windows.h>
#include <ShellApi.h>
#endif

namespace mn {

void DragDrop::Initialize(void* hwnd) {
#ifdef _WIN32
    DragAcceptFiles((HWND)hwnd, TRUE);
#endif
}

std::vector<std::wstring> DragDrop::GetDroppedFiles(void* hDrop) {
    std::vector<std::wstring> files;
#ifdef _WIN32
    HDROP drop = (HDROP)hDrop;
    UINT count = DragQueryFileW(drop, 0, nullptr, 0);
    
    for (UINT i = 0; i < count; i++) {
        wchar_t buffer[MAX_PATH];
        DragQueryFileW(drop, i, buffer, MAX_PATH);
        files.push_back(buffer);
    }
    
    DragFinish(drop);
#endif
    return files;
}

} // namespace mn