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
    
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(8, 8));
    ImGui::PushStyleColor(ImGuiCol_ChildBg, ImVec4(0.961f, 0.961f, 0.961f, 1.0f)); // #F5F5F5
    
    ImGui::BeginChild("Categories", ImVec2(150, 0), false);
    
    for (const auto& cat : *m_categories) {
        bool selected = (cat.id == m_currentId);
        std::string name = StringUtils::WStringToUtf8(cat.name);
        
        // 自定义选中样式
        if (selected) {
            ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderHovered, ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderActive, ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
        } else {
            ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0, 0, 0, 0));
            ImGui::PushStyleColor(ImGuiCol_HeaderHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderActive, ImVec4(0.91f, 0.92f, 0.95f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.4f, 0.4f, 0.4f, 1.0f));
        }
        
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(12, 10));
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 4.0f);
        
        if (ImGui::Selectable(name.c_str(), selected, ImGuiSelectableFlags_None, ImVec2(0, 0))) {
            m_currentId = cat.id;
            if (m_onChanged) {
                m_onChanged(cat.id);
            }
        }
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(4);
    }
    
    ImGui::EndChild();
    
    ImGui::PopStyleColor();
    ImGui::PopStyleVar();
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