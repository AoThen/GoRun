#include "IconCache.h"
#include "utils/PathUtils.h"

#ifdef _WIN32
#include <ShellApi.h>
#endif

namespace mn {

void IconCache::Initialize(const std::wstring& cacheDir) {
    m_cacheDir = cacheDir;
    PathUtils::EnsureDirectory(cacheDir);
}

std::wstring IconCache::GetIconPath(const Item& item) {
    std::wstring cachePath = m_cacheDir + L"\\" + item.id + L".png";
    if (PathUtils::Exists(cachePath)) {
        return cachePath;
    }
    
    RefreshIcon(item);
    return cachePath;
}

void IconCache::RefreshIcon(const Item& item) {
#ifdef _WIN32
    std::wstring cachePath = m_cacheDir + L"\\" + item.id + L".png";
    
    SHFILEINFOW sfi = {};
    DWORD_PTR result = SHGetFileInfoW(
        item.target.c_str(),
        0,
        &sfi,
        sizeof(sfi),
        SHGFI_ICON | SHGFI_LARGEICON
    );
    
    if (result && sfi.hIcon) {
        // 简化：只保存图标路径引用
        DestroyIcon(sfi.hIcon);
    }
#endif
}

void IconCache::DeleteCache(const std::wstring& itemId) {
    std::wstring cachePath = m_cacheDir + L"\\" + itemId + L".png";
    if (PathUtils::Exists(cachePath)) {
#ifdef _WIN32
        DeleteFileW(cachePath.c_str());
#endif
    }
}

} // namespace mn
