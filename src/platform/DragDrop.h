#pragma once

#include <string>
#include <vector>

namespace mn {

class DragDrop {
public:
    void Initialize(void* hwnd);
    std::vector<std::wstring> GetDroppedFiles(void* hDrop);
};

} // namespace mn