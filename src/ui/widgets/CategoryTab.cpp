#include "CategoryTab.h"
#include "utils/StringUtils.h"
#include "ui/Theme.h"
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
    
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(8, 8));
    ImGui::PushStyleColor(ImGuiCol_ChildBg, isDark ? ImVec4(0.12f, 0.12f, 0.12f, 1.0f) : ImVec4(0.961f, 0.961f, 0.961f, 1.0f));
    
    ImGui::BeginChild("Categories", ImVec2(150, 0), false);
    
    for (size_t i = 0; i < m_categories->size(); i++) {
        auto& cat = (*m_categories)[i];
        bool selected = (cat.id == m_currentId);
        std::string name = StringUtils::WStringToUtf8(cat.name);
        
        ImGui::PushID(static_cast<int>(i));
        
        // 自定义选中样式
        if (selected) {
            ImGui::PushStyleColor(ImGuiCol_Header, isDark ? ImVec4(0.25f, 0.25f, 0.25f, 1.0f) : ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderHovered, isDark ? ImVec4(0.25f, 0.25f, 0.25f, 1.0f) : ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderActive, isDark ? ImVec4(0.3f, 0.3f, 0.3f, 1.0f) : ImVec4(0.902f, 0.914f, 0.941f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_Text, isDark ? ImVec4(0.9f, 0.9f, 0.9f, 1.0f) : ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
        } else {
            ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0, 0, 0, 0));
            ImGui::PushStyleColor(ImGuiCol_HeaderHovered, isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_HeaderActive, isDark ? ImVec4(0.22f, 0.22f, 0.22f, 1.0f) : ImVec4(0.91f, 0.92f, 0.95f, 1.0f));
            ImGui::PushStyleColor(ImGuiCol_Text, isDark ? ImVec4(0.7f, 0.7f, 0.7f, 1.0f) : ImVec4(0.4f, 0.4f, 0.4f, 1.0f));
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
        
        // 右键菜单（使用分类 ID 作为弹窗 ID 的一部分）
        std::string popupId = "category_context_" + StringUtils::WStringToUtf8(cat.id);
        if (ImGui::BeginPopupContextItem(popupId.c_str(), ImGuiPopupFlags_MouseButtonRight)) {
            RenderContextMenu(cat);
            ImGui::EndPopup();
        }
        
        ImGui::PopID();
    }
    
    // 底部添加分类按钮
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();
    
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(12, 8));
    if (ImGui::Button("+ 新建分类", ImVec2(-FLT_MIN, 0))) {
        if (m_onAdd) {
            m_onAdd();
        }
    }
    ImGui::PopStyleVar();
    
    ImGui::EndChild();
    
    ImGui::PopStyleColor();
    ImGui::PopStyleVar();
}

void CategoryTab::RenderContextMenu(const Category& cat) {
    if (ImGui::MenuItem("重命名")) {
        if (m_onRename) {
            m_onRename(cat.id);
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem("删除", nullptr, false, m_categories && m_categories->size() > 1)) {
        if (m_onDelete) {
            m_onDelete(cat.id);
        }
        ImGui::CloseCurrentPopup();
    }
}

void CategoryTab::OnCategoryChanged(std::function<void(const std::wstring&)> callback) {
    m_onChanged = callback;
}

void CategoryTab::OnCategoryAdd(std::function<void()> callback) {
    m_onAdd = callback;
}

void CategoryTab::OnCategoryDelete(std::function<void(const std::wstring&)> callback) {
    m_onDelete = callback;
}

void CategoryTab::OnCategoryRename(std::function<void(const std::wstring&)> callback) {
    m_onRename = callback;
}

void CategoryTab::SetCurrentCategory(const std::wstring& id) {
    m_currentId = id;
}

} // namespace mn