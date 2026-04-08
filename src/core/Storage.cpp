#include "Storage.h"
#include "utils/StringUtils.h"
#include "utils/PathUtils.h"
#include <nlohmann/json.hpp>
#include <fstream>
#include <algorithm>
#include <windows.h>

namespace mn {

using json = nlohmann::json;

void Storage::Initialize(const std::wstring& path) {
    m_path = path;
    // 创建空的 JSON 结构，确保后续保存时有数据
    if (!PathUtils::Exists(path)) {
        SaveToFile();
    }
}

bool Storage::Load(const std::wstring& path) {
    m_path = path;
    
    std::ifstream file(path);
    if (!file.is_open()) return false;
    
    try {
        json j;
        file >> j;
        
        if (j.contains("categories") && j["categories"].is_array()) {
            for (const auto& cat : j["categories"]) {
                Category c;
                c.id = StringUtils::Utf8ToWString(cat.value("id", ""));
                c.name = StringUtils::Utf8ToWString(cat.value("name", ""));
                c.sortOrder = cat.value("sortOrder", 0);
                c.viewType = static_cast<ViewType>(cat.value("viewType", 0));
                c.iconSize = cat.value("iconSize", 48);
                m_categories.push_back(c);
            }
        }
        
        if (j.contains("items") && j["items"].is_array()) {
            for (const auto& item : j["items"]) {
                Item i;
                i.id = StringUtils::Utf8ToWString(item.value("id", ""));
                i.name = StringUtils::Utf8ToWString(item.value("name", ""));
                i.target = StringUtils::Utf8ToWString(item.value("target", ""));
                i.arguments = StringUtils::Utf8ToWString(item.value("arguments", ""));
                i.workingDir = StringUtils::Utf8ToWString(item.value("workingDir", ""));
                i.iconPath = StringUtils::Utf8ToWString(item.value("iconPath", ""));
                i.iconIndex = item.value("iconIndex", 0);
                i.runAsAdmin = item.value("runAsAdmin", false);
                i.runCount = item.value("runCount", 0);
                i.keywords = StringUtils::Utf8ToWString(item.value("keywords", ""));
                i.remark = StringUtils::Utf8ToWString(item.value("remark", ""));
                i.categoryId = StringUtils::Utf8ToWString(item.value("categoryId", ""));
                i.sortOrder = item.value("sortOrder", 0);
                m_items.push_back(i);
            }
        }
        
        if (j.contains("config") && j["config"].is_object()) {
            for (auto& [key, val] : j["config"].items()) {
                if (val.is_string()) {
                    m_config[key] = StringUtils::Utf8ToWString(val.get<std::string>());
                }
            }
        }
        
        return true;
    } catch (const std::exception& e) {
        // 输出错误日志到调试窗口
        OutputDebugStringA("Storage::Load failed: ");
        OutputDebugStringA(e.what());
        OutputDebugStringA("\n");
        return false;
    } catch (...) {
        OutputDebugStringA("Storage::Load failed: unknown error\n");
        return false;
    }
}

bool Storage::Save(const std::wstring& path) {
    m_path = path;
    return SaveToFile();
}

bool Storage::SaveToFile() {
    json j;
    j["version"] = "1.0.0";
    
    for (const auto& cat : m_categories) {
        json c;
        c["id"] = StringUtils::WStringToUtf8(cat.id);
        c["name"] = StringUtils::WStringToUtf8(cat.name);
        c["sortOrder"] = cat.sortOrder;
        c["viewType"] = static_cast<int>(cat.viewType);
        c["iconSize"] = cat.iconSize;
        j["categories"].push_back(c);
    }
    
    for (const auto& item : m_items) {
        json i;
        i["id"] = StringUtils::WStringToUtf8(item.id);
        i["name"] = StringUtils::WStringToUtf8(item.name);
        i["target"] = StringUtils::WStringToUtf8(item.target);
        i["arguments"] = StringUtils::WStringToUtf8(item.arguments);
        i["workingDir"] = StringUtils::WStringToUtf8(item.workingDir);
        i["iconPath"] = StringUtils::WStringToUtf8(item.iconPath);
        i["iconIndex"] = item.iconIndex;
        i["runAsAdmin"] = item.runAsAdmin;
        i["runCount"] = item.runCount;
        i["keywords"] = StringUtils::WStringToUtf8(item.keywords);
        i["remark"] = StringUtils::WStringToUtf8(item.remark);
        i["categoryId"] = StringUtils::WStringToUtf8(item.categoryId);
        i["sortOrder"] = item.sortOrder;
        j["items"].push_back(i);
    }
    
    for (const auto& [key, val] : m_config) {
        j["config"][key] = StringUtils::WStringToUtf8(val);
    }
    
    std::ofstream file(m_path);
    if (!file.is_open()) return false;
    
    file << j.dump(4);
    return true;
}

std::vector<Category> Storage::GetCategories() const {
    return m_categories;
}

Category Storage::GetCategory(const std::wstring& id) const {
    for (const auto& cat : m_categories) {
        if (cat.id == id) return cat;
    }
    return {};
}

bool Storage::AddCategory(const Category& category) {
    if (category.id.empty()) return false;
    m_categories.push_back(category);
    return SaveToFile();
}

bool Storage::UpdateCategory(const Category& category) {
    for (auto& cat : m_categories) {
        if (cat.id == category.id) {
            cat = category;
            return SaveToFile();
        }
    }
    return false;
}

bool Storage::DeleteCategory(const std::wstring& id) {
    auto itemIds = GetItemIdsByCategory(id);
    for (const auto& itemId : itemIds) {
        DeleteItem(itemId);
    }
    
    m_categories.erase(
        std::remove_if(m_categories.begin(), m_categories.end(),
            [&](const Category& c) { return c.id == id; }),
        m_categories.end()
    );
    
    return SaveToFile();
}

std::vector<Item> Storage::GetItems(const std::wstring& categoryId) const {
    if (categoryId.empty()) return m_items;
    
    std::vector<Item> result;
    for (const auto& item : m_items) {
        if (item.categoryId == categoryId) {
            result.push_back(item);
        }
    }
    return result;
}

Item Storage::GetItem(const std::wstring& id) const {
    for (const auto& item : m_items) {
        if (item.id == id) return item;
    }
    return {};
}

bool Storage::AddItem(const Item& item) {
    if (item.id.empty()) return false;
    m_items.push_back(item);
    return SaveToFile();
}

bool Storage::UpdateItem(const Item& item) {
    for (auto& i : m_items) {
        if (i.id == item.id) {
            i = item;
            return SaveToFile();
        }
    }
    return false;
}

bool Storage::DeleteItem(const std::wstring& id) {
    m_items.erase(
        std::remove_if(m_items.begin(), m_items.end(),
            [&](const Item& i) { return i.id == id; }),
        m_items.end()
    );
    return SaveToFile();
}

std::vector<std::wstring> Storage::GetItemIdsByCategory(const std::wstring& categoryId) const {
    std::vector<std::wstring> ids;
    for (const auto& item : m_items) {
        if (item.categoryId == categoryId) {
            ids.push_back(item.id);
        }
    }
    return ids;
}

std::wstring Storage::GetConfig(const std::string& key, const std::wstring& defaultVal) const {
    auto it = m_config.find(key);
    if (it != m_config.end()) return it->second;
    return defaultVal;
}

bool Storage::SetConfig(const std::string& key, const std::wstring& value) {
    m_config[key] = value;
    return SaveToFile();
}

} // namespace mn