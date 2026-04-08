#pragma once

#include <string>
#include <unordered_map>
#include "Types.h"

namespace mn {

class IconTextureManager {
public:
    void Initialize(class D3D11Renderer* renderer, class IconCache* iconCache);
    
    // 获取图标纹理，返回 ImGui 可用的纹理指针
    void* GetIconTexture(const Item& item);
    
    // 清除缓存
    void ClearCache();

private:
    D3D11Renderer* m_renderer = nullptr;
    IconCache* m_iconCache = nullptr;
    std::unordered_map<std::wstring, void*> m_textureCache;
};

} // namespace mn
