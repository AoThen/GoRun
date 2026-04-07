#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>

namespace mn {

class ItemGrid {
public:
    void SetItems(std::vector<Item>* items);
    void Render();
    
    void OnItemDoubleClicked(std::function<void(const Item&)> callback);
    void OnItemRightClicked(std::function<void(const Item&)> callback);

private:
    std::vector<Item>* m_items = nullptr;
    int m_selectedIndex = -1;
    std::function<void(const Item&)> m_onDoubleClick;
    std::function<void(const Item&)> m_onRightClick;
};

} // namespace mn