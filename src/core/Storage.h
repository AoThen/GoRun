#pragma once

#include "Types.h"
#include <string>
#include <vector>
#include <unordered_map>

namespace mn {

class Storage {
public:
    void Initialize(const std::wstring& path);
    bool Load(const std::wstring& path);
    bool Save(const std::wstring& path);
    
    std::vector<Category> GetCategories() const;
    Category GetCategory(const std::wstring& id) const;
    bool AddCategory(const Category& category);
    bool UpdateCategory(const Category& category);
    bool DeleteCategory(const std::wstring& id);
    
    std::vector<Item> GetItems(const std::wstring& categoryId = L"") const;
    Item GetItem(const std::wstring& id) const;
    bool AddItem(const Item& item);
    bool UpdateItem(const Item& item);
    bool DeleteItem(const std::wstring& id);
    
    std::vector<std::wstring> GetItemIdsByCategory(const std::wstring& categoryId) const;
    
    std::wstring GetConfig(const std::string& key, const std::wstring& defaultVal = L"") const;
    bool SetConfig(const std::string& key, const std::wstring& value);

private:
    bool SaveToFile();
    bool RotateBackups();
    
    std::wstring m_path;
    std::vector<Category> m_categories;
    std::vector<Item> m_items;
    std::unordered_map<std::string, std::wstring> m_config;
};

} // namespace mn