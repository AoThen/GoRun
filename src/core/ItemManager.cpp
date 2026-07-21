#include "ItemManager.h"
#include "Storage.h"
#include "IconCache.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include "utils/Logger.h"
#include <algorithm>
#include <cwctype>
#include <utility>

#ifdef _WIN32
#include <ShObjIdl.h>
#include <ShlObj.h>
#endif

namespace mn {

std::vector<Item> ItemManager::s_emptyItems;

void ItemManager::Initialize(Storage* storage, IconCache* iconCache) {
    m_storage = storage;
    m_iconCache = iconCache;
    
    m_categories = storage->GetCategories();
    auto allItems = storage->GetItems();
    
    for (const auto& item : allItems) {
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
    LOG_INFOW(L"ItemManager::AddCategory: " + category.name);
}

void ItemManager::UpdateCategory(const Category& category) {
    for (auto& cat : m_categories) {
        if (cat.id == category.id) {
            cat = category;
            m_storage->UpdateCategory(category);
            LOG_INFOW(L"ItemManager::UpdateCategory: " + category.name);
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
    LOG_INFOW(L"ItemManager::DeleteCategory: " + id);
}

const std::vector<Item>& ItemManager::GetItems(const std::wstring& categoryId) {
    auto it = m_itemsByCategory.find(categoryId);
    if (it != m_itemsByCategory.end()) {
        return it->second;
    }
    return s_emptyItems;
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
    
    // 自动设置 sortOrder 为当前分类最大序号+1
    auto& catItems = m_itemsByCategory[item.categoryId];
    item.sortOrder = catItems.empty() ? 0 : catItems.back().sortOrder + 1;
    
    if (m_iconCache) {
        m_iconCache->GetIconPath(item);
    }
    
    m_itemsByCategory[item.categoryId].push_back(item);
    m_storage->AddItem(item);
    LOG_INFOW(L"ItemManager::AddItem: " + item.name);
}

void ItemManager::UpdateItem(const Item& item) {
    // 更新 m_itemsByCategory - 支持跨分类变更
    for (auto& [catId, items] : m_itemsByCategory) {
        auto it = std::find_if(items.begin(), items.end(),
            [&](const Item& i) { return i.id == item.id; });
        if (it != items.end()) {
            if (catId != item.categoryId) {
                // categoryId 变更，移动到新分类
                items.erase(it);
                m_itemsByCategory[item.categoryId].push_back(item);
            } else {
                *it = item;
            }
            break;
        }
    }

    m_storage->UpdateItem(item);
    LOG_INFOW(L"ItemManager::UpdateItem: " + item.name);
}

void ItemManager::MoveItem(const std::wstring& itemId, const std::wstring& targetCategoryId) {
    // 从 m_itemsByCategory 中查找
    Item* itemInAll = nullptr;
    for (auto& [catId, items] : m_itemsByCategory) {
        for (auto& i : items) {
            if (i.id == itemId) {
                itemInAll = &i;
                break;
            }
        }
        if (itemInAll) break;
    }
    if (!itemInAll) return;

    std::wstring oldCatId = itemInAll->categoryId;
    if (oldCatId == targetCategoryId) return;  // 同分类无需移动

    itemInAll->categoryId = targetCategoryId;

    // 在 m_itemsByCategory 中移动
    auto& oldItems = m_itemsByCategory[oldCatId];
    auto it = std::find_if(oldItems.begin(), oldItems.end(),
        [&](const Item& i) { return i.id == itemId; });
    if (it != oldItems.end()) {
        m_itemsByCategory[targetCategoryId].push_back(std::move(*it));
        oldItems.erase(it);
    }

    m_storage->UpdateItem(*itemInAll);
    LOG_INFOW(L"ItemManager::MoveItem: " + itemInAll->name + L" -> " + targetCategoryId);
}

void ItemManager::DeleteItem(const std::wstring& id) {
    Item* item = GetItem(id);
    if (item) {
        LOG_INFOW(L"ItemManager::DeleteItem: " + item->name);
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
    if (m_storage) m_storage->BeginBatch();
    for (const auto& file : files) {
        Item item;
        item.id = GenerateId(L"item");
        item.categoryId = categoryId;
        item.name = PathUtils::GetFileBaseName(file);
        item.target = file;
        item.workingDir = PathUtils::GetParentDir(file);
        
        // 检查是否为快捷方式（大小写不敏感）
        std::wstring ext = file.size() > 4 ? file.substr(file.size() - 4) : L"";
        std::transform(ext.begin(), ext.end(), ext.begin(), ::towlower);
        if (ext == L".lnk") {
            ShortcutInfo shortcut = ResolveShortcut(file);
            if (shortcut.success) {
                item.target = shortcut.target;
                item.arguments = shortcut.arguments;
                item.workingDir = shortcut.workingDir;
                if (!shortcut.iconPath.empty()) {
                    item.iconPath = shortcut.iconPath;
                    item.iconIndex = shortcut.iconIndex;
                }
                // 如果快捷方式有描述名称，优先使用
                if (!shortcut.name.empty()) {
                    item.name = shortcut.name;
                } else {
                    // 否则从目标文件获取名称
                    item.name = PathUtils::GetFileBaseName(shortcut.target);
                }
            }
        }
        
        AddItem(item);
    }
    if (m_storage) m_storage->EndBatch();
}

ItemManager::ShortcutInfo ItemManager::ResolveShortcut(const std::wstring& lnkPath) {
    ShortcutInfo info;
    
#ifdef _WIN32
    // COM 已在 App::Initialize() 中初始化
    IShellLinkW* pShellLink = nullptr;
    IPersistFile* pPersistFile = nullptr;
    HRESULT hr;
    
    hr = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER, 
                          IID_IShellLinkW, reinterpret_cast<void**>(&pShellLink));
    if (FAILED(hr)) {
        LOG_ERRORW(L"ItemManager::ResolveShortcut: CoCreateInstance failed");
        return info;
    }
    
    hr = pShellLink->QueryInterface(IID_IPersistFile, reinterpret_cast<void**>(&pPersistFile));
    if (FAILED(hr)) {
        LOG_ERRORW(L"ItemManager::ResolveShortcut: QueryInterface failed");
        pShellLink->Release();
        return info;
    }
    
    hr = pPersistFile->Load(lnkPath.c_str(), STGM_READ);
    if (FAILED(hr)) {
        LOG_ERRORW(L"ItemManager::ResolveShortcut: IPersistFile::Load failed");
        pPersistFile->Release();
        pShellLink->Release();
        return info;
    }
    
    // 解析快捷方式（查找原始目标）
    hr = pShellLink->Resolve(nullptr, SLR_NO_UI | SLR_NOUPDATE);
    
    // 获取目标路径
    wchar_t targetPath[MAX_PATH] = {0};
    hr = pShellLink->GetPath(targetPath, MAX_PATH, nullptr, 0);
    if (SUCCEEDED(hr) && targetPath[0] != L'\0') {
        info.target = targetPath;
    }
    
    // 获取参数
    wchar_t arguments[1024] = {0};
    hr = pShellLink->GetArguments(arguments, 1024);
    if (SUCCEEDED(hr) && arguments[0] != L'\0') {
        info.arguments = arguments;
    }
    
    // 获取工作目录
    wchar_t workingDir[MAX_PATH] = {0};
    hr = pShellLink->GetWorkingDirectory(workingDir, MAX_PATH);
    if (SUCCEEDED(hr) && workingDir[0] != L'\0') {
        info.workingDir = workingDir;
    }
    
    // 获取图标路径和索引
    wchar_t iconPath[MAX_PATH] = {0};
    int iconIndex = 0;
    hr = pShellLink->GetIconLocation(iconPath, MAX_PATH, &iconIndex);
    if (SUCCEEDED(hr) && iconPath[0] != L'\0') {
        info.iconPath = iconPath;
        info.iconIndex = iconIndex;
    }
    
    // 获取快捷方式描述名称
    wchar_t description[MAX_PATH] = {0};
    hr = pShellLink->GetDescription(description, MAX_PATH);
    if (SUCCEEDED(hr) && description[0] != L'\0') {
        info.name = description;
    }
    
    // 如果没有自定义图标，使用目标路径作为图标路径
    if (info.iconPath.empty() && !info.target.empty()) {
        info.iconPath = info.target;
        info.iconIndex = 0;
    }
    
    pPersistFile->Release();
    pShellLink->Release();
    
    info.success = !info.target.empty();
    if (info.success) {
        LOG_INFOW(L"ItemManager::ResolveShortcut: resolved " + lnkPath + L" -> " + info.target);
    } else {
        LOG_ERRORW(L"ItemManager::ResolveShortcut: failed to resolve " + lnkPath);
    }
#endif
    
    return info;
}

std::vector<Item> ItemManager::SearchItems(const std::wstring& query) {
    std::vector<std::pair<Item, int>> scoredResults;
    
    if (query.empty()) {
        return {};
    }
    
    for (const auto& [catId, items] : m_itemsByCategory) {
        for (const auto& item : items) {
            auto matchResult = StringUtils::SearchMatch(item.name, item.keywords, query);
            if (matchResult.matched) {
                // 综合得分 = 匹配得分 + 运行次数权重
                int totalScore = matchResult.score + item.runCount;
                scoredResults.push_back({item, totalScore});
            }
        }
    }
    
    // 按得分降序排序
    std::sort(scoredResults.begin(), scoredResults.end(),
        [](const auto& a, const auto& b) {
            return a.second > b.second;
        });
    
    std::vector<Item> results;
    for (const auto& pair : scoredResults) {
        results.push_back(pair.first);
    }
    
    return results;
}

void ItemManager::IncrementRunCount(const std::wstring& itemId) {
    Item* item = GetItem(itemId);
    if (item) {
        item->runCount++;
        UpdateItem(*item);
    }
}

} // namespace mn
