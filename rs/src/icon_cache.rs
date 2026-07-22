#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::model::Item;

pub struct IconCache {
    cache_dir: PathBuf,
}

impl IconCache {
    pub fn new() -> Self {
        let dir = config::icons_dir();
        let _ = fs::create_dir_all(&dir);
        log::info!("IconCache initialized: dir={:?}", dir);
        IconCache { cache_dir: dir }
    }

    pub fn cache_path(&self, item_id: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        item_id.hash(&mut hasher);
        let hash = hasher.finish();
        self.cache_dir.join(format!("{:x}.png", hash))
    }

    pub fn get_icon_path(&self, item_id: &str) -> Option<PathBuf> {
        let path = self.cache_path(item_id);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn delete_cache(&self, item_id: &str) {
        let path = self.cache_path(item_id);
        if path.exists() {
            log::debug!("Icon cache deleted: id={}", item_id);
            let _ = fs::remove_file(&path);
        }
    }

    pub fn refresh_icon(&self, item: &Item) -> Option<PathBuf> {
        let cache_path = self.cache_path(&item.id);
        log::debug!("Refreshing icon for item: id={}", item.id);

        let hicon = if !item.icon_path.is_empty() {
            log::debug!("Trying custom icon_path: {}", item.icon_path);
            extract_hicon(&item.icon_path, item.icon_index)
        } else {
            None
        };

        let hicon = hicon.or_else(|| {
            if !item.target.is_empty() {
                log::debug!("Falling back to target file: {}", item.target);
                extract_hicon_from_target(&item.target)
            } else {
                None
            }
        });

        let hicon = hicon.or_else(|| {
            log::debug!("Using default system icon");
            get_default_icon()
        })?;

        let success = save_hicon_as_png(hicon, &cache_path);

        #[cfg(windows)]
        unsafe {
            use windows::Win32::Graphics::Gdi::DestroyIcon;
            let _ = DestroyIcon(hicon);
        }

        if success {
            log::info!("Icon saved: {:?}", cache_path);
            Some(cache_path)
        } else {
            log::warn!("Failed to save icon for item: id={}", item.id);
            None
        }
    }
}

#[cfg(windows)]
fn extract_hicon(icon_path: &str, icon_index: i32) -> Option<windows::Win32::Graphics::Gdi::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Graphics::Gdi::HICON;
    use windows::Win32::UI::Shell::ExtractIconExW;

    let wide: Vec<u16> = std::ffi::OsStr::new(icon_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut large_icon = HICON::default();
    let mut small_icon = HICON::default();

    let count = unsafe {
        ExtractIconExW(
            windows::core::PCWSTR(wide.as_ptr()),
            icon_index,
            Some(&mut large_icon),
            Some(&mut small_icon),
            1,
        )
    };

    if count == 0 {
        log::debug!("ExtractIconExW returned 0 for: {}", icon_path);
        return None;
    }

    if !large_icon.is_invalid() {
        if !small_icon.is_invalid() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DestroyIcon(small_icon);
            }
        }
        Some(large_icon)
    } else if !small_icon.is_invalid() {
        Some(small_icon)
    } else {
        None
    }
}

#[cfg(windows)]
fn extract_hicon_from_target(target: &str) -> Option<windows::Win32::Graphics::Gdi::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Graphics::Gdi::HICON;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON};

    let wide: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut file_info = windows::Win32::UI::Shell::SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON;

    let result = unsafe {
        SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            0,
            Some(&mut file_info),
            std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            flags,
        )
    };

    if result == 0 {
        log::debug!("SHGetFileInfoW failed for: {}", target);
        return None;
    }

    if file_info.hIcon.is_invalid() {
        None
    } else {
        Some(file_info.hIcon)
    }
}

#[cfg(windows)]
fn get_default_icon() -> Option<windows::Win32::Graphics::Gdi::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Graphics::Gdi::HICON;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
    };

    let wide: Vec<u16> = std::ffi::OsStr::new(".txt")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut file_info = windows::Win32::UI::Shell::SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES;

    let result = unsafe {
        SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            0x80,
            Some(&mut file_info),
            std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            flags,
        )
    };

    if result == 0 {
        log::debug!("SHGetFileInfoW failed for default icon");
        return None;
    }

    if file_info.hIcon.is_invalid() {
        None
    } else {
        Some(file_info.hIcon)
    }
}

#[cfg(windows)]
fn save_hicon_as_png(hicon: windows::Win32::Graphics::Gdi::HICON, path: &Path) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetIconInfo, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, ICONINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::DrawIconEx;

    unsafe {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            log::warn!("GetIconInfo failed");
            return false;
        }

        let mut width = 0i32;
        let mut height = 0i32;

        if !icon_info.hbmColor.is_invalid() {
            let mut bmp = windows::Win32::Graphics::Gdi::BITMAP::default();
            let size = windows::Win32::Graphics::Gdi::GetObjectW(
                icon_info.hbmColor,
                std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32,
                Some(&mut bmp),
            );
            if size > 0 {
                width = bmp.bmWidth;
                height = bmp.bmHeight;
            }
            let _ = DeleteObject(icon_info.hbmColor);
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask);
        }

        if width == 0 || height == 0 {
            width = 48;
            height = 48;
        }

        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            log::warn!("CreateCompatibleDC failed");
            return false;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut pixel_data_ptr = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut pixel_data_ptr, None, 0);

        if hbitmap.is_invalid() || pixel_data_ptr.is_null() {
            log::warn!("CreateDIBSection failed");
            let _ = DeleteDC(hdc);
            return false;
        }

        let old_bitmap = SelectObject(hdc, hbitmap);

        let result = DrawIconEx(
            hdc,
            0,
            0,
            hicon,
            width as u32,
            height as u32,
            0,
            None,
            0x0003,
        );

        if result.is_err() {
            log::warn!("DrawIconEx failed");
            SelectObject(hdc, old_bitmap);
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc);
            return false;
        }

        let pixel_count = (width * height) as usize;
        let pixel_slice = std::slice::from_raw_parts(pixel_data_ptr as *const u8, pixel_count * 4);

        let mut rgba_pixels = Vec::with_capacity(pixel_count * 4);
        for chunk in pixel_slice.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            rgba_pixels.push(r);
            rgba_pixels.push(g);
            rgba_pixels.push(b);
            rgba_pixels.push(a);
        }

        SelectObject(hdc, old_bitmap);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc);

        match image::save_buffer(
            path,
            &rgba_pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        ) {
            Ok(_) => true,
            Err(e) => {
                log::warn!("Failed to save PNG: {}", e);
                false
            }
        }
    }
}

#[cfg(not(windows))]
fn extract_hicon(_icon_path: &str, _icon_index: i32) -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn extract_hicon_from_target(_target: &str) -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn get_default_icon() -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn save_hicon_as_png(_hicon: u32, _path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_no_traversal() {
        let cache = IconCache::new();
        let path1 = cache.cache_path("normal_id");
        let path2 = cache.cache_path("../../etc/passwd");
        assert!(path1.starts_with(&cache.cache_dir));
        assert!(path2.starts_with(&cache.cache_dir));
    }

    #[test]
    fn test_cache_path_deterministic() {
        let cache = IconCache::new();
        let path1 = cache.cache_path("test_id");
        let path2 = cache.cache_path("test_id");
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_cache_path_different_ids() {
        let cache = IconCache::new();
        let path1 = cache.cache_path("id_a");
        let path2 = cache.cache_path("id_b");
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_get_icon_path_nonexistent() {
        let cache = IconCache::new();
        let result = cache.get_icon_path("nonexistent_id");
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_cache_nonexistent() {
        let cache = IconCache::new();
        cache.delete_cache("nonexistent_id");
    }
}
