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
    
    // 如果缓存存在，直接返回
    if (PathUtils::Exists(cachePath)) {
        return cachePath;
    }
    
    // 尝试提取图标
    RefreshIcon(item);
    
    // 检查是否成功创建
    if (PathUtils::Exists(cachePath)) {
        return cachePath;
    }
    
    // 提取失败，返回空
    return L"";
}

void IconCache::RefreshIcon(const Item& item) {
#ifdef _WIN32
    std::wstring cachePath = m_cacheDir + L"\\" + item.id + L".png";
    
    HICON hIcon = nullptr;
    
    // 1. 首先尝试从自定义图标路径提取（如果有）
    if (!item.iconPath.empty() && PathUtils::Exists(item.iconPath)) {
        ExtractIconExW(item.iconPath.c_str(), item.iconIndex, &hIcon, nullptr, 1);
    }
    
    // 2. 如果失败，尝试从原始目标路径提取（快捷方式解析前的路径）
    if (!hIcon && !item.target.empty() && PathUtils::Exists(item.target)) {
        SHFILEINFOW sfi = {};
        DWORD_PTR result = SHGetFileInfoW(
            item.target.c_str(),
            0,
            &sfi,
            sizeof(sfi),
            SHGFI_ICON | SHGFI_LARGEICON
        );
        if (result && sfi.hIcon) {
            hIcon = sfi.hIcon;
        }
    }
    
    // 3. 如果仍然没有图标，使用默认图标
    if (!hIcon) {
        SHFILEINFOW sfi = {};
        DWORD_PTR result = SHGetFileInfoW(
            L".txt",
            FILE_ATTRIBUTE_NORMAL,
            &sfi,
            sizeof(sfi),
            SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES
        );
        if (result && sfi.hIcon) {
            hIcon = sfi.hIcon;
        }
    }
    
    // 4. 保存图标
    if (hIcon) {
        Gdiplus::Bitmap* bitmap = Gdiplus::Bitmap::FromHICON(hIcon);
        if (bitmap) {
            CLSID clsid;
            if (GetEncoderClsid(L"image/png", &clsid) != -1) {
                Gdiplus::Status status = bitmap->Save(cachePath.c_str(), &clsid, nullptr);
                if (status != Gdiplus::Ok) {
                    // 保存失败
                    OutputDebugStringW(L"IconCache: Failed to save icon\n");
                }
            }
            delete bitmap;
        }
        DestroyIcon(hIcon);
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
