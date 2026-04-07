#include "IconCache.h"
#include "utils/PathUtils.h"

#ifdef _WIN32
#include <Windows.h>
#include <ShellApi.h>
#include <comdef.h>
#include <gdiplus.h>
#include <shlwapi.h>
#pragma comment(lib, "gdiplus.lib")
#endif

namespace mn {

#ifdef _WIN32
static int GetEncoderClsid(const WCHAR* format, CLSID* pClsid) {
    UINT num = 0, size = 0;
    Gdiplus::GetImageEncodersSize(&num, &size);
    if (size == 0) return -1;
    
    Gdiplus::ImageCodecInfo* pImageCodecInfo = (Gdiplus::ImageCodecInfo*)malloc(size);
    if (!pImageCodecInfo) return -1;
    
    Gdiplus::GetImageEncoders(num, size, pImageCodecInfo);
    for (UINT j = 0; j < num; ++j) {
        if (wcscmp(pImageCodecInfo[j].MimeType, format) == 0) {
            *pClsid = pImageCodecInfo[j].Clsid;
            free(pImageCodecInfo);
            return j;
        }
    }
    free(pImageCodecInfo);
    return -1;
}
#endif

void IconCache::Initialize(const std::wstring& cacheDir) {
    m_cacheDir = cacheDir;
    PathUtils::EnsureDirectory(cacheDir);
    
#ifdef _WIN32
    // 初始化 GDI+
    Gdiplus::GdiplusStartupInput gdiplusStartupInput;
    Gdiplus::GdiplusStartup(&m_gdiplusToken, &gdiplusStartupInput, nullptr);
#endif
}

IconCache::~IconCache() {
#ifdef _WIN32
    if (m_gdiplusToken) {
        Gdiplus::GdiplusShutdown(m_gdiplusToken);
    }
#endif
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
        // 使用 GDI+ 保存图标为 PNG
        Gdiplus::Bitmap* bitmap = Gdiplus::Bitmap::FromHICON(sfi.hIcon);
        if (bitmap) {
            CLSID clsid;
            if (GetEncoderClsid(L"image/png", &clsid) != -1) {
                bitmap->Save(cachePath.c_str(), &clsid, nullptr);
            }
            delete bitmap;
        }
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
