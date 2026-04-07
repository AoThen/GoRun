#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>

namespace mn {

class IconTextureManager;

class ItemGrid {
public:
    void SetItems(std::vector<Item>* items);
    void SetIconTextureManager(IconTextureManager* manager);
    void Render();
    
    void OnItemClicked(std::function<void(const Item&)> callback);
    void OnItemRunAsAdmin(std::function<void(const Item&)> callback);
    void OnItemEdit(std::function<void(Item&)> callback);
    void OnItemDelete(std::function<void(const Item&)> callback);

private:
    void RenderContextMenu(const Item& item);
    
    std::vector<Item>* m_items = nullptr;
    IconTextureManager* m_iconTextureManager = nullptr;
    int m_selectedIndex = -1;
    
    std::function<void(const Item&)> m_onClick;
    std::function<void(const Item&)> m_onRunAsAdmin;
    std::function<void(Item&)> m_onEdit;
    std::function<void(const Item&)> m_onDelete;
};

} // namespace mn