#include "ItemGrid.h"
#include "core/IconTextureManager.h"
#include "core/Localization.h"
#include "utils/StringUtils.h"
#include "ui/Theme.h"
#include <imgui.h>
#include <cmath>
#include <algorithm>

namespace mn {

void ItemGrid::SetItems(std::vector<Item>* items) {
    m_items = items;
    m_selectedIndex = -1;
    // 清理旧的动画状态，避免内存缓慢增长
    m_hoverAnimState.clear();
}

void ItemGrid::SetIconTextureManager(IconTextureManager* manager) {
    m_iconTextureManager = manager;
}

void ItemGrid::SetViewType(ViewType viewType) {
    m_viewType = viewType;
}

void ItemGrid::ClearHoverAnimation(const std::wstring& itemId) {
    m_hoverAnimState.erase(itemId);
}

void ItemGrid::Render() {
    // 按 sortOrder 排序后渲染
    if (m_items) {
        m_sortedItems = *m_items;
        std::sort(m_sortedItems.begin(), m_sortedItems.end(),
            [](const Item& a, const Item& b) { return a.sortOrder < b.sortOrder; });
    }
    
    if (m_viewType == ViewType::Icon) {
        RenderIconView();
    } else {
        RenderListView();
    }
}

float ItemGrid::GetHoverScale(const std::wstring& itemId, bool isHovered) {
    float deltaTime = ImGui::GetIO().DeltaTime;
    
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
    
    if (!m_items || m_sortedItems.empty()) {
        ImVec4 emptyColor = (GetCurrentTheme() == ThemeType::Dark) 
            ? ImVec4(0.5f, 0.5f, 0.5f, 1.0f) 
            : ImVec4(0.5f, 0.5f, 0.5f, 1.0f);
        ImGui::TextColored(emptyColor, "%s", TrUtf8("Tip_NoItems").c_str());
        if (ImGui::BeginPopupContextWindow("empty_area_context", ImGuiPopupFlags_MouseButtonRight)) {
            if (ImGui::MenuItem(TrUtf8("Menu_NewItem").c_str())) {
                if (m_onAddItem) m_onAddItem();
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        }
        ImGui::EndChild();
        ImGui::PopStyleVar();
        return;
    }
    
    float windowWidth = ImGui::GetContentRegionAvail().x;
    int columns = (std::max)(1, (int)(windowWidth / 95));
    
    // 根据主题设置颜色
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    ImVec4 hoverBgColor = isDark ? ImVec4(0.157f, 0.157f, 0.157f, 1.0f) : ImVec4(0.94f, 0.94f, 0.94f, 1.0f);
    ImVec4 activeBgColor = isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.91f, 0.92f, 0.95f, 1.0f);
    ImVec4 textColor = isDark ? ImVec4(0.8f, 0.8f, 0.8f, 1.0f) : ImVec4(0.33f, 0.33f, 0.33f, 1.0f);
    ImVec4 placeholderColor = isDark ? ImVec4(0.3f, 0.3f, 0.3f, 1.0f) : ImVec4(0.3f, 0.3f, 0.3f, 1.0f);
    
    int index = 0;
    for (auto& item : m_sortedItems) {
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
        
        // 尝试加载图标纹理
        ImTextureID iconTexture = ImTextureID(0);
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item);
        }
        
        // 先创建按钮获取悬停状态
        if (iconTexture) {
            // 显示图标纹理
            if (ImGui::ImageButton("##icon", iconTexture, baseIconSize)) {
                m_selectedIndex = index;
                if (m_onClick) {
                    m_onClick(item);
                }
            }
        } else {
            // 显示占位符
            if (ImGui::Button("##icon", baseIconSize)) {
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
                int charLen = 1;
                unsigned char c = static_cast<unsigned char>(name[0]);
                if ((c & 0xF8) == 0xF0) charLen = 4;
                else if ((c & 0xF0) == 0xE0) charLen = 3;
                else if ((c & 0xE0) == 0xC0) charLen = 2;
                for (int i = 0; i < name.size() && i < charLen; i++) {
                    firstChar[i] = name[i];
                }
                
                // 根据字体大小计算居中偏移
                ImVec2 textSize = ImGui::CalcTextSize(firstChar);
                ImGui::SetCursorScreenPos(ImVec2(center.x - textSize.x / 2, center.y - textSize.y / 2));
                ImGui::PushStyleColor(ImGuiCol_Text, placeholderColor);
                ImGui::TextUnformatted(firstChar);
                ImGui::PopStyleColor();
            }
        }
        
        // 使用 ImGui API 获取悬停状态
        bool isHovered = ImGui::IsItemHovered();
        float scale = GetHoverScale(item.id, isHovered);
        
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
        
        // 右键菜单
        if (ImGui::BeginPopupContextItem("item_context", ImGuiPopupFlags_MouseButtonRight)) {
            RenderContextMenu(item);
            ImGui::EndPopup();
        }
        
        ImGui::PopStyleVar(2);
        ImGui::PopStyleColor(3);
        
        // 名称标签
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
    
    if (!m_items || m_sortedItems.empty()) {
        ImVec4 emptyColor = (GetCurrentTheme() == ThemeType::Dark) 
            ? ImVec4(0.5f, 0.5f, 0.5f, 1.0f) 
            : ImVec4(0.5f, 0.5f, 0.5f, 1.0f);
        ImGui::TextColored(emptyColor, "%s", TrUtf8("Tip_NoItems").c_str());
        if (ImGui::BeginPopupContextWindow("empty_area_context", ImGuiPopupFlags_MouseButtonRight)) {
            if (ImGui::MenuItem(TrUtf8("Menu_NewItem").c_str())) {
                if (m_onAddItem) m_onAddItem();
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        }
        ImGui::EndChild();
        ImGui::PopStyleVar();
        return;
    }
    
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    ImVec4 textColor = isDark ? ImVec4(0.8f, 0.8f, 0.8f, 1.0f) : ImVec4(0.33f, 0.33f, 0.33f, 1.0f);
    ImVec4 placeholderColor = isDark ? ImVec4(0.3f, 0.3f, 0.3f, 1.0f) : ImVec4(0.6f, 0.6f, 0.6f, 1.0f);
    ImVec4 hoverColor = isDark ? ImVec4(0.157f, 0.157f, 0.157f, 1.0f) : ImVec4(0.94f, 0.94f, 0.94f, 1.0f);
    
    ImVec2 iconSize(24, 24);
    float contentWidth = ImGui::GetContentRegionAvail().x;
    
    int index = 0;
    for (auto& item : m_sortedItems) {
        ImGui::PushID(index);
        
        std::string name = StringUtils::WStringToUtf8(item.name);
        
        // 获取图标纹理
        ImTextureID iconTexture = ImTextureID(0);
        if (m_iconTextureManager) {
            iconTexture = m_iconTextureManager->GetIconTexture(item);
        }
        
        // 计算行高
        float rowHeight = iconSize.y + 8;
        
        // 先创建不可见按钮占据整行（用于点击和悬停检测）
        ImVec2 rowPos = ImGui::GetCursorScreenPos();
        bool clicked = ImGui::InvisibleButton("##row", ImVec2(contentWidth, rowHeight));
        bool hovered = ImGui::IsItemHovered();
        
        // 在悬停时绘制背景高亮
        if (hovered) {
            ImDrawList* drawList = ImGui::GetWindowDrawList();
            ImU32 bgColor = isDark ? IM_COL32(40, 40, 40, 255) : IM_COL32(240, 240, 240, 255);
            drawList->AddRectFilled(rowPos, ImVec2(rowPos.x + contentWidth, rowPos.y + rowHeight), bgColor, 4.0f);
        }
        
        // 回到行起始位置绘制内容
        ImGui::SetCursorScreenPos(rowPos);
        ImGui::SetCursorPosX(ImGui::GetCursorPosX() + 4);  // 左侧内边距
        
        // 图标区域
        if (iconTexture) {
            ImGui::SetCursorPosY(ImGui::GetCursorPosY() + (rowHeight - iconSize.y) / 2);
            ImGui::Image(iconTexture, iconSize);
        } else {
            // 占位符 - 显示首字符
            ImGui::SetCursorPosY(ImGui::GetCursorPosY() + (rowHeight - iconSize.y) / 2);
            ImGui::PushStyleColor(ImGuiCol_Button, isDark ? ImVec4(0.2f, 0.2f, 0.2f, 1.0f) : ImVec4(0.9f, 0.9f, 0.9f, 1.0f));
            ImGui::Button("##placeholder", iconSize);
            ImGui::PopStyleColor();
            
            // 在占位符上显示首字符
            if (!name.empty()) {
                ImVec2 min = ImGui::GetItemRectMin();
                ImVec2 max = ImGui::GetItemRectMax();
                ImVec2 center((min.x + max.x) / 2, (min.y + max.y) / 2);
                
                char firstChar[8] = {};
                int charLen = 1;
                unsigned char c = static_cast<unsigned char>(name[0]);
                if ((c & 0xF8) == 0xF0) charLen = 4;
                else if ((c & 0xF0) == 0xE0) charLen = 3;
                else if ((c & 0xE0) == 0xC0) charLen = 2;
                for (int i = 0; i < name.size() && i < charLen; i++) {
                    firstChar[i] = name[i];
                }
                
                // 根据字体大小计算居中偏移
                ImVec2 textSize = ImGui::CalcTextSize(firstChar);
                ImGui::SetCursorScreenPos(ImVec2(center.x - textSize.x / 2, center.y - textSize.y / 2));
                ImGui::PushStyleColor(ImGuiCol_Text, placeholderColor);
                ImGui::TextUnformatted(firstChar);
                ImGui::PopStyleColor();
            }
        }
        
        ImGui::SameLine(0, 8);
        
        // 文本 - 垂直居中
        ImGui::SetCursorPosY(rowPos.y - ImGui::GetWindowPos().y + (rowHeight - ImGui::GetTextLineHeight()) / 2 + ImGui::GetScrollY());
        ImGui::PushStyleColor(ImGuiCol_Text, textColor);
        ImGui::Text("%s", name.c_str());
        ImGui::PopStyleColor();
        
        // 处理点击事件
        if (clicked) {
            m_selectedIndex = index;
            if (m_onClick) {
                m_onClick(item);
            }
        }
        
        // 右键菜单 - 绑定到 InvisibleButton
        if (ImGui::BeginPopupContextItem("item_context", ImGuiPopupFlags_MouseButtonRight)) {
            RenderContextMenu(item);
            ImGui::EndPopup();
        }
        
        ImGui::PopID();
        index++;
    }
    
    ImGui::EndChild();
    ImGui::PopStyleVar();
}

void ItemGrid::RenderContextMenu(Item& item) {
    if (ImGui::MenuItem(TrUtf8("Menu_CopyPath").c_str())) {
        if (m_onCopyPath) m_onCopyPath(item);
        ImGui::CloseCurrentPopup();
    }
    if (ImGui::MenuItem(TrUtf8("Menu_OpenLocation").c_str())) {
        if (m_onOpenLocation) m_onOpenLocation(item);
        ImGui::CloseCurrentPopup();
    }
    if (ImGui::BeginMenu(TrUtf8("Menu_MoveToCategory").c_str())) {
        if (m_allCategories) {
            for (auto& cat : *m_allCategories) {
                if (ImGui::MenuItem(StringUtils::WStringToUtf8(cat.name).c_str())) {
                    if (m_onMoveToCategory) m_onMoveToCategory(item, cat.id);
                    ImGui::CloseCurrentPopup();
                }
            }
        }
        ImGui::EndMenu();
    }
    if (ImGui::MenuItem(TrUtf8("Menu_Properties").c_str())) {
        if (m_onProperties) m_onProperties(item);
        ImGui::CloseCurrentPopup();
    }
    ImGui::Separator();
    
    if (ImGui::MenuItem(TrUtf8("Menu_Run").c_str())) {
        if (m_onClick) {
            m_onClick(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem(TrUtf8("Menu_RunAsAdmin").c_str())) {
        if (m_onRunAsAdmin) {
            m_onRunAsAdmin(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem(TrUtf8("Menu_RefreshIcon").c_str())) {
        if (m_onRefreshIcon) {
            m_onRefreshIcon(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem(TrUtf8("Menu_Edit").c_str())) {
        if (m_onEdit) {
            m_onEdit(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    if (ImGui::MenuItem(TrUtf8("Menu_DelItem").c_str())) {
        if (m_onDelete) {
            m_onDelete(item);
        }
        ImGui::CloseCurrentPopup();
    }
    
    ImGui::Separator();
    
    if (ImGui::MenuItem(TrUtf8("Menu_ViewIcon0").c_str(), nullptr, m_viewType == ViewType::Icon)) {
        SetViewType(ViewType::Icon);
    }
    
    if (ImGui::MenuItem(TrUtf8("Menu_ViewIcon1").c_str(), nullptr, m_viewType == ViewType::List)) {
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

void ItemGrid::OnItemAdd(std::function<void()> callback) {
    m_onAddItem = callback;
}

void ItemGrid::OnItemCopyPath(std::function<void(const Item&)> callback) { m_onCopyPath = callback; }
void ItemGrid::OnItemOpenLocation(std::function<void(const Item&)> callback) { m_onOpenLocation = callback; }
void ItemGrid::OnItemMoveToCategory(std::function<void(const Item&, const std::wstring&)> callback) { m_onMoveToCategory = callback; }
void ItemGrid::OnItemProperties(std::function<void(const Item&)> callback) { m_onProperties = callback; }
void ItemGrid::SetAllCategories(const std::vector<Category>* categories) { m_allCategories = categories; }

} // namespace mn