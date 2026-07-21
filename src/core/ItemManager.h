#pragma once

#include "Types.h"
#include <vector>
#include <unordered_map>

namespace mn {

class Storage;
class IconCache;

class ItemManager {
public:
    void Initialize(Storage* storage, IconCache* iconCache);
    
    std::vector<Category>& GetCategories();
    Category* GetCategory(const std::wstring& id);
    void AddCategory(Category category);
    void UpdateCategory(const Category& category);
    void DeleteCategory(const std::wstring& id);
    
    const std::vector<Item>& GetItems(const std::wstring& categoryId);
    Item* GetItem(const std::wstring& id);
    void AddItem(Item item);
    void UpdateItem(const Item& item);
    void MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId);
    void DeleteItem(const std::wstring& id);
    
    void RefreshItemIcon(const std::wstring& itemId);
    void HandleDrop(const std::vector<std::wstring>& files, const std::wstring& categoryId);
    
    std::vector<Item> SearchItems(const std::wstring& query);
    void IncrementRunCount(const std::wstring& itemId);
    
    // 快捷方式解析
    struct ShortcutInfo {
        std::wstring target;
        std::wstring arguments;
        std::wstring workingDir;
        std::wstring iconPath;
        int iconIndex = 0;
        bool success = false;
        std::wstring name;  // 快捷方式描述名称
    };
    ShortcutInfo ResolveShortcut(const std::wstring& lnkPath);

private:
    Storage* m_storage = nullptr;
    IconCache* m_iconCache = nullptr;
    std::vector<Category> m_categories;
    std::unordered_map<std::wstring, std::vector<Item>> m_itemsByCategory;
    static std::vector<Item> s_emptyItems;
};

} // namespace mn
