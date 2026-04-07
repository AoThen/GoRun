#include "IconTextureManager.h"
#include "platform/D3D11Renderer.h"
#include "IconCache.h"
#include "utils/PathUtils.h"

namespace mn {

void IconTextureManager::Initialize(D3D11Renderer* renderer, IconCache* iconCache) {
    m_renderer = renderer;
    m_iconCache = iconCache;
}

void* IconTextureManager::GetIconTexture(const std::wstring& itemId, const std::wstring& target) {
    // 检查缓存
    auto it = m_textureCache.find(itemId);
    if (it != m_textureCache.end()) {
        return it->second;
    }
    
    if (!m_renderer || !m_iconCache) return nullptr;
    
    // 构建缓存图标路径
    Item tempItem;
    tempItem.id = itemId;
    tempItem.target = target;
    
    std::wstring iconPath = m_iconCache->GetIconPath(tempItem);
    
    if (iconPath.empty() || !PathUtils::Exists(iconPath)) {
        return nullptr;
    }
    
    // 加载纹理
    void* texture = m_renderer->LoadTexture(iconPath);
    if (texture) {
        m_textureCache[itemId] = texture;
    }
    
    return texture;
}

void IconTextureManager::ClearCache() {
    m_textureCache.clear();
}

} // namespace mn
