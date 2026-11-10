#include "Theme.h"
#include <imgui.h>

namespace mn {

ThemeType g_currentTheme = ThemeType::Light;

void ApplyLightTheme() {
    g_currentTheme = ThemeType::Light;
    
    ImGuiStyle& style = ImGui::GetStyle();
    
    // 圆角
    style.WindowRounding = 8.0f;
    style.ChildRounding = 6.0f;
    style.FrameRounding = 4.0f;
    style.PopupRounding = 4.0f;
    style.ScrollbarRounding = 4.0f;
    style.GrabRounding = 4.0f;
    style.TabRounding = 4.0f;
    
    // 边框
    style.WindowBorderSize = 0.0f;
    style.ChildBorderSize = 0.0f;
    style.PopupBorderSize = 0.0f;
    style.FrameBorderSize = 0.0f;
    
    // 间距
    style.WindowPadding = ImVec2(12, 12);
    style.FramePadding = ImVec2(8, 6);
    style.CellPadding = ImVec2(8, 6);
    style.ItemSpacing = ImVec2(12, 8);
    style.ItemInnerSpacing = ImVec2(6, 6);
    
    // 配色方案 - Windows 11 Light Theme
    ImVec4 white = ImVec4(1.0f, 1.0f, 1.0f, 1.0f);
    ImVec4 bgLight = ImVec4(0.961f, 0.961f, 0.961f, 1.0f);      // #F5F5F5
    ImVec4 bgLighter = ImVec4(0.902f, 0.914f, 0.941f, 1.0f);    // #E7EAF0 - 选中背景
    ImVec4 textDark = ImVec4(0.2f, 0.2f, 0.2f, 1.0f);           // #333333
    ImVec4 textGray = ImVec4(0.4f, 0.4f, 0.4f, 1.0f);           // #666666
    ImVec4 textLight = ImVec4(0.6f, 0.6f, 0.6f, 1.0f);          // #999999
    ImVec4 borderLight = ImVec4(0.85f, 0.85f, 0.85f, 1.0f);     // #D9D9D9
    ImVec4 accent = ImVec4(0.0f, 0.478f, 0.8f, 1.0f);           // #0078D4 - 蓝色高亮
    ImVec4 hoverBg = ImVec4(0.94f, 0.94f, 0.94f, 1.0f);         // #F0F0F0
    ImVec4 activeBg = ImVec4(0.91f, 0.92f, 0.95f, 1.0f);        // #E8EAF2
    
    // 窗口
    style.Colors[ImGuiCol_WindowBg] = white;
    style.Colors[ImGuiCol_ChildBg] = white;
    style.Colors[ImGuiCol_PopupBg] = white;
    style.Colors[ImGuiCol_Border] = borderLight;
    style.Colors[ImGuiCol_BorderShadow] = ImVec4(0, 0, 0, 0);
    
    // 标题栏
    style.Colors[ImGuiCol_TitleBg] = bgLight;
    style.Colors[ImGuiCol_TitleBgActive] = bgLight;
    style.Colors[ImGuiCol_TitleBgCollapsed] = bgLight;
    
    // 菜单栏
    style.Colors[ImGuiCol_MenuBarBg] = bgLight;
    
    // 文字
    style.Colors[ImGuiCol_Text] = textDark;
    style.Colors[ImGuiCol_TextDisabled] = textLight;
    style.Colors[ImGuiCol_TextSelectedBg] = ImVec4(0.0f, 0.478f, 0.8f, 0.35f);
    
    // 按钮
    style.Colors[ImGuiCol_Button] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ButtonHovered] = hoverBg;
    style.Colors[ImGuiCol_ButtonActive] = activeBg;
    
    // 输入框
    style.Colors[ImGuiCol_FrameBg] = bgLight;
    style.Colors[ImGuiCol_FrameBgHovered] = ImVec4(0.94f, 0.94f, 0.94f, 1.0f);
    style.Colors[ImGuiCol_FrameBgActive] = ImVec4(0.91f, 0.92f, 0.95f, 1.0f);
    
    // 头部（折叠、树节点等）
    style.Colors[ImGuiCol_Header] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_HeaderHovered] = hoverBg;
    style.Colors[ImGuiCol_HeaderActive] = bgLighter;
    
    // 选中项
    style.Colors[ImGuiCol_Separator] = borderLight;
    style.Colors[ImGuiCol_SeparatorHovered] = textLight;
    style.Colors[ImGuiCol_SeparatorActive] = textGray;
    
    // 滚动条
    style.Colors[ImGuiCol_ScrollbarBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ScrollbarGrab] = ImVec4(0.7f, 0.7f, 0.7f, 0.5f);
    style.Colors[ImGuiCol_ScrollbarGrabHovered] = ImVec4(0.6f, 0.6f, 0.6f, 0.7f);
    style.Colors[ImGuiCol_ScrollbarGrabActive] = ImVec4(0.5f, 0.5f, 0.5f, 0.8f);
    
    // 滑块
    style.Colors[ImGuiCol_SliderGrab] = textGray;
    style.Colors[ImGuiCol_SliderGrabActive] = textDark;
    
    // Tab
    style.Colors[ImGuiCol_Tab] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TabHovered] = hoverBg;
    style.Colors[ImGuiCol_TabActive] = white;
    style.Colors[ImGuiCol_TabUnfocused] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TabUnfocusedActive] = bgLighter;
    
    // 表头
    style.Colors[ImGuiCol_TableHeaderBg] = bgLight;
    style.Colors[ImGuiCol_TableBorderStrong] = borderLight;
    style.Colors[ImGuiCol_TableBorderLight] = borderLight;
    style.Colors[ImGuiCol_TableRowBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TableRowBgAlt] = bgLight;
    
    // 拖拽
    style.Colors[ImGuiCol_DragDropTarget] = ImVec4(0.0f, 0.478f, 0.8f, 0.9f);
    
    // 导航
    style.Colors[ImGuiCol_NavHighlight] = accent;
    style.Colors[ImGuiCol_NavWindowingHighlight] = ImVec4(1.0f, 1.0f, 1.0f, 0.7f);
    style.Colors[ImGuiCol_NavWindowingDimBg] = ImVec4(0.8f, 0.8f, 0.8f, 0.2f);
    
    // 模态窗口
    style.Colors[ImGuiCol_ModalWindowDimBg] = ImVec4(0.2f, 0.2f, 0.2f, 0.35f);
    
    // CheckMark
    style.Colors[ImGuiCol_CheckMark] = accent;
    
    // Resize Grip
    style.Colors[ImGuiCol_ResizeGrip] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ResizeGripHovered] = textLight;
    style.Colors[ImGuiCol_ResizeGripActive] = textGray;
    
    // Plot
    style.Colors[ImGuiCol_PlotLines] = textGray;
    style.Colors[ImGuiCol_PlotLinesHovered] = accent;
    style.Colors[ImGuiCol_PlotHistogram] = accent;
    style.Colors[ImGuiCol_PlotHistogramHovered] = ImVec4(0.0f, 0.35f, 0.6f, 1.0f);
}

void ApplyDarkTheme() {
    g_currentTheme = ThemeType::Dark;
    
    ImGuiStyle& style = ImGui::GetStyle();
    
    // 圆角
    style.WindowRounding = 8.0f;
    style.ChildRounding = 6.0f;
    style.FrameRounding = 4.0f;
    style.PopupRounding = 4.0f;
    style.ScrollbarRounding = 4.0f;
    style.GrabRounding = 4.0f;
    style.TabRounding = 4.0f;
    
    // 边框
    style.WindowBorderSize = 0.0f;
    style.ChildBorderSize = 0.0f;
    style.PopupBorderSize = 0.0f;
    style.FrameBorderSize = 0.0f;
    
    // 间距
    style.WindowPadding = ImVec2(12, 12);
    style.FramePadding = ImVec2(8, 6);
    style.CellPadding = ImVec2(8, 6);
    style.ItemSpacing = ImVec2(12, 8);
    style.ItemInnerSpacing = ImVec2(6, 6);
    
    // 配色方案 - Modern Dark Theme
    ImVec4 bgDark = ImVec4(0.063f, 0.063f, 0.063f, 1.0f);        // #101010 - 主背景
    ImVec4 bgDark2 = ImVec4(0.09f, 0.09f, 0.09f, 1.0f);          // #171717 - 次背景
    ImVec4 bgDark3 = ImVec4(0.118f, 0.118f, 0.118f, 1.0f);       // #1E1E1E - 卡片背景
    ImVec4 bgDark4 = ImVec4(0.157f, 0.157f, 0.157f, 1.0f);       // #282828 - 悬停背景
    ImVec4 bgDark5 = ImVec4(0.2f, 0.2f, 0.2f, 1.0f);             // #333333 - 激活背景
    ImVec4 textWhite = ImVec4(0.95f, 0.95f, 0.95f, 1.0f);        // #F2F2F2 - 主文字
    ImVec4 textGray = ImVec4(0.7f, 0.7f, 0.7f, 1.0f);            // #B3B3B3 - 次文字
    ImVec4 textDim = ImVec4(0.5f, 0.5f, 0.5f, 1.0f);             // #808080 - 禁用文字
    ImVec4 borderDark = ImVec4(0.25f, 0.25f, 0.25f, 1.0f);       // #404040 - 边框
    ImVec4 accent = ImVec4(0.4f, 0.7f, 1.0f, 1.0f);              // #66B3FF - 蓝色高亮
    ImVec4 accentHover = ImVec4(0.5f, 0.75f, 1.0f, 1.0f);        // 悬停高亮
    ImVec4 accentActive = ImVec4(0.35f, 0.65f, 0.95f, 1.0f);     // 激活高亮
    
    // 窗口
    style.Colors[ImGuiCol_WindowBg] = bgDark;
    style.Colors[ImGuiCol_ChildBg] = bgDark2;
    style.Colors[ImGuiCol_PopupBg] = bgDark3;
    style.Colors[ImGuiCol_Border] = borderDark;
    style.Colors[ImGuiCol_BorderShadow] = ImVec4(0, 0, 0, 0);
    
    // 标题栏
    style.Colors[ImGuiCol_TitleBg] = bgDark;
    style.Colors[ImGuiCol_TitleBgActive] = bgDark;
    style.Colors[ImGuiCol_TitleBgCollapsed] = bgDark;
    
    // 菜单栏
    style.Colors[ImGuiCol_MenuBarBg] = bgDark2;
    
    // 文字
    style.Colors[ImGuiCol_Text] = textWhite;
    style.Colors[ImGuiCol_TextDisabled] = textDim;
    style.Colors[ImGuiCol_TextSelectedBg] = ImVec4(0.4f, 0.7f, 1.0f, 0.35f);
    
    // 按钮
    style.Colors[ImGuiCol_Button] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ButtonHovered] = bgDark4;
    style.Colors[ImGuiCol_ButtonActive] = bgDark5;
    
    // 输入框
    style.Colors[ImGuiCol_FrameBg] = bgDark3;
    style.Colors[ImGuiCol_FrameBgHovered] = bgDark4;
    style.Colors[ImGuiCol_FrameBgActive] = bgDark5;
    
    // 头部（折叠、树节点等）
    style.Colors[ImGuiCol_Header] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_HeaderHovered] = bgDark4;
    style.Colors[ImGuiCol_HeaderActive] = bgDark5;
    
    // 分隔线
    style.Colors[ImGuiCol_Separator] = borderDark;
    style.Colors[ImGuiCol_SeparatorHovered] = textDim;
    style.Colors[ImGuiCol_SeparatorActive] = textGray;
    
    // 滚动条
    style.Colors[ImGuiCol_ScrollbarBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ScrollbarGrab] = ImVec4(0.3f, 0.3f, 0.3f, 0.5f);
    style.Colors[ImGuiCol_ScrollbarGrabHovered] = ImVec4(0.4f, 0.4f, 0.4f, 0.7f);
    style.Colors[ImGuiCol_ScrollbarGrabActive] = ImVec4(0.5f, 0.5f, 0.5f, 0.8f);
    
    // 滑块
    style.Colors[ImGuiCol_SliderGrab] = textGray;
    style.Colors[ImGuiCol_SliderGrabActive] = textWhite;
    
    // Tab
    style.Colors[ImGuiCol_Tab] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TabHovered] = bgDark4;
    style.Colors[ImGuiCol_TabActive] = bgDark3;
    style.Colors[ImGuiCol_TabUnfocused] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TabUnfocusedActive] = bgDark4;
    
    // 表头
    style.Colors[ImGuiCol_TableHeaderBg] = bgDark2;
    style.Colors[ImGuiCol_TableBorderStrong] = borderDark;
    style.Colors[ImGuiCol_TableBorderLight] = borderDark;
    style.Colors[ImGuiCol_TableRowBg] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_TableRowBgAlt] = bgDark2;
    
    // 拖拽
    style.Colors[ImGuiCol_DragDropTarget] = ImVec4(0.4f, 0.7f, 1.0f, 0.9f);
    
    // 导航
    style.Colors[ImGuiCol_NavHighlight] = accent;
    style.Colors[ImGuiCol_NavWindowingHighlight] = ImVec4(0.4f, 0.7f, 1.0f, 0.7f);
    style.Colors[ImGuiCol_NavWindowingDimBg] = ImVec4(0.1f, 0.1f, 0.1f, 0.3f);
    
    // 模态窗口
    style.Colors[ImGuiCol_ModalWindowDimBg] = ImVec4(0.0f, 0.0f, 0.0f, 0.5f);
    
    // CheckMark
    style.Colors[ImGuiCol_CheckMark] = accent;
    
    // Resize Grip
    style.Colors[ImGuiCol_ResizeGrip] = ImVec4(0, 0, 0, 0);
    style.Colors[ImGuiCol_ResizeGripHovered] = textDim;
    style.Colors[ImGuiCol_ResizeGripActive] = textGray;
    
    // Plot
    style.Colors[ImGuiCol_PlotLines] = textGray;
    style.Colors[ImGuiCol_PlotLinesHovered] = accent;
    style.Colors[ImGuiCol_PlotHistogram] = accent;
    style.Colors[ImGuiCol_PlotHistogramHovered] = accentHover;
}

void ApplyTheme(ThemeType theme) {
    if (theme == ThemeType::Dark) {
        ApplyDarkTheme();
    } else {
        ApplyLightTheme();
    }
}

ThemeType GetCurrentTheme() {
    return g_currentTheme;
}

void ToggleTheme() {
    if (g_currentTheme == ThemeType::Light) {
        ApplyDarkTheme();
    } else {
        ApplyLightTheme();
    }
}

}