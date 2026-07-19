#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "core/IconTextureManager.h"
#include "utils/StringUtils.h"
#include "ui/Theme.h"
#include "app/App.h"
#include <imgui.h>
#include <algorithm>
#include <windows.h>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    m_iconTextureManager = iconTextureManager;
    
    // 恢复保存的主题
    ApplyTheme(static_cast<ThemeType>(m_config->GetTheme()));
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_searchBuf[0] = '\0';
        m_isSearching = false;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
    });
    
    // 新建分类
    m_categoryTab.OnCategoryAdd([this]() {
        Category cat;
        cat.id = GenerateId(L"cat");
        cat.name = L"新分类";
        m_itemManager->AddCategory(cat);
        m_categoryTab.SetCategories(&m_itemManager->GetCategories());
        m_categoryTab.SetCurrentCategory(cat.id);
        m_currentCategoryId = cat.id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(cat.id));
        // 自动打开重命名对话框
        strncpy(m_renameCategoryBuf, "新分类", sizeof(m_renameCategoryBuf) - 1);
        m_showRenameCategory = true;
        m_openRenamePopup = true;
    });
    
    // 删除分类
    m_categoryTab.OnCategoryDelete([this](const std::wstring& id) {
        if (m_itemManager->GetCategories().size() > 1) {
            m_itemManager->DeleteCategory(id);
            m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            auto& cats = m_itemManager->GetCategories();
            if (!cats.empty()) {
                m_currentCategoryId = cats[0].id;
                m_categoryTab.SetCurrentCategory(m_currentCategoryId);
                m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
            }
        } else {
            ShowError(L"至少需要保留一个分类");
        }
    });
    
    // 重命名分类
    m_categoryTab.OnCategoryRename([this](const std::wstring& id) {
        Category* cat = m_itemManager->GetCategory(id);
        if (cat) {
            m_currentCategoryId = id;
            std::string name = StringUtils::WStringToUtf8(cat->name);
            strncpy(m_renameCategoryBuf, name.c_str(), sizeof(m_renameCategoryBuf) - 1);
            m_renameCategoryBuf[sizeof(m_renameCategoryBuf) - 1] = '\0';
            m_showRenameCategory = true;
            m_openRenamePopup = true;
        }
    });
    
    // 设置 Runner 回调，运行成功后递增运行次数
    m_runner->SetRunCallback([this](const Item& item, bool success) {
        if (success) {
            m_itemManager->IncrementRunCount(item.id);
        }
    });
    
    // 单击运行
    m_itemGrid.OnItemClicked([this](const Item& item) {
        RunResult result = m_runner->Run(item);
        if (!result.success) {
            ShowError(result.errorMessage);
        }
    });
    
    // 管理员运行
    m_itemGrid.OnItemRunAsAdmin([this](const Item& item) {
        RunResult result = m_runner->RunAsAdmin(item);
        if (!result.success) {
            ShowError(result.errorMessage);
        }
    });
    
    // 编辑项目
    m_itemGrid.OnItemEdit([this](Item& item) {
        m_editDialog.Show(&item);
    });
    
    // 删除项目
    m_itemGrid.OnItemDelete([this](const Item& item) {
        m_deleteItemId = item.id;
        m_deleteItemName = item.name;
        m_showDeleteConfirm = true;
        m_openDeletePopup = true;
    });
    
    // 刷新图标
    m_itemGrid.OnItemRefreshIcon([this](const Item& item) {
        m_iconTextureManager->RefreshIcon(item);
    });
    
    // 设置图标纹理管理器
    m_itemGrid.SetIconTextureManager(iconTextureManager);
    
    // 编辑保存后刷新并保存
    m_editDialog.OnSave([this](const Item& item) {
        // 更新项目到存储
        m_itemManager->UpdateItem(item);
        RefreshItems();
    });
    
    auto& categories = itemManager->GetCategories();
    if (!categories.empty()) {
        m_currentCategoryId = categories[0].id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    }
}

void MainWindow::Render() {
    if (!m_visible) return;
    
    // Escape 键处理：清除搜索或关闭窗口
    if (ImGui::IsKeyPressed(ImGuiKey_Escape, false)) {
        if (m_isSearching) {
            m_searchBuf[0] = '\0';
            m_isSearching = false;
            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        } else if (m_editDialog.IsVisible()) {
            m_editDialog.Hide();
        } else {
            App::Get()->ToggleWindow();
        }
    }
    
    // 菜单栏
    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu("文件")) {
            if (ImGui::MenuItem("新建分类")) {
                Category cat;
                cat.id = GenerateId(L"cat");
                cat.name = L"新分类";
                m_itemManager->AddCategory(cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            if (ImGui::MenuItem("删除当前分类", nullptr, false, !m_itemManager->GetCategories().empty())) {
                if (!m_currentCategoryId.empty()) {
                    if (m_itemManager->GetCategories().size() > 1) {
                        m_itemManager->DeleteCategory(m_currentCategoryId);
                        m_categoryTab.SetCategories(&m_itemManager->GetCategories());
                        auto& cats = m_itemManager->GetCategories();
                        if (!cats.empty()) {
                            m_currentCategoryId = cats[0].id;
                            m_categoryTab.SetCurrentCategory(m_currentCategoryId);
                            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
                        }
                    } else {
                        ShowError(L"至少需要保留一个分类");
                    }
                }
            }
            ImGui::Separator();
            if (ImGui::MenuItem("退出")) {
                PostQuitMessage(0);
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu("编辑")) {
            if (ImGui::MenuItem("重命名当前分类", nullptr, false, !m_itemManager->GetCategories().empty())) {
                // 打开重命名对话框
                Category* cat = m_itemManager->GetCategory(m_currentCategoryId);
                if (cat) {
                    std::string name = StringUtils::WStringToUtf8(cat->name);
                    strncpy(m_renameCategoryBuf, name.c_str(), sizeof(m_renameCategoryBuf) - 1);
                    m_renameCategoryBuf[sizeof(m_renameCategoryBuf) - 1] = '\0';
                    m_showRenameCategory = true;
                    m_openRenamePopup = true;
                }
            }
            ImGui::EndMenu();
        }
        ImGui::EndMenuBar();
    }
    
    RenderSearchBar();
    
    ImGui::BeginChild("MainContent", ImVec2(0, 0), false);
    
    ImGui::BeginChild("Left", ImVec2(160, 0), false);
    m_categoryTab.Render();
    ImGui::EndChild();
    
    ImGui::SameLine(0, 0);
    
    ImGui::BeginChild("Right", ImVec2(0, 0), false);
    m_itemGrid.Render();
    ImGui::EndChild();
    
    ImGui::EndChild();
    
    m_editDialog.Render();
    
    // 分类重命名对话框
    RenderRenameCategoryDialog();
    
    // 删除确认对话框
    RenderDeleteConfirmDialog();
    
    if (m_showError) {
        // 只在首次显示时打开弹窗
        if (m_openErrorPopup) {
            ImGui::OpenPopup("错误");
            m_openErrorPopup = false;
        }
        
        if (ImGui::BeginPopupModal("错误", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button("确定", ImVec2(100, 0))) {
                m_showError = false;
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        } else {
            // 弹窗被用户关闭，重置状态
            m_showError = false;
        }
    }
}

void MainWindow::RenderSearchBar() {
    // 搜索框（左侧）
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(12, 10));
    ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 6.0f);
    
    bool isDark = (GetCurrentTheme() == ThemeType::Dark);
    if (isDark) {
        ImGui::PushStyleColor(ImGuiCol_FrameBg, ImVec4(0.15f, 0.15f, 0.15f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_FrameBgHovered, ImVec4(0.18f, 0.18f, 0.18f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_FrameBgActive, ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
    } else {
        ImGui::PushStyleColor(ImGuiCol_FrameBg, ImVec4(0.96f, 0.96f, 0.96f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_FrameBgHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_FrameBgActive, ImVec4(0.92f, 0.92f, 0.92f, 1.0f));
    }
    
    // 首次显示时聚焦搜索框（只执行一次）
    if (m_focusSearch) {
        ImGui::SetKeyboardFocusHere();
        m_focusSearch = false;
    }
    
    ImGui::SetNextItemWidth(-FLT_MIN);
    bool searchChanged = ImGui::InputTextWithHint("##search", "搜索...", m_searchBuf, sizeof(m_searchBuf));
    
    ImGui::PopStyleColor(3);
    ImGui::PopStyleVar(2);
    
    if (searchChanged) {
        m_lastSearchTime = ImGui::GetTime();
        strcpy(m_pendingSearchBuf, m_searchBuf);
        m_searchPending = true;
    }
    
    // 检查延迟搜索
    if (m_searchPending && (ImGui::GetTime() - m_lastSearchTime) >= m_searchDelay) {
        m_searchPending = false;
        
        std::string searchText(m_pendingSearchBuf);
        m_isSearching = !searchText.empty();
        
        if (m_isSearching) {
            UpdateSearchResults();
            m_searchSelectedIndex = -1;
        } else {
            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        }
    }
    
    // 搜索键盘导航
    if (m_isSearching && !m_searchResults.empty()) {
        if (ImGui::IsKeyPressed(ImGuiKey_DownArrow, false)) {
            m_searchSelectedIndex = (m_searchSelectedIndex + 1) % static_cast<int>(m_searchResults.size());
        }
        if (ImGui::IsKeyPressed(ImGuiKey_UpArrow, false)) {
            m_searchSelectedIndex = (m_searchSelectedIndex - 1 + static_cast<int>(m_searchResults.size())) % static_cast<int>(m_searchResults.size());
        }
        if (ImGui::IsKeyPressed(ImGuiKey_Enter, false) && m_searchSelectedIndex >= 0) {
            RunResult result = m_runner->Run(m_searchResults[m_searchSelectedIndex]);
            if (!result.success) {
                ShowError(result.errorMessage);
            }
        }
    }
    
    ImGui::Spacing();
    
    // 视图切换按钮
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(8, 8));
    ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 6.0f);
    
    const char* viewLabel = (m_currentViewType == ViewType::Icon) ? "图标" : "列表";
    if (ImGui::Button(viewLabel, ImVec2(50, 0))) {
        ToggleViewType();
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("%s", (m_currentViewType == ViewType::Icon) ? "切换到列表视图" : "切换到图标视图");
    }
    
    // 主题切换按钮
    ImGui::SameLine(0, 4);
    const char* themeLabel = (GetCurrentTheme() == ThemeType::Light) ? "浅色" : "深色";
    if (ImGui::Button(themeLabel, ImVec2(50, 0))) {
        ToggleTheme();
        if (m_config) {
            m_config->SetTheme(static_cast<int>(GetCurrentTheme()));
        }
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("%s", (GetCurrentTheme() == ThemeType::Light) ? "切换到深色主题" : "切换到浅色主题");
    }
    
    ImGui::PopStyleVar(2);
    
    ImGui::Spacing();
    ImGui::Spacing();
}

void MainWindow::UpdateSearchResults() {
    std::string searchStr(m_searchBuf);
    std::wstring searchWStr = StringUtils::Utf8ToWString(searchStr);
    
    m_searchResults = m_itemManager->SearchItems(searchWStr);
    m_itemGrid.SetItems(&m_searchResults);
}

void MainWindow::RefreshItems() {
    if (m_isSearching) {
        UpdateSearchResults();
    } else {
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    }
}

void MainWindow::Show() {
    m_visible = true;
    m_focusSearch = true;  // 显示时聚焦搜索框
}

void MainWindow::Hide() {
    m_visible = false;
}

void MainWindow::Toggle() {
    m_visible = !m_visible;
}

bool MainWindow::IsVisible() const {
    return m_visible;
}

void MainWindow::SetCurrentCategory(const std::wstring& categoryId) {
    m_currentCategoryId = categoryId;
    m_categoryTab.SetCurrentCategory(categoryId);
    m_itemGrid.SetItems(&m_itemManager->GetItems(categoryId));
}

void MainWindow::ShowError(const std::wstring& message) {
    m_errorMessage = message;
    m_showError = true;
    m_openErrorPopup = true;  // 标记需要打开弹窗
}

void MainWindow::RenderRenameCategoryDialog() {
    if (!m_showRenameCategory) return;
    
    // 只在首次显示时打开弹窗
    if (m_openRenamePopup) {
        ImGui::OpenPopup("重命名分类");
        m_openRenamePopup = false;
    }
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(300, 0));
    
    if (ImGui::BeginPopupModal("重命名分类", nullptr, ImGuiWindowFlags_NoResize)) {
        if (ImGui::InputText("分类名称", m_renameCategoryBuf, sizeof(m_renameCategoryBuf), ImGuiInputTextFlags_EnterReturnsTrue)) {
            // 回车确认
            Category* cat = m_itemManager->GetCategory(m_currentCategoryId);
            if (cat) {
                cat->name = StringUtils::Utf8ToWString(m_renameCategoryBuf);
                m_itemManager->UpdateCategory(*cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            m_showRenameCategory = false;
            ImGui::CloseCurrentPopup();
        }
        
        ImGui::Separator();
        
        if (ImGui::Button("确定", ImVec2(100, 0))) {
            Category* cat = m_itemManager->GetCategory(m_currentCategoryId);
            if (cat) {
                cat->name = StringUtils::Utf8ToWString(m_renameCategoryBuf);
                m_itemManager->UpdateCategory(*cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            m_showRenameCategory = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("取消", ImVec2(100, 0))) {
            m_showRenameCategory = false;
            ImGui::CloseCurrentPopup();
        }
        
        ImGui::EndPopup();
    } else {
        // 弹窗被用户关闭（点击外部或按 ESC），重置状态
        m_showRenameCategory = false;
    }
}

void MainWindow::RenderDeleteConfirmDialog() {
    if (!m_showDeleteConfirm) return;
    
    if (m_openDeletePopup) {
        ImGui::OpenPopup("确认删除");
        m_openDeletePopup = false;
    }
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(320, 0));
    
    if (ImGui::BeginPopupModal("确认删除", nullptr, ImGuiWindowFlags_NoResize)) {
        std::string displayName = StringUtils::WStringToUtf8(m_deleteItemName);
        if (displayName.length() > 30) {
            displayName = displayName.substr(0, 30) + "...";
        }
        ImGui::Text("确定要删除 \"%s\" 吗？", displayName.c_str());
        ImGui::TextUnformatted("此操作不可撤销。");
        ImGui::Separator();
        
        if (ImGui::Button("确定删除", ImVec2(120, 0))) {
            m_itemGrid.ClearHoverAnimation(m_deleteItemId);
            m_itemManager->DeleteItem(m_deleteItemId);
            RefreshItems();
            m_showDeleteConfirm = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("取消", ImVec2(120, 0))) {
            m_showDeleteConfirm = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    } else {
        m_showDeleteConfirm = false;
    }
}

void MainWindow::ToggleViewType() {
    m_currentViewType = (m_currentViewType == ViewType::Icon) ? ViewType::List : ViewType::Icon;
    m_itemGrid.SetViewType(m_currentViewType);
}

} // namespace mn
