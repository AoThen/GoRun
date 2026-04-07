#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "utils/StringUtils.h"
#include <imgui.h>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
    });
    
    m_itemGrid.OnItemDoubleClicked([this](const Item& item) {
        RunResult result = m_runner->Run(item);
        if (!result.success) {
            ShowError(result.errorMessage);
        }
    });
    
    m_editDialog.OnSave([this](const Item& item) {
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    });
    
    auto& categories = itemManager->GetCategories();
    if (!categories.empty()) {
        m_currentCategoryId = categories[0].id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    }
}

void MainWindow::Render() {
    if (!m_visible) return;
    
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
    
    ImGui::BeginChild("Left", ImVec2(150, 0), true);
    m_categoryTab.Render();
    ImGui::EndChild();
    
    ImGui::SameLine();
    
    ImGui::BeginChild("Right", ImVec2(0, 0), true);
    m_itemGrid.Render();
    ImGui::EndChild();
    
    m_editDialog.Render();
    
    if (m_showError) {
        ImGui::OpenPopup("错误");
        if (ImGui::BeginPopupModal("错误", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button("确定")) {
                m_showError = false;
                ImGui::CloseCurrentPopup();
            }
            ImGui::EndPopup();
        }
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
