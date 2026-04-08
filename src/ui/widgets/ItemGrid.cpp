#include "ItemGrid.h"
#include "core/IconTextureManager.h"
#include "utils/StringUtils.h"
#include "ui/Theme.h"
#include <imgui.h>
#include <cmath>

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

float ItemGrid::GetHoverScale(const std::wstring& itemId, bool isHovered) {
    // 获取当前时间
    float time = ImGui::GetTime();
    float deltaTime = time - m_lastTime;
    m_lastTime = time;
    
    // 限制 deltaTime 防止跳帧
    if (deltaTime > 0.1f) deltaTime = 0.1f;
    
    // 目标值：悬停时 1.08，否则 1.0
    float target = isHovered ? 1.08f : 1.0f;
    
    // 获取当前动画状态
    float current = m_hoverAnimState[itemId];
    
    // 平滑插值
    float speed = 8.0f; // 动画速度
    if (std::abs(target - current) > 0.001f) {
        current += (target - current) * speed * deltaTime;
    } else {
        current = target;
    }
    
    m_hoverAnimState[itemId] = current;
    return current;
}

void ItemGrid::RenderIconView() {
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(16, 16));
    ImGui::BeginChild("Items", ImVec2(0, 0), false);
    
    if (!m_items || m_items->empty()) {
        // 空结果提示 - 根据主题调整颜色
        ImVec4 emptyColor = (GetCurrentTheme() == ThemeType::Dark) 
            ? ImVec4(0.5f, 0.5f, 0.5f, 1.0f) 
            : ImVec4(0.5f, 0.5f, 0.5f, 1.0f);
        ImGui::TextColored(emptyColor, "暂无项目");
        ImGui::EndChild();
        ImGui::PopStyleVar();
        return;
    }
    
    float windowWidth = ImGui::GetContentRegionAvail().x;
    int columns = std::max(1, (int)(windowWidth / 95));
    
    // 根据主题设置颜色
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    ImVec4 hoverBgColor = isDark ? ImVec4(0.157f, 0.157f, 0.157f, 1.0f) : ImVec4(0.94f, 0.94f, 0.94f, 1.0f);
    ImVec4 activeBgColor = isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.91f, 0.92f, 0.95f, 1.0f);
    ImVec4 textColor = isDark ? ImVec4(0.8f, 0.8f, 0.8f, 1.0f) : ImVec4(0.33f, 0.33f, 0.33f, 1.0f);
    ImVec4 placeholderColor = isDark ? ImVec4(0.3f, 0.3f, 0.3f, 1.0f) : ImVec4(0.3f, 0.3f, 0.3f, 1.0f);
    
    int index = 0;
    for (auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        ImGui::BeginGroup();
        
        // 基础按钮样式
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0, 0, 0, 0));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, hoverBgColor);
        ImGui::PushStyleColor(ImGuiCol_ButtonActive, activeBgColor);
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 8.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(4, 4));
        
        ImVec2 baseIconSize(64, 64);
        
        // 检测悬停状态并计算动画缩放
        std::string buttonId = "##icon_" + std::to_string(index);
        ImGui::PushID(buttonId.c_str());
        
        // 预先检测悬停
        ImVec2 mousePos = ImGui::GetIO().MousePos;
        ImVec2 itemMin = ImGui::GetCursorScreenPos();
        ImVec2 itemMax = ImVec2(itemMin.x + baseIconSize.x + 8, itemMin.y + baseIconSize.y + 8);
        bool isHovered = (mousePos.x >= itemMin.x && mousePos.x <= itemMax.x &&
                         mousePos.y >= itemMin.y && mousePos.y <= itemMax.y);
        
        float scale = GetHoverScale(item.id, isHovered);
        ImVec2 iconSize(baseIconSize.x * scale, baseIconSize.y * scale);
        
        // 尝试加载图标纹理
        void* iconTexture = nullptr;
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item);
        }
        
        // 计算居中偏移
        float offsetX = (baseIconSize.x - iconSize.x) / 2;
        float offsetY = (baseIconSize.y - iconSize.y) / 2;
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + offsetX);
        ImGui::SetCursorPosY(ImGui::GetCursorPosY() + offsetY);
        
        if (iconTexture) {
            // 显示图标纹理
            if (ImGui::ImageButton("##icon", iconTexture, iconSize)) {
                m_selectedIndex = index;
                if (m_onClick) {
                    m_onClick(item);
                }
            }
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
                ImGui::PushStyleColor(ImGuiCol_Text, placeholderColor);
                ImGui::TextUnformatted(firstChar);
                ImGui::PopStyleColor();
            }
        }
        
        // 悬停时绘制发光效果
        if (scale > 1.01f) {
            ImVec2 min = ImGui::GetItemRectMin();
            ImVec2 max = ImGui::GetItemRectMax();
            ImDrawList* drawList = ImGui::GetWindowDrawList();
            
            // 根据主题选择发光颜色
            ImU32 glowColor = isDark 
                ? IM_COL32(100, 180, 255, (int)(40 * (scale - 1.0f) / 0.08f))
                : IM_COL32(0, 120, 212, (int)(30 * (scale - 1.0f) / 0.08f));
            
            drawList->AddRect(min, max, glowColor, 8.0f, 0, 2.0f);
        }
        
        // 右键菜单（必须在 PopID 之前绑定）
        if (ImGui::BeginPopupContextItem("item_context", ImGuiPopupFlags_MouseButtonRight)) {
            RenderContextMenu(item);
            ImGui::EndPopup();
        }
        
        ImGui::PopID();
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(3);
        
        // 名称标签
        float labelOffset = (baseIconSize.x - 64) / 2;
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + labelOffset);
        ImGui::PushStyleColor(ImGuiCol_Text, textColor);
        
        std::string displayName = StringUtils::TruncateUtf8(name, 6);
        
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
    
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    ImVec4 hoverBgColor = isDark ? ImVec4(0.157f, 0.157f, 0.157f, 1.0f) : ImVec4(0.94f, 0.94f, 0.94f, 1.0f);
    ImVec4 activeBgColor = isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.91f, 0.92f, 0.95f, 1.0f);
    ImVec4 textColor = isDark ? ImVec4(0.8f, 0.8f, 0.8f, 1.0f) : ImVec4(0.33f, 0.33f, 0.33f, 1.0f);
    
    int index = 0;
    for (auto& item : *m_items) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        ImGui::BeginGroup();
        
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0, 0, 0, 0));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, hoverBgColor);
        ImGui::PushStyleColor(ImGuiCol_ButtonActive, activeBgColor);
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 4.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(4, 4));
        
        // 列表项：小图标 + 名称
        ImVec2 iconSize(24, 24);
        
        void* iconTexture = nullptr;
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item);
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
            ImGui::PushStyleColor(ImGuiCol_Button, isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
            ImGui::Button("##placeholder", iconSize);
            ImGui::PopStyleColor();
            ImGui::SameLine(0, 8);
        }
        
        ImGui::PushStyleColor(ImGuiCol_Text, textColor);
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
    
    if (ImGui::MenuItem("刷新图标")) {
        if (m_onRefreshIcon) {
            m_onRefreshIcon(item);
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

void ItemGrid::OnItemRefreshIcon(std::function<void(const Item&)> callback) {
    m_onRefreshIcon = callback;
}

} // namespace mn