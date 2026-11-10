#pragma once

namespace mn {

// 主题类型
enum class ThemeType {
    Light = 0,
    Dark = 1
};

// 当前主题
extern ThemeType g_currentTheme;

// 应用主题
void ApplyLightTheme();
void ApplyDarkTheme();
void ApplyTheme(ThemeType theme);

// 获取当前主题
ThemeType GetCurrentTheme();

// 切换主题
void ToggleTheme();

}