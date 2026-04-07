#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "core/IconTextureManager.h"
#include "utils/StringUtils.h"
#include <imgui.h>
#include <algorithm>
#include <windows.h>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    m_iconTextureManager = iconTextureManager;
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_searchBuf[0] = '\0';
        m_isSearching = false;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
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
        m_itemManager->DeleteItem(item.id);
        RefreshItems();
    });
    
    // 设置图标纹理管理器
    m_itemGrid.SetIconTextureManager(iconTextureManager);
    
    // 编辑保存后刷新
    m_editDialog.OnSave([this](const Item& item) {
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
    
    // 菜单栏
    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu(u8"文件")) {
            if (ImGui::MenuItem(u8"新建分类")) {
                Category cat;
                cat.id = GenerateId(L"cat");
                cat.name = L"新分类";
                m_itemManager->AddCategory(cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            if (ImGui::MenuItem(u8"删除当前分类", nullptr, false, !m_itemManager->GetCategories().empty())) {
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
            if (ImGui::MenuItem(u8"退出")) {
                PostQuitMessage(0);
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu(u8"编辑")) {
            if (ImGui::MenuItem(u8"重命名当前分类", nullptr, false, !m_itemManager->GetCategories().empty())) {
                // TODO: 实现分类重命名对话框
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
    
    if (m_showError) {
        ImGui::OpenPopup(u8"错误");
        if (ImGui::BeginPopupModal(u8"错误", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button(u8"确定", ImVec2(100, 0))) {
                m_showError = false;
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        }
    }
}

void MainWindow::RenderSearchBar() {
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(12, 10));
    ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 6.0f);
    ImGui::PushStyleColor(ImGuiCol_FrameBg, ImVec4(0.96f, 0.96f, 0.96f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_FrameBgHovered, ImVec4(0.94f, 0.94f, 0.94f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_FrameBgActive, ImVec4(0.92f, 0.92f, 0.92f, 1.0f));
    
    ImGui::SetNextItemWidth(-FLT_MIN);
    bool searchChanged = ImGui::InputTextWithHint("##search", u8"搜索...", m_searchBuf, sizeof(m_searchBuf));
    
    ImGui::PopStyleColor(3);
    ImGui::PopStyleVar(2);
    
    if (searchChanged) {
        std::string searchText(m_searchBuf);
        m_isSearching = !searchText.empty();
        
        if (m_isSearching) {
            UpdateSearchResults();
        } else {
            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        }
    }
    
    ImGui::Spacing();
    ImGui::Spacing();
}

void MainWindow::UpdateSearchResults() {
    m_searchResults.clear();
    
    std::string searchStr(m_searchBuf);
    std::wstring searchWStr = StringUtils::Utf8ToWString(searchStr);
    std::wstring searchLower = StringUtils::ToLower(searchWStr);
    
    for (const auto& cat : m_itemManager->GetCategories()) {
        for (const auto& item : m_itemManager->GetItems(cat.id)) {
            std::wstring nameLower = StringUtils::ToLower(item.name);
            bool matchName = nameLower.find(searchLower) != std::wstring::npos;
            
            std::wstring keywordsLower = StringUtils::ToLower(item.keywords);
            bool matchKeywords = keywordsLower.find(searchLower) != std::wstring::npos;
            
            std::wstring targetLower = StringUtils::ToLower(item.target);
            bool matchTarget = targetLower.find(searchLower) != std::wstring::npos;
            
            if (matchName || matchKeywords || matchTarget) {
                m_searchResults.push_back(item);
            }
        }
    }
    
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
}

} // namespace mn
