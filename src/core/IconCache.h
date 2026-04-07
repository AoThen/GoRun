#pragma once

#include "Types.h"
#include <string>
#include <unordered_map>

namespace mn {

class IconCache {
public:
    ~IconCache();
    
    void Initialize(const std::wstring& cacheDir);
    
    std::wstring GetIconPath(const Item& item);
    void RefreshIcon(const Item& item);
    void DeleteCache(const std::wstring& itemId);

private:
    std::wstring m_cacheDir;
    std::unordered_map<std::wstring, std::wstring> m_cache;
    void* m_gdiplusToken = nullptr;  // GDI+ token
};

} // namespace mn
