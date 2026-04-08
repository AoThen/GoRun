#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>
#include <unordered_map>

namespace mn {

class IconTextureManager;

class ItemGrid {
public:
    void SetItems(std::vector<Item>* items);
    void SetIconTextureManager(IconTextureManager* manager);
    void SetViewType(ViewType viewType);
    ViewType GetViewType() const { return m_viewType; }
    void Render();
    void ClearHoverAnimation(const std::wstring& itemId);  // 清理指定项目的动画状态
    
    void OnItemClicked(std::function<void(const Item&)> callback);
    void OnItemRunAsAdmin(std::function<void(const Item&)> callback);
    void OnItemEdit(std::function<void(Item&)> callback);
    void OnItemDelete(std::function<void(const Item&)> callback);
    void OnItemRefreshIcon(std::function<void(const Item&)> callback);

private:
    void RenderIconView();
    void RenderListView();
    void RenderContextMenu(Item& item);
    
    // 悬停动画辅助
    float GetHoverScale(const std::wstring& itemId, bool isHovered);
    
    std::vector<Item>* m_items = nullptr;
    IconTextureManager* m_iconTextureManager = nullptr;
    int m_selectedIndex = -1;
    ViewType m_viewType = ViewType::Icon;
    
    // 悬停动画状态
    std::unordered_map<std::wstring, float> m_hoverAnimState;
    float m_lastTime = 0.0f;
    
    std::function<void(const Item&)> m_onClick;
    std::function<void(const Item&)> m_onRunAsAdmin;
    std::function<void(Item&)> m_onEdit;
    std::function<void(const Item&)> m_onDelete;
    std::function<void(const Item&)> m_onRefreshIcon;
};

} // namespace mn