#include "IconTextureManager.h"
#include "platform/D3D11Renderer.h"
#include "IconCache.h"
#include "utils/PathUtils.h"

namespace mn {

void IconTextureManager::Initialize(D3D11Renderer* renderer, IconCache* iconCache) {
    m_renderer = renderer;
    m_iconCache = iconCache;
}

void* IconTextureManager::GetIconTexture(const Item& item) {
    // 检查缓存
    auto it = m_textureCache.find(item.id);
    if (it != m_textureCache.end()) {
        return it->second;
    }
    
    if (!m_renderer || !m_iconCache) return nullptr;
    
    // 获取缓存图标路径
    std::wstring iconPath = m_iconCache->GetIconPath(item);
    
    if (iconPath.empty() || !PathUtils::Exists(iconPath)) {
        return nullptr;
    }
    
    // 加载纹理
    void* texture = m_renderer->LoadTexture(iconPath);
    if (texture) {
        m_textureCache[item.id] = texture;
    }
    
    return texture;
}

void IconTextureManager::ClearCache() {
    m_textureCache.clear();
}

void IconTextureManager::RefreshIcon(const Item& item) {
    // 移除内存中的纹理缓存
    m_textureCache.erase(item.id);
    
    // 删除磁盘上的缓存文件并重新提取
    if (m_iconCache) {
        // 获取旧缓存路径，释放 D3D11 中的旧纹理
        std::wstring oldPath = m_iconCache->GetCachePath(item.id);
        if (m_renderer) {
            m_renderer->UnloadTextureByPath(oldPath);
        }
        m_iconCache->DeleteCache(item.id);
        m_iconCache->RefreshIcon(item);
    }
}

} // namespace mn
