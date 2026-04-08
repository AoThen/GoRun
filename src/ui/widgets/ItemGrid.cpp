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

void ItemGrid::SetViewType(ViewType viewType) {
    m_viewType = viewType;
}

void ItemGrid::Render() {
    if (m_viewType == ViewType::Icon) {
        RenderIconView();
    } else {
        RenderListView();
    }
}

void ItemGrid::RenderIconView() {
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(16, 16));
    ImGui::BeginChild("Items", ImVec2(0, 0), false);
    
    if (!m_items || m_items->empty()) {
        // 空结果提示
        ImGui::TextColored(ImVec4(0.5f, 0.5f, 0.5f, 1.0f), "暂无项目");
        ImGui::EndChild();
        ImGui::PopStyleVar();
        return;
    }
    
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
        
        // 右键菜单 - 使用 ImGuiPopupFlags_MouseButtonRight 确保右键触发
        if (ImGui::BeginPopupContextItem("item_context", ImGuiPopupFlags_MouseButtonRight)) {
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

void ItemGrid::RenderListView() {
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(8, 8));
    ImGui::BeginChild("Items", ImVec2(0, 0), false);
    
    if (!m_items || m_items->empty()) {
        ImGui::TextColored(ImVec4(0.5f, 0.5f, 0.5f, 1.0f), "暂无项目");
        ImGui::EndChild();
        ImGui::PopStyleVar();
        return;
    }
    
    int index = 0;
    for (auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        ImGui::BeginGroup();
        
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0, 0, 0, 0));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_ButtonActive, ImVec4(0.91f, 0.92f, 0.95f, 1.0f));
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 4.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(4, 4));
        
        // 列表项：小图标 + 名称
        ImVec2 iconSize(24, 24);
        
        void* iconTexture = nullptr;
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item.id, item.target);
        }
        
        // 整行可点击
        float itemWidth = ImGui::GetContentRegionAvail().x;
        if (ImGui::Button("##item", ImVec2(itemWidth, 32))) {
            m_selectedIndex = index;
            if (m_onClick) {
                m_onClick(item);
            }
        }
        
        // 右键菜单
        if (ImGui::BeginPopupContextItem("item_context", ImGuiPopupFlags_MouseButtonRight)) {
            RenderContextMenu(item);
            ImGui::EndPopup();
        }
        
        // 在按钮上绘制图标和文本
        ImVec2 min = ImGui::GetItemRectMin();
        ImGui::SetCursorScreenPos(ImVec2(min.x + 4, min.y + 4));
        
        if (iconTexture) {
            ImGui::Image(iconTexture, iconSize);
            ImGui::SameLine(0, 8);
        } else {
            // 占位符
            ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
            ImGui::Button("##placeholder", iconSize);
            ImGui::PopStyleColor();
            ImGui::SameLine(0, 8);
        }
        
        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.33f, 0.33f, 0.33f, 1.0f));
        ImGui::Text("%s", name.c_str());
        ImGui::PopStyleColor();
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(3);
        
        ImGui::EndGroup();
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
    ImGui::PopStyleVar();
}

void ItemGrid::RenderContextMenu(const Item& item) {
    if (ImGui::MenuItem("运行")) {
        if (m_onClick) {
            m_onClick(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem("以管理员运行")) {
        if (m_onRunAsAdmin) {
            m_onRunAsAdmin(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem("编辑")) {
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
    
    if (ImGui::MenuItem("删除")) {
        if (m_onDelete) {
            m_onDelete(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem("图标视图", nullptr, m_viewType == ViewType::Icon)) {
        SetViewType(ViewType::Icon);
    }
    
    if (ImGui::MenuItem("列表视图", nullptr, m_viewType == ViewType::List)) {
        SetViewType(ViewType::List);
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
