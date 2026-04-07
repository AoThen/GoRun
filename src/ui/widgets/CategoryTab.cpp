#include "CategoryTab.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void CategoryTab::SetCategories(std::vector<Category>* categories) {
    m_categories = categories;
    if (categories && !categories->empty() && m_currentId.empty()) {
        m_currentId = (*categories)[0].id;
    }
}

void CategoryTab::Render() {
    if (!m_categories) return;
    
    ImGui::BeginChild("Categories", ImVec2(150, 0), true);
    
    for (const auto& cat : *m_categories) {
        bool selected = (cat.id == m_currentId);
        std::string name = StringUtils::WStringToUtf8(cat.name);
        
        if (ImGui::Selectable(name.c_str(), selected)) {
            m_currentId = cat.id;
            if (m_onChanged) {
                m_onChanged(cat.id);
            }
        }
    }
    
    ImGui::EndChild();
}

void CategoryTab::OnCategoryChanged(std::function<void(const std::wstring&)> callback) {
    m_onChanged = callback;
}

void CategoryTab::SetCurrentCategory(const std::wstring& id) {
    m_currentId = id;
}

std::wstring CategoryTab::GetCurrentCategory() const {
    return m_currentId;
}

} // namespace mn