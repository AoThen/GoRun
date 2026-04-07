#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "utils/StringUtils.h"
#include <imgui.h>
#include <algorithm>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_searchBuf[0] = '\0';
        m_isSearching = false;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
    });
    
    m_itemGrid.OnItemClicked([this](const Item& item) {
        RunResult result = m_runner->Run(item);
        if (!result.success) {
            ShowError(result.errorMessage);
        }
    });
    
    m_editDialog.OnSave([this](const Item& item) {
        if (m_isSearching) {
            UpdateSearchResults();
        } else {
            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        }
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
        if (ImGui::BeginMenu("文件")) {
            if (ImGui::MenuItem("新建分类")) {
                Category cat;
                cat.id = GenerateId(L"cat");
                cat.name = L"新分类";
                m_itemManager->AddCategory(cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
            }
            ImGui::EndMenu();
        }
        ImGui::EndMenuBar();
    }
    
    // 搜索栏
    RenderSearchBar();
    
    // 主布局
    ImGui::BeginChild("MainContent", ImVec2(0, 0), false);
    
    // 左侧分类
    ImGui::BeginChild("Left", ImVec2(160, 0), false);
    m_categoryTab.Render();
    ImGui::EndChild();
    
    ImGui::SameLine(0, 0);
    
    // 右侧项目
    ImGui::BeginChild("Right", ImVec2(0, 0), false);
    m_itemGrid.Render();
    ImGui::EndChild();
    
    ImGui::EndChild();
    
    m_editDialog.Render();
    
    if (m_showError) {
        ImGui::OpenPopup("错误");
        if (ImGui::BeginPopupModal("错误", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button("确定", ImVec2(100, 0))) {
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
    
    // 搜索图标 + 输入框
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
    
    // 搜索所有分类中的项目
    for (const auto& cat : m_itemManager->GetCategories()) {
        for (const auto& item : m_itemManager->GetItems(cat.id)) {
            // 匹配名称
            std::wstring nameLower = StringUtils::ToLower(item.name);
            bool matchName = nameLower.find(searchLower) != std::wstring::npos;
            
            // 匹配关键词
            std::wstring keywordsLower = StringUtils::ToLower(item.keywords);
            bool matchKeywords = keywordsLower.find(searchLower) != std::wstring::npos;
            
            // 匹配路径
            std::wstring targetLower = StringUtils::ToLower(item.target);
            bool matchTarget = targetLower.find(searchLower) != std::wstring::npos;
            
            if (matchName || matchKeywords || matchTarget) {
                m_searchResults.push_back(item);
            }
        }
    }
    
    m_itemGrid.SetItems(&m_searchResults);
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
