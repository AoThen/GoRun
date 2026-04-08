#include "ItemManager.h"
#include "Storage.h"
#include "IconCache.h"
#include "utils/PathUtils.h"
#include <algorithm>

#ifdef _WIN32
#include <ShObjIdl.h>
#include <ShlObj.h>
#endif

namespace mn {

void ItemManager::Initialize(Storage* storage, IconCache* iconCache) {
    m_storage = storage;
    m_iconCache = iconCache;
    
    m_categories = storage->GetCategories();
    m_allItems = storage->GetItems();
    
    for (const auto& item : m_allItems) {
        m_itemsByCategory[item.categoryId].push_back(item);
    }
    
    if (m_categories.empty()) {
        Category defaultCat;
        defaultCat.id = GenerateId(L"cat");
        defaultCat.name = L"默认";
        AddCategory(defaultCat);
    }
}

std::vector<Category>& ItemManager::GetCategories() {
    return m_categories;
}

Category* ItemManager::GetCategory(const std::wstring& id) {
    for (auto& cat : m_categories) {
        if (cat.id == id) return &cat;
    }
    return nullptr;
}

void ItemManager::AddCategory(Category category) {
    if (category.id.empty()) {
        category.id = GenerateId(L"cat");
    }
    m_categories.push_back(category);
    m_storage->AddCategory(category);
}

void ItemManager::UpdateCategory(const Category& category) {
    for (auto& cat : m_categories) {
        if (cat.id == category.id) {
            cat = category;
            m_storage->UpdateCategory(category);
            break;
        }
    }
}

void ItemManager::DeleteCategory(const std::wstring& id) {
    auto it = m_itemsByCategory.find(id);
    if (it != m_itemsByCategory.end()) {
        for (const auto& item : it->second) {
            m_iconCache->DeleteCache(item.id);
        }
        m_itemsByCategory.erase(it);
    }
    
    m_storage->DeleteCategory(id);
    
    m_categories.erase(
        std::remove_if(m_categories.begin(), m_categories.end(),
            [&](const Category& c) { return c.id == id; }),
        m_categories.end()
    );
}

std::vector<Item>& ItemManager::GetItems(const std::wstring& categoryId) {
    return m_itemsByCategory[categoryId];
}

Item* ItemManager::GetItem(const std::wstring& id) {
    for (auto& [catId, items] : m_itemsByCategory) {
        for (auto& item : items) {
            if (item.id == id) return &item;
        }
    }
    return nullptr;
}

void ItemManager::AddItem(Item item) {
    if (item.id.empty()) {
        item.id = GenerateId(L"item");
    }
    
    if (m_iconCache) {
        m_iconCache->GetIconPath(item);
    }
    
    m_itemsByCategory[item.categoryId].push_back(item);
    m_allItems.push_back(item);
    m_storage->AddItem(item);
}

void ItemManager::UpdateItem(const Item& item) {
    // 更新 m_allItems
    for (auto& i : m_allItems) {
        if (i.id == item.id) {
            i = item;
            break;
        }
    }
    
    // 更新 m_itemsByCategory
    for (auto& [catId, items] : m_itemsByCategory) {
        for (auto& i : items) {
            if (i.id == item.id) {
                i = item;
                break;
            }
        }
    }
    
    m_storage->UpdateItem(item);
}

void ItemManager::MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId) {
    Item* item = GetItem(itemId);
    if (item) {
        std::wstring oldCatId = item->categoryId;
        item->categoryId = targetCategoryId;
        
        auto& oldItems = m_itemsByCategory[oldCatId];
        oldItems.erase(
            std::remove_if(oldItems.begin(), oldItems.end(),
                [&](const Item& i) { return i.id == itemId; }),
            oldItems.end()
        );
        m_itemsByCategory[targetCategoryId].push_back(*item);
        
        m_storage->UpdateItem(*item);
    }
}

void ItemManager::DeleteItem(const std::wstring& id) {
    Item* item = GetItem(id);
    if (item) {
        std::wstring catId = item->categoryId;
        
        if (m_iconCache) {
            m_iconCache->DeleteCache(id);
        }
        
        auto& items = m_itemsByCategory[catId];
        items.erase(
            std::remove_if(items.begin(), items.end(),
                [&](const Item& i) { return i.id == id; }),
            items.end()
        );
        
        m_allItems.erase(
            std::remove_if(m_allItems.begin(), m_allItems.end(),
                [&](const Item& i) { return i.id == id; }),
            m_allItems.end()
        );
        
        m_storage->DeleteItem(id);
    }
}

void ItemManager::RefreshItemIcon(const std::wstring& itemId) {
    Item* item = GetItem(itemId);
    if (item && m_iconCache) {
        m_iconCache->RefreshIcon(*item);
    }
}

void ItemManager::HandleDrop(const std::vector<std::wstring>& files, const std::wstring& categoryId) {
    for (const auto& file : files) {
        Item item;
        item.id = GenerateId(L"item");
        item.categoryId = categoryId;
        item.name = PathUtils::GetFileName(file);
        item.target = file;
        item.workingDir = PathUtils::GetParentDir(file);
        
        AddItem(item);
    }
}

} // namespace mn
