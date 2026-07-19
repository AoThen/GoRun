#pragma once

#include <string>
#include <unordered_map>
#include <imgui.h>
#include "Types.h"

namespace mn {

class IconTextureManager {
public:
    void Initialize(class D3D11Renderer* renderer, class IconCache* iconCache);
    
    // 获取图标纹理，返回 ImGui 可用的纹理指针
    ImTextureID GetIconTexture(const Item& item);
    
    // 清除缓存
    void ClearCache();
    
    // 刷新指定项目的图标缓存
    void RefreshIcon(const Item& item);

private:
    D3D11Renderer* m_renderer = nullptr;
    IconCache* m_iconCache = nullptr;
    std::unordered_map<std::wstring, ImTextureID> m_textureCache;
};

} // namespace mn
