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
            use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
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
fn extract_hicon(
    icon_path: &str,
    icon_index: i32,
) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ExtractIconExW;
    use windows::Win32::UI::WindowsAndMessaging::HICON;

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
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(small_icon);
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
fn extract_hicon_from_target(
    target: &str,
) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    let wide: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut file_info = windows::Win32::UI::Shell::SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON;

    let result = unsafe {
        SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
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
fn get_default_icon() -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
    };
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    let wide: Vec<u16> = std::ffi::OsStr::new(".txt")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut file_info = windows::Win32::UI::Shell::SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES;

    let result = unsafe {
        SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0x80),
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
fn save_hicon_as_png(hicon: windows::Win32::UI::WindowsAndMessaging::HICON, path: &Path) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetObjectW, ReleaseDC, SelectObject,
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, GetDIBits,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    unsafe {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info as *mut _).is_err() {
            log::warn!("GetIconInfo failed");
            return false;
        }

        let mut bmp = BITMAP::default();
        let bmp_size = std::mem::size_of::<BITMAP>() as i32;
        if GetObjectW(icon_info.hbmColor, bmp_size, &mut bmp as *mut BITMAP) == 0 {
            log::warn!("GetObjectW failed for icon bitmap");
            let _ = DeleteObject(icon_info.hbmColor);
            let _ = DeleteObject(icon_info.hbmMask);
            return false;
        }

        let width = bmp.bmWidth;
        let height = bmp.bmHeight;

        if width <= 0 || height <= 0 || width > 256 || height > 256 {
            log::warn!("Invalid icon dimensions: {}x{}", width, height);
            let _ = DeleteObject(icon_info.hbmColor);
            let _ = DeleteObject(icon_info.hbmMask);
            return false;
        }

        let hdc_screen = GetDC(HWND(0));
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm_old = SelectObject(hdc_mem, icon_info.hbmColor);

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let buf_size = (width * height * 4) as usize;
        let mut pixels = vec![0u8; buf_size];

        let lines = GetDIBits(
            hdc_mem,
            icon_info.hbmColor,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, hbm_old);
        DeleteDC(hdc_mem);
        ReleaseDC(HWND(0), hdc_screen);
        let _ = DeleteObject(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmMask);

        if lines == 0 {
            log::warn!("GetDIBits failed");
            return false;
        }

        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        match image::save_buffer(
            path,
            &pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        ) {
            Ok(_) => {
                log::debug!("Icon saved to {:?} ({}x{})", path, width, height);
                true
            }
            Err(e) => {
                log::warn!("Failed to save icon PNG: {}", e);
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
