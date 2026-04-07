#include "ItemGrid.h"
#include "core/IconTextureManager.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void ItemGrid::SetItems(std::vector<Item>* items) {
    m_items = items;
    m_selectedIndex = -1;
}

void ItemGrid::SetIconTextureManager(IconTextureManager* manager) {
    m_iconTextureManager = manager;
}

void ItemGrid::Render() {
    if (!m_items) return;
    
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(16, 16));
    ImGui::BeginChild("Items", ImVec2(0, 0), false);
    
    float windowWidth = ImGui::GetContentRegionAvail().x;
    int columns = std::max(1, (int)(windowWidth / 95));
    
    int index = 0;
    for (auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        ImGui::BeginGroup();
        
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0, 0, 0, 0));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_ButtonActive, ImVec4(0.91f, 0.92f, 0.95f, 1.0f));
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 6.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(0, 0));
        
        ImVec2 iconSize(64, 64);
        
        // 尝试加载图标纹理
        void* iconTexture = nullptr;
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item.id, item.target);
        }
        
        if (iconTexture) {
            // 显示图标纹理
            ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(1, 1, 1, 1));
            if (ImGui::ImageButton("##icon", iconTexture, iconSize)) {
                m_selectedIndex = index;
                if (m_onClick) {
                    m_onClick(item);
                }
            }
            ImGui::PopStyleColor();
        } else {
            // 显示占位符
            if (ImGui::Button("##icon", iconSize)) {
                m_selectedIndex = index;
                if (m_onClick) {
                    m_onClick(item);
                }
            }
            
            // 显示首字符
            ImVec2 min = ImGui::GetItemRectMin();
            ImVec2 max = ImGui::GetItemRectMax();
            ImVec2 center((min.x + max.x) / 2, (min.y + max.y) / 2);
            
            if (!name.empty()) {
                char firstChar[8] = {};
                int charLen = 0;
                for (int i = 0; i < name.size() && charLen < 4; i++) {
                    firstChar[charLen++] = name[i];
                    if ((name[i] & 0xC0) != 0x80) break;
                }
                firstChar[charLen] = '\0';
                
                ImGui::SetCursorScreenPos(ImVec2(center.x - 14, center.y - 14));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.3f, 0.3f, 0.3f, 1.0f));
                ImGui::TextUnformatted(firstChar);
                ImGui::PopStyleColor();
            }
        }
        
        // 右键菜单
        if (ImGui::BeginPopupContextItem("item_context")) {
            RenderContextMenu(item);
            ImGui::EndPopup();
        }
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(3);
        
        // 名称标签
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() - iconSize.x / 2 + 32);
        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.33f, 0.33f, 0.33f, 1.0f));
        
        std::string displayName = name;
        if (displayName.length() > 8) {
            displayName = displayName.substr(0, 7) + "..";
        }
        
        ImGui::TextWrapped("%s", displayName.c_str());
        ImGui::PopStyleColor();
        
        ImGui::EndGroup();
        
        if ((index + 1) % columns != 0) {
            ImGui::SameLine(0, 16);
        }
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
    ImGui::PopStyleVar();
}

void ItemGrid::RenderContextMenu(const Item& item) {
    if (ImGui::MenuItem(u8"运行")) {
        if (m_onClick) {
            m_onClick(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem(u8"以管理员运行")) {
        if (m_onRunAsAdmin) {
            m_onRunAsAdmin(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem(u8"编辑")) {
        if (m_onEdit) {
            for (auto& i : *m_items) {
                if (i.id == item.id) {
                    m_onEdit(i);
                    break;
                }
            }
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem(u8"删除")) {
        if (m_onDelete) {
            m_onDelete(item);
        }
        ImGui::CloseCurrentPopup();
    }
}

void ItemGrid::OnItemClicked(std::function<void(const Item&)> callback) {
    m_onClick = callback;
}

void ItemGrid::OnItemRunAsAdmin(std::function<void(const Item&)> callback) {
    m_onRunAsAdmin = callback;
}

void ItemGrid::OnItemEdit(std::function<void(Item&)> callback) {
    m_onEdit = callback;
}

void ItemGrid::OnItemDelete(std::function<void(const Item&)> callback) {
    m_onDelete = callback;
}

} // namespace mn
