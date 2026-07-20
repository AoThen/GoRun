#pragma once

#include "widgets/CategoryTab.h"
#include "widgets/ItemGrid.h"
#include "dialogs/EditDialog.h"
#include "core/Types.h"
#include <string>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class ItemManager;
class Config;
class Runner;
class IconTextureManager;

class MainWindow {
public:
#ifdef _WIN32
    void Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager, HWND hwnd);
#else
    void Initialize(ItemManager* itemManager, Config* config, Runner* runner, IconTextureManager* iconTextureManager);
#endif
    void Render();
    
    void Show();
    void Hide();
    void Toggle();
    bool IsVisible() const;
    void SetCurrentCategory(const std::wstring& categoryId);
    const std::wstring& GetCurrentCategoryId() const { return m_currentCategoryId; }
    void ToggleViewType();

private:
    void RenderSearchBar();
    void UpdateSearchResults();
    void RefreshItems();
    void ShowError(const std::wstring& message);
    void RenderRenameCategoryDialog();
    void RenderDeleteConfirmDialog();
    void HandleNewItem();
    
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
    float m_lastSearchTime = 0.0f;
    float m_searchDelay = 0.3f;  // 300ms 延迟
    char m_pendingSearchBuf[256] = {};
    bool m_searchPending = false;
    bool m_focusSearch = true;   // 首次显示时聚焦搜索框
    int m_searchSelectedIndex = -1;
    
    std::wstring m_errorMessage;
    bool m_showError = false;
    bool m_openErrorPopup = false;  // 控制错误弹窗首次打开
    
    // 分类重命名对话框
    bool m_showRenameCategory = false;
    bool m_openRenamePopup = false;  // 控制弹窗首次打开
    char m_renameCategoryBuf[256] = {};
    
    // 视图切换
    ViewType m_currentViewType = ViewType::Icon;
    
    // 删除确认对话框
    bool m_showDeleteConfirm = false;
    bool m_openDeletePopup = false;
    std::wstring m_deleteItemId;
    std::wstring m_deleteItemName;

    // 新建项目
    Item m_newItem;
    bool m_editingNewItem = false;
};

} // namespace mn
