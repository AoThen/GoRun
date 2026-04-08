#pragma once

#include "widgets/CategoryTab.h"
#include "widgets/ItemGrid.h"
#include "dialogs/EditDialog.h"
#include "core/Types.h"
#include <string>

namespace mn {

class ItemManager;
class Config;
class Runner;
class IconTextureManager;

class MainWindow {
public:
    void Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager);
    void Render();
    
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    void SetCurrentCategory(const std::wstring& categoryId);

private:
    void RenderMenuBar();
    void RenderSearchBar();
    void UpdateSearchResults();
    void RefreshItems();
    void ShowError(const std::wstring& message);
    void RenderRenameCategoryDialog();
    
    ItemManager* m_itemManager = nullptr;
    Config* m_config = nullptr;
    Runner* m_runner = nullptr;
    IconTextureManager* m_iconTextureManager = nullptr;
    
    CategoryTab m_categoryTab;
    ItemGrid m_itemGrid;
    EditDialog m_editDialog;
    
    std::wstring m_currentCategoryId;
    bool m_visible = false;
    
    // 搜索相关
    char m_searchBuf[256] = {};
    bool m_isSearching = false;
    std::vector<Item> m_searchResults;
    
    std::wstring m_errorMessage;
    bool m_showError = false;
    
    // 分类重命名对话框
    bool m_showRenameCategory = false;
    char m_renameCategoryBuf[256] = {};
};

} // namespace mn
