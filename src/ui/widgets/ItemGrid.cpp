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
    
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(16, 16));
    ImGui::BeginChild("Items", ImVec2(0, 0), false);
    
    // 计算列数（每项宽度约 90px）
    float windowWidth = ImGui::GetContentRegionAvail().x;
    int columns = std::max(1, (int)(windowWidth / 95));
    
    int index = 0;
    for (const auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        // 项目容器
        ImGui::BeginGroup();
        
        // 图标按钮样式
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0, 0, 0, 0));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_ButtonActive, ImVec4(0.91f, 0.92f, 0.95f, 1.0f));
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 6.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(0, 0));
        
        // 图标按钮（64x64）
        ImVec2 iconSize(64, 64);
        if (ImGui::Button("##icon", iconSize)) {
            m_selectedIndex = index;
            if (m_onClick) {
                m_onClick(item);
            }
        }
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(3);
        
        // 图标占位符（显示首字符）
        ImVec2 min = ImGui::GetItemRectMin();
        ImVec2 max = ImGui::GetItemRectMax();
        ImVec2 center((min.x + max.x) / 2, (min.y + max.y) / 2);
        
        // 在图标区域绘制首字符
        ImDrawList* drawList = ImGui::GetWindowDrawList();
        if (!name.empty()) {
            char firstChar[8] = {};
            int charLen = 0;
            // 获取第一个字符
            for (int i = 0; i < name.size() && charLen < 4; i++) {
                firstChar[charLen++] = name[i];
                if ((name[i] & 0xC0) != 0x80) break; // UTF-8 第一个字符结束
            }
            firstChar[charLen] = '\0';
            
            // 绘制首字符（使用大号字体）
            ImGui::SetCursorScreenPos(ImVec2(center.x - 14, center.y - 14));
            ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.3f, 0.3f, 0.3f, 1.0f));
            ImGui::TextUnformatted(firstChar);
            ImGui::PopStyleColor();
        }
        
        // 名称标签
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() - iconSize.x / 2 + 32);
        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.33f, 0.33f, 0.33f, 1.0f));
        ImGui::SetNextItemWidth(80);
        
        // 截断过长名称
        std::string displayName = name;
        if (displayName.length() > 8) {
            displayName = displayName.substr(0, 7) + "..";
        }
        
        ImGui::TextWrapped("%s", displayName.c_str());
        ImGui::PopStyleColor();
        
        ImGui::EndGroup();
        
        // 换行或同行排列
        if ((index + 1) % columns != 0) {
            ImGui::SameLine(0, 16);
        }
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
    ImGui::PopStyleVar();
}

void ItemGrid::OnItemClicked(std::function<void(const Item&)> callback) {
    m_onClick = callback;
}

void ItemGrid::OnItemRightClicked(std::function<void(const Item&)> callback) {
    m_onRightClick = callback;
}

} // namespace mn
