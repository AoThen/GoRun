#include "MainWindow.h"
#include "core/ItemManager.h"
#include "core/Config.h"
#include "core/Runner.h"
#include "core/IconTextureManager.h"
#include "core/Localization.h"
#include "utils/StringUtils.h"
#include "ui/Theme.h"
#include "app/App.h"
#include <imgui.h>
#include <algorithm>
#include <windows.h>
#include <shellapi.h>

namespace mn {

void MainWindow::Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager, HWND hwnd) {
    m_itemManager = itemManager;
    m_config = config;
    m_runner = runner;
    m_iconTextureManager = iconTextureManager;
    m_editDialog.SetOwner(hwnd);
    
    // 恢复保存的主题
    ApplyTheme(static_cast<ThemeType>(m_config->GetTheme()));
    
    m_categoryTab.SetCategories(&itemManager->GetCategories());
    
    m_categoryTab.OnCategoryChanged([this](const std::wstring& id) {
        m_currentCategoryId = id;
        m_searchBuf[0] = '\0';
        m_isSearching = false;
        m_itemGrid.SetItems(&m_itemManager->GetItems(id));
        Category* cat = m_itemManager->GetCategory(id);
        if (cat) {
            m_currentViewType = cat->viewType;
            m_itemGrid.SetViewType(m_currentViewType);
        }
    });
    
    // 新建分类
    m_categoryTab.OnCategoryAdd([this]() {
        Category cat;
        cat.id = GenerateId(L"cat");
        cat.name = StringUtils::Utf8ToWString(TrUtf8("Default_NewCategory"));
        m_itemManager->AddCategory(cat);
        m_categoryTab.SetCategories(&m_itemManager->GetCategories());
        m_categoryTab.SetCurrentCategory(cat.id);
        m_currentCategoryId = cat.id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(cat.id));
        // 自动打开重命名对话框
        std::string defaultNewCat = TrUtf8("Default_NewCategory");
        strncpy(m_renameCategoryBuf, defaultNewCat.c_str(), sizeof(m_renameCategoryBuf) - 1);
        m_renameCategoryBuf[sizeof(m_renameCategoryBuf) - 1] = '\0';
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
            ShowError(StringUtils::Utf8ToWString(TrUtf8("Tip_NeedOneCategory")));
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
        m_editingNewItem = false;
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
        if (m_editingNewItem) {
            m_itemManager->AddItem(item);
            m_editingNewItem = false;
        } else {
            m_itemManager->UpdateItem(item);
        }
        RefreshItems();
    });
    
    // 新建项目回调
    m_itemGrid.OnItemAdd([this]() {
        HandleNewItem();
    });
    
    // 右键菜单回调
    m_itemGrid.OnItemCopyPath([this](const Item& item) {
        if (OpenClipboard(nullptr)) {
            EmptyClipboard();
            std::wstring path = item.target;
            HGLOBAL hGlobal = GlobalAlloc(GMEM_MOVEABLE, (path.size() + 1) * sizeof(wchar_t));
            if (hGlobal) {
                wcscpy_s(static_cast<wchar_t*>(GlobalLock(hGlobal)), path.size() + 1, path.c_str());
                GlobalUnlock(hGlobal);
                SetClipboardData(CF_UNICODETEXT, hGlobal);
            }
            CloseClipboard();
        }
    });
    m_itemGrid.OnItemOpenLocation([this](const Item& item) {
        std::wstring dir = item.target.substr(0, item.target.find_last_of(L"\\/"));
        if (!dir.empty()) {
            std::wstring escaped = item.target;
            size_t pos = 0;
            while ((pos = escaped.find(L'\"', pos)) != std::wstring::npos) {
                escaped.replace(pos, 1, L"\"\"");
                pos += 2;
            }
            ShellExecuteW(nullptr, L"open", L"explorer.exe", (L"/select,\"" + escaped + L"\"").c_str(), nullptr, SW_SHOW);
        }
    });
    m_itemGrid.OnItemMoveToCategory([this](const Item& item, const std::wstring& categoryId) {
        App::Get()->GetItemManager()->MoveItem(item.id, categoryId);
    });
    m_itemGrid.OnItemProperties([this](const Item& item) {
        SHELLEXECUTEINFOW sei = { sizeof(sei) };
        sei.lpVerb = L"properties";
        sei.lpFile = item.target.c_str();
        sei.nShow = SW_SHOW;
        ShellExecuteExW(&sei);
    });
    m_itemGrid.SetAllCategories(&itemManager->GetCategories());

    // 项目拖拽排序
    m_itemGrid.OnReorderItems([this](const std::vector<std::wstring>& orderedIds) {
        m_itemManager->ReorderItems(m_currentCategoryId, orderedIds);
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
    });

    // 分类拖拽排序
    m_categoryTab.OnCategoryOrderChanged([this](const std::vector<std::wstring>& orderedIds) {
        m_itemManager->ReorderCategories(orderedIds);
        m_categoryTab.SetCategories(&m_itemManager->GetCategories());
        if (!m_itemManager->GetCategories().empty()) {
            m_currentCategoryId = m_itemManager->GetCategories()[0].id;
            m_categoryTab.SetCurrentCategory(m_currentCategoryId);
            m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        }
    });
    
    auto& categories = itemManager->GetCategories();
    if (!categories.empty()) {
        m_currentCategoryId = categories[0].id;
        m_itemGrid.SetItems(&m_itemManager->GetItems(m_currentCategoryId));
        Category* cat = m_itemManager->GetCategory(m_currentCategoryId);
        if (cat) {
            m_currentViewType = cat->viewType;
            m_itemGrid.SetViewType(m_currentViewType);
        }
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
        if (ImGui::BeginMenu(TrUtf8("Menu_File").c_str())) {
            if (ImGui::MenuItem(TrUtf8("Menu_NewItem").c_str())) {
                HandleNewItem();
            }
            if (ImGui::MenuItem(TrUtf8("Menu_NewC").c_str())) {
                Category cat;
                cat.id = GenerateId(L"cat");
cat.name = StringUtils::Utf8ToWString(TrUtf8("Default_NewCategory"));
                m_itemManager->AddCategory(cat);
                m_categoryTab.SetCategories(&m_itemManager->GetCategories());
                m_currentCategoryId = cat.id;
                m_categoryTab.SetCurrentCategory(cat.id);
                m_itemGrid.SetItems(&m_itemManager->GetItems(cat.id));
                std::string name = StringUtils::WStringToUtf8(cat.name);
                strncpy(m_renameCategoryBuf, name.c_str(), sizeof(m_renameCategoryBuf) - 1);
                m_renameCategoryBuf[sizeof(m_renameCategoryBuf) - 1] = '\0';
                m_showRenameCategory = true;
                m_openRenamePopup = true;
            }
            if (ImGui::MenuItem(TrUtf8("Menu_DelCurrentC").c_str(), nullptr, false, !m_itemManager->GetCategories().empty())) {
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
                        ShowError(StringUtils::Utf8ToWString(TrUtf8("Tip_NeedOneCategory")));
                    }
                }
            }
            ImGui::Separator();
            if (ImGui::MenuItem(TrUtf8("Menu_Exit").c_str())) {
                App::Get()->Quit();
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu(TrUtf8("Menu_Edit").c_str())) {
            if (ImGui::MenuItem(TrUtf8("Menu_RenameCurrentC").c_str(), nullptr, false, !m_itemManager->GetCategories().empty())) {
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
        if (ImGui::BeginMenu(TrUtf8("Menu_View").c_str())) {
            if (ImGui::MenuItem(TrUtf8("Menu_ViewIcon0").c_str(), nullptr, m_currentViewType == ViewType::Icon)) {
                if (m_currentViewType != ViewType::Icon) ToggleViewType();
            }
            if (ImGui::MenuItem(TrUtf8("Menu_ViewIcon1").c_str(), nullptr, m_currentViewType == ViewType::List)) {
                if (m_currentViewType != ViewType::List) ToggleViewType();
            }
            ImGui::Separator();
            const char* themeLabel = (GetCurrentTheme() == ThemeType::Light) ? TrUtf8("Menu_SwitchDark").c_str() : TrUtf8("Menu_SwitchLight").c_str();
            if (ImGui::MenuItem(themeLabel)) {
                ToggleTheme();
                if (m_config) {
                    m_config->SetTheme(static_cast<int>(GetCurrentTheme()));
                }
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu(TrUtf8("Menu_Set").c_str())) {
            if (ImGui::MenuItem(TrUtf8("SSTR_010002").c_str(), nullptr, m_config && m_config->GetAutoStart())) {
                if (m_config) {
                    m_config->SetAutoStart(!m_config->GetAutoStart());
                }
            }
            if (ImGui::BeginMenu(TrUtf8("Menu_Language").c_str())) {
                std::string currentLang = "zh-CN";
                if (auto* loc = Localization::Get()) {
                    currentLang = loc->GetCurrentLanguage();
                }
                if (ImGui::MenuItem("简体中文", nullptr, currentLang == "zh-CN")) {
                    if (auto* loc = Localization::Get()) loc->SetLanguage("zh-CN");
                    if (m_config) m_config->SetLanguage("zh-CN");
                }
                if (ImGui::MenuItem("繁體中文", nullptr, currentLang == "zh-TW")) {
                    if (auto* loc = Localization::Get()) loc->SetLanguage("zh-TW");
                    if (m_config) m_config->SetLanguage("zh-TW");
                }
                if (ImGui::MenuItem("English", nullptr, currentLang == "en-US")) {
                    if (auto* loc = Localization::Get()) loc->SetLanguage("en-US");
                    if (m_config) m_config->SetLanguage("en-US");
                }
                ImGui::EndMenu();
            }
            if (ImGui::MenuItem(TrUtf8("Menu_FollowMouse").c_str(), nullptr, m_config && m_config->GetFollowMouse())) {
                if (m_config) {
                    m_config->SetFollowMouse(!m_config->GetFollowMouse());
                }
            }
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu(TrUtf8("Menu_Help").c_str())) {
            if (ImGui::MenuItem(TrUtf8("Menu_About").c_str())) {
                ShowError(StringUtils::Utf8ToWString(TrUtf8("About_Info")));
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
    
    m_editDialog.SetCategories(m_itemManager->GetCategories());
    m_editDialog.Render();
    
    // 分类重命名对话框
    RenderRenameCategoryDialog();
    
    // 删除确认对话框
    RenderDeleteConfirmDialog();
    
    if (m_showError) {
        // 只在首次显示时打开弹窗
        if (m_openErrorPopup) {
            ImGui::OpenPopup(TrUtf8("Title_Error").c_str());
            m_openErrorPopup = false;
        }
        
        if (ImGui::BeginPopupModal(TrUtf8("Title_Error").c_str(), nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
            ImGui::Text("%s", StringUtils::WStringToUtf8(m_errorMessage).c_str());
            if (ImGui::Button(TrUtf8("Btn_OK").c_str(), ImVec2(100, 0))) {
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
    bool searchChanged = ImGui::InputTextWithHint("##search", TrUtf8("Tip_Search").c_str(), m_searchBuf, sizeof(m_searchBuf));
    
    ImGui::PopStyleColor(3);
    ImGui::PopStyleVar(2);
    
    if (searchChanged) {
        m_lastSearchTime = ImGui::GetTime();
        strncpy(m_pendingSearchBuf, m_searchBuf, sizeof(m_pendingSearchBuf) - 1);
        m_pendingSearchBuf[sizeof(m_pendingSearchBuf) - 1] = '\0';
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
            m_itemGrid.SetSearchQuery(L"");
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
    
    const char* viewLabel = (m_currentViewType == ViewType::Icon) ? TrUtf8("Menu_ViewIcon0").c_str() : TrUtf8("Menu_ViewIcon1").c_str();
    if (ImGui::Button(viewLabel, ImVec2(50, 0))) {
        ToggleViewType();
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("%s", (m_currentViewType == ViewType::Icon) ? TrUtf8("Tip_SwitchListView").c_str() : TrUtf8("Tip_SwitchIconView").c_str());
    }
    
    // 主题切换按钮
    ImGui::SameLine(0, 4);
    const char* themeLabel = (GetCurrentTheme() == ThemeType::Light) ? TrUtf8("Btn_DarkMode").c_str() : TrUtf8("Btn_LightMode").c_str();
    if (ImGui::Button(themeLabel, ImVec2(50, 0))) {
        ToggleTheme();
        if (m_config) {
            m_config->SetTheme(static_cast<int>(GetCurrentTheme()));
        }
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("%s", (GetCurrentTheme() == ThemeType::Light) ? TrUtf8("Tip_SwitchDark").c_str() : TrUtf8("Tip_SwitchLight").c_str());
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
    m_itemGrid.SetSearchQuery(searchWStr);
    m_runner->SetSearchQuery(searchWStr);
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
        ImGui::OpenPopup(TrUtf8("Title_RenameCategory").c_str());
        m_openRenamePopup = false;
    }
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(300, 0));
    
    if (ImGui::BeginPopupModal(TrUtf8("Title_RenameCategory").c_str(), nullptr, ImGuiWindowFlags_NoResize)) {
        if (ImGui::InputText(TrUtf8("STR_CategoryName").c_str(), m_renameCategoryBuf, sizeof(m_renameCategoryBuf), ImGuiInputTextFlags_EnterReturnsTrue)) {
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
        
        if (ImGui::Button(TrUtf8("Btn_OK").c_str(), ImVec2(100, 0))) {
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
        if (ImGui::Button(TrUtf8("Btn_Cancel").c_str(), ImVec2(100, 0))) {
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
        ImGui::OpenPopup(TrUtf8("Title_ConfirmDelete").c_str());
        m_openDeletePopup = false;
    }
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(320, 0));
    
    if (ImGui::BeginPopupModal(TrUtf8("Title_ConfirmDelete").c_str(), nullptr, ImGuiWindowFlags_NoResize)) {
        std::string displayName = StringUtils::WStringToUtf8(m_deleteItemName);
        if (displayName.length() > 30) {
            displayName = displayName.substr(0, 30) + "...";
        }
        std::string delFmt = TrUtf8("Tip_DelItemFormat");
        size_t pos = delFmt.find("%s");
        if (pos != std::string::npos) {
            delFmt.replace(pos, 2, displayName);
        }
        ImGui::Text("%s", delFmt.c_str());
        ImGui::Text("%s", TrUtf8("Tip_UndoNotPossible").c_str());
        ImGui::Separator();
        
        if (ImGui::Button(TrUtf8("Btn_ConfirmDelete").c_str(), ImVec2(120, 0))) {
            m_itemGrid.ClearHoverAnimation(m_deleteItemId);
            m_itemManager->DeleteItem(m_deleteItemId);
            RefreshItems();
            m_showDeleteConfirm = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(TrUtf8("Btn_Cancel").c_str(), ImVec2(120, 0))) {
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
    Category* cat = m_itemManager->GetCategory(m_currentCategoryId);
    if (cat) {
        cat->viewType = m_currentViewType;
        m_itemManager->UpdateCategory(*cat);
    }
}

void MainWindow::HandleNewItem() {
    m_editingNewItem = false;
    if (m_currentCategoryId.empty()) {
        auto& categories = m_itemManager->GetCategories();
        if (categories.empty()) return;
        m_currentCategoryId = categories[0].id;
    }
    m_newItem = Item();
    m_newItem.id = GenerateId(L"item");
    m_newItem.categoryId = m_currentCategoryId;
    m_editingNewItem = true;
    m_editDialog.Show(&m_newItem);
}

} // namespace mn
