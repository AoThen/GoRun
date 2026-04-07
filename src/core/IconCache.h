#pragma once

#include "Types.h"
#include <string>
#include <unordered_map>
#include <cstdint>

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
    uintptr_t m_gdiplusToken = 0;  // GDI+ token (ULONG_PTR on Windows)
};

} // namespace mn