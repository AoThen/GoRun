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

class MainWindow {
public:
    void Initialize(ItemManager* itemManager, Config* config, Runner* runner);
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
    void ShowError(const std::wstring& message);
    
    ItemManager* m_itemManager = nullptr;
    Config* m_config = nullptr;
    Runner* m_runner = nullptr;
    
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
};

} // namespace mn
