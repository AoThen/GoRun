#include "Storage.h"
#include "utils/StringUtils.h"
#include "utils/PathUtils.h"
#include "utils/Logger.h"
#include <nlohmann/json.hpp>
#include <fstream>
#include <algorithm>
#include <windows.h>

namespace mn {

using json = nlohmann::json;

void Storage::Initialize(const std::wstring& path) {
    m_path = path;
    if (!PathUtils::Exists(path)) {
        SaveToFile();
    }
}

void Storage::BeginBatch() {
    m_batchLevel++;
}

void Storage::EndBatch() {
    if (m_batchLevel > 0) {
        m_batchLevel--;
    }
    if (m_batchLevel == 0 && m_dirty) {
        SaveToFile();
    }
}

bool Storage::Flush() {
    if (!m_dirty) return true;
    return SaveToFile();
}

bool Storage::RotateBackups() {
    std::wstring bak1 = m_path + L".bak1";
    std::wstring bak2 = m_path + L".bak2";
    std::wstring bak3 = m_path + L".bak3";
    
    // bak2 -> bak3
    if (PathUtils::Exists(bak2)) {
        if (!MoveFileExW(bak2.c_str(), bak3.c_str(), MOVEFILE_REPLACE_EXISTING)) {
            LOG_ERROR("Storage::RotateBackups: failed to rotate bak2 -> bak3");
        }
    }
    // bak1 -> bak2
    if (PathUtils::Exists(bak1)) {
        if (!MoveFileExW(bak1.c_str(), bak2.c_str(), MOVEFILE_REPLACE_EXISTING)) {
            LOG_ERROR("Storage::RotateBackups: failed to rotate bak1 -> bak2");
        }
    }
    // current -> bak1
    if (PathUtils::Exists(m_path)) {
        if (!CopyFileW(m_path.c_str(), bak1.c_str(), FALSE)) {
            LOG_ERROR("Storage::RotateBackups: failed to copy current -> bak1");
            return false;
        }
    }
    return true;
}

bool Storage::SaveToFile() {
    if (!m_dirty) return true;
    m_dirty = false;
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
    
    RotateBackups();
    
    // 写入临时文件，然后原子重命名
    std::wstring tmpPath = m_path + L".tmp";
    {
        std::ofstream file(tmpPath);
        if (!file.is_open()) {
            LOG_ERROR("Storage::SaveToFile: failed to open temp file");
            DeleteFileW(tmpPath.c_str());
            return false;
        }
        file << j.dump(4);
    }
    
    if (!MoveFileExW(tmpPath.c_str(), m_path.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        LOG_ERROR("Storage::SaveToFile: failed to rename temp file");
        DeleteFileW(tmpPath.c_str());
        return false;
    }
    
    return true;
}

bool Storage::Load(const std::wstring& path) {
    m_path = path;
    
    m_categories.clear();
    m_items.clear();
    m_config.clear();
    
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
                int vt = cat.value("viewType", 0);
                c.viewType = (vt == 0 || vt == 1) ? static_cast<ViewType>(vt) : ViewType::Icon;
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
                i.iconIndex = (std::max)(0, item.value("iconIndex", 0));
                i.runAsAdmin = item.value("runAsAdmin", false);
                i.runCount = (std::max)(0, item.value("runCount", 0));
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
        LOG_ERROR(std::string("Storage::Load failed: ") + e.what());
        return false;
    } catch (...) {
        LOG_ERROR("Storage::Load failed: unknown error");
        return false;
    }
}

bool Storage::Save(const std::wstring& path) {
    m_path = path;
    return SaveToFile();
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
    m_dirty = true;
    if (m_batchLevel == 0) return SaveToFile();
    return true;
}

bool Storage::UpdateCategory(const Category& category) {
    for (auto& cat : m_categories) {
        if (cat.id == category.id) {
            cat = category;
            m_dirty = true;
            if (m_batchLevel == 0) return SaveToFile();
            return true;
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
    
    m_dirty = true;
    if (m_batchLevel == 0) return SaveToFile();
    return true;
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
    m_dirty = true;
    if (m_batchLevel == 0) return SaveToFile();
    return true;
}

bool Storage::UpdateItem(const Item& item) {
    for (auto& i : m_items) {
        if (i.id == item.id) {
            i = item;
            m_dirty = true;
            if (m_batchLevel == 0) return SaveToFile();
            return true;
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
    m_dirty = true;
    if (m_batchLevel == 0) return SaveToFile();
    return true;
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
    m_dirty = true;
    if (m_batchLevel == 0) return SaveToFile();
    return true;
}

} // namespace mn