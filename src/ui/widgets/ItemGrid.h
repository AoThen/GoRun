#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>
#include <unordered_map>

namespace mn {

class IconTextureManager;

class ItemGrid {
public:
    void SetItems(const std::vector<Item>* items);
    void SetIconTextureManager(IconTextureManager* manager);
    void SetViewType(ViewType viewType);
    void SetSearchQuery(const std::wstring& query);
    void Render();
    void ClearHoverAnimation(const std::wstring& itemId);  // 清理指定项目的动画状态
    
    void OnItemClicked(std::function<void(const Item&)> callback);
    void OnItemRunAsAdmin(std::function<void(const Item&)> callback);
    void OnItemEdit(std::function<void(Item&)> callback);
    void OnItemDelete(std::function<void(const Item&)> callback);
    void OnItemRefreshIcon(std::function<void(const Item&)> callback);
    void OnItemAdd(std::function<void()> callback);
    void OnItemCopyPath(std::function<void(const Item&)> callback);
    void OnItemOpenLocation(std::function<void(const Item&)> callback);
    void OnItemMoveToCategory(std::function<void(const Item&, const std::wstring&)> callback);
    void OnItemProperties(std::function<void(const Item&)> callback);
    void SetAllCategories(const std::vector<Category>* categories);

private:
    void RenderIconView();
    void RenderListView();
    void RenderContextMenu(Item& item);
    
    // 悬停动画辅助
    float GetHoverScale(const std::wstring& itemId, bool isHovered);
    
    const std::vector<Item>* m_items = nullptr;
    std::vector<Item> m_sortedItems;
    IconTextureManager* m_iconTextureManager = nullptr;
    int m_selectedIndex = -1;
    ViewType m_viewType = ViewType::Icon;
    bool m_sortDirty = false;
    
    // 悬停动画状态
    std::unordered_map<std::wstring, float> m_hoverAnimState;
    
    std::function<void(const Item&)> m_onClick;
    std::function<void(const Item&)> m_onRunAsAdmin;
    std::function<void(Item&)> m_onEdit;
    std::function<void(const Item&)> m_onDelete;
    std::function<void(const Item&)> m_onRefreshIcon;
    std::function<void()> m_onAddItem;
    std::function<void(const Item&)> m_onCopyPath;
    std::function<void(const Item&)> m_onOpenLocation;
    std::function<void(const Item&, const std::wstring&)> m_onMoveToCategory;
    std::function<void(const Item&)> m_onProperties;
    const std::vector<Category>* m_allCategories = nullptr;
    
    // 搜索高亮
    std::wstring m_searchQuery;
};

} // namespace mn