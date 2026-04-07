#include "ItemGrid.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void ItemGrid::SetItems(std::vector<Item>* items) {
    m_items = items;
    m_selectedIndex = -1;
}

void ItemGrid::Render() {
    if (!m_items) return;
    
    ImGui::BeginChild("Items", ImVec2(0, 0), true);
    
    int index = 0;
    for (const auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        if (ImGui::Button(name.c_str(), ImVec2(80, 80))) {
            m_selectedIndex = index;
        }
        
        if (ImGui::IsItemHovered() && ImGui::IsMouseDoubleClicked(0)) {
            if (m_onDoubleClick) {
                m_onDoubleClick(item);
            }
        }
        
        if ((index + 1) % 4 != 0) {
            ImGui::SameLine();
        }
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
}

void ItemGrid::OnItemDoubleClicked(std::function<void(const Item&)> callback) {
    m_onDoubleClick = callback;
}

void ItemGrid::OnItemRightClicked(std::function<void(const Item&)> callback) {
    m_onRightClick = callback;
}

} // namespace mn
