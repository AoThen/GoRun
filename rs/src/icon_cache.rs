use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config;
use crate::model::Item;

#[derive(Clone)]
pub struct IconCache {
    cache_dir: PathBuf,
    /// item_id -> 最近一次缓存文件名；编辑图标/目标/索引后按映射表清理旧文件
    item_keys: Arc<Mutex<HashMap<String, String>>>,
}

impl IconCache {
    pub fn new() -> Self {
        let dir = config::icons_dir();
        let _ = fs::create_dir_all(&dir);
        log::info!("IconCache initialized: dir={:?}", dir);
        IconCache {
            cache_dir: dir,
            item_keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cache_key(id: &str, icon_path: &str, icon_index: i32, target: &str) -> u64 {
        // FNV-1a 64-bit：跨 Rust 版本稳定（DefaultHasher 不保证），且纳入图标/目标信息，
        // 编辑图标或目标后自动失效缓存
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |data: &[u8]| {
            for &b in data {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x1_0000_0001_b3);
            }
        };
        feed(id.as_bytes());
        feed(&[0x1f]);
        feed(icon_path.as_bytes());
        feed(&[0x1f]);
        feed(&icon_index.to_le_bytes());
        feed(&[0x1f]);
        feed(target.as_bytes());
        hash
    }

    pub fn cache_path(&self, item: &Item) -> PathBuf {
        let hash = Self::cache_key(&item.id, &item.icon_path, item.icon_index, &item.target);
        self.cache_dir.join(format!("{:x}.png", hash))
    }

    /// 由已落盘的 `item.cached_icon` 恢复图标（进程重启后 item_keys 为空，
    /// 无法靠内存映射清理旧文件，故只校验文件存在）。
    pub fn restored_icon_path(&self, item: &Item) -> Option<PathBuf> {
        if item.cached_icon.is_empty() {
            return None;
        }
        let path = PathBuf::from(&item.cached_icon);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_icon_path(&self, item: &Item) -> Option<PathBuf> {
        let path = self.cache_path(item);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn delete_cache(&self, item: &Item) {
        // 编辑图标/目标/索引后缓存键变化，旧键文件按映射表清理（无界增长修复）
        if let Some(old_name) = self
            .item_keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&item.id)
        {
            let old_path = self.cache_dir.join(&old_name);
            if old_path.exists() {
                log::debug!(
                    "Icon cache deleted (old key): id={}, file={}",
                    item.id,
                    old_name
                );
                let _ = fs::remove_file(&old_path);
            }
        }
        let path = self.cache_path(item);
        if path.exists() {
            log::debug!("Icon cache deleted: id={}", item.id);
            let _ = fs::remove_file(&path);
        }
    }

    pub fn refresh_icon(&self, item: &Item) -> Option<PathBuf> {
        let cache_path = self.cache_path(item);
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
            let file_name = cache_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.item_keys
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(item.id.clone(), file_name);
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
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetObjectW, ReleaseDC,
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
        if GetObjectW(icon_info.hbmColor, bmp_size, Some(&mut bmp as *mut BITMAP as *mut std::ffi::c_void)) == 0 {
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

        // 32bpp 图标色板 alpha 常为全 0（透明信息存于 AND mask），需从 mask 重建 alpha，
        // 否则保存后图标完全透明不可见。仅检查 GetDIBits 实际返回的行。
        let mut mask_bits: Vec<u8> = Vec::new();
        let mut mask_row: usize = 0;
        let mask_needed = if lines != 0 && bmp.bmBitsPixel >= 32 {
            let valid_len = (lines as usize)
                .saturating_mul(width as usize)
                .saturating_mul(4)
                .min(pixels.len());
            pixels[..valid_len].chunks_exact(4).all(|c| c[3] == 0)
        } else {
            false
        };
        if mask_needed {
            mask_row = ((width as usize + 31) / 32) * 4;
            mask_bits.resize(mask_row * height as usize, 0);
            let mut mask_bmi = BITMAPINFO::default();
            mask_bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            mask_bmi.bmiHeader.biWidth = width;
            mask_bmi.bmiHeader.biHeight = -height;
            mask_bmi.bmiHeader.biPlanes = 1;
            mask_bmi.bmiHeader.biBitCount = 1;
            mask_bmi.bmiHeader.biCompression = BI_RGB.0;
            let mask_lines = GetDIBits(
                hdc_mem,
                icon_info.hbmMask,
                0,
                height as u32,
                Some(mask_bits.as_mut_ptr() as *mut _),
                &mut mask_bmi,
                DIB_RGB_COLORS,
            );
            if mask_lines == 0 {
                mask_bits.clear();
            }
        }

        DeleteDC(hdc_mem);
        ReleaseDC(HWND(0), hdc_screen);
        let _ = DeleteObject(icon_info.hbmColor);
        let _ = DeleteObject(icon_info.hbmMask);

        if lines == 0 {
            log::warn!("GetDIBits failed");
            return false;
        }

        let low_bpp = bmp.bmBitsPixel < 32;
        let use_mask = !mask_bits.is_empty();
        let w = width as usize;
        for (i, chunk) in pixels.chunks_exact_mut(4).enumerate() {
            chunk.swap(0, 2);
            if low_bpp {
                chunk[3] = 255;
            } else if use_mask {
                // mask 位 1 = 透明，0 = 不透明（1bpp 行内 MSB-first，行 4 字节对齐）
                let row = i / w;
                let col = i % w;
                let byte = mask_bits[row * mask_row + col / 8];
                chunk[3] = if (byte >> (7 - col % 8)) & 1 == 0 { 255 } else { 0 };
            }
        }

        let tmp_path = path.with_extension("png.tmp");
        if let Err(e) = image::save_buffer(
            &tmp_path,
            &pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        ) {
            log::warn!("Failed to save icon PNG: {}", e);
            let _ = fs::remove_file(&tmp_path);
            return false;
        }
        if fs::rename(&tmp_path, path).is_err() {
            log::warn!("Failed to rename temp icon to {:?}", path);
            let _ = fs::remove_file(&tmp_path);
            return false;
        }

        log::debug!("Icon saved to {:?} ({}x{})", path, width, height);
        true
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
    use crate::model::Item;

    fn test_item(id: &str) -> Item {
        Item {
            id: id.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_path_no_traversal() {
        let cache = IconCache::new();
        let path1 = cache.cache_path(&test_item("normal_id"));
        let path2 = cache.cache_path(&test_item("../../etc/passwd"));
        assert!(path1.starts_with(&cache.cache_dir));
        assert!(path2.starts_with(&cache.cache_dir));
    }

    #[test]
    fn test_cache_path_deterministic() {
        let cache = IconCache::new();
        let item = test_item("test_id");
        let path1 = cache.cache_path(&item);
        let path2 = cache.cache_path(&item);
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_cache_path_different_ids() {
        let cache = IconCache::new();
        let path1 = cache.cache_path(&test_item("id_a"));
        let path2 = cache.cache_path(&test_item("id_b"));
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_cache_path_edits_invalidate() {
        // 编辑图标路径/目标/索引后缓存键变化（自动失效）
        let cache = IconCache::new();
        let base = Item {
            id: "id_x".into(),
            icon_path: "a.ico".into(),
            icon_index: 0,
            target: "app.exe".into(),
            ..Default::default()
        };
        let p1 = cache.cache_path(&base);
        let mut edited = base.clone();
        edited.icon_path = "b.ico".into();
        let p2 = cache.cache_path(&edited);
        edited.icon_index = 1;
        let p3 = cache.cache_path(&edited);
        edited.target = "other.exe".into();
        let p4 = cache.cache_path(&edited);
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
        assert_ne!(p3, p4);
    }

    #[test]
    fn test_cache_key_stable_across_platforms() {
        // FNV-1a 结果确定性（无随机种子），断言固定值防回归
        let h = IconCache::cache_key("abc", "", 0, "");
        assert_eq!(h, 0x7cb7_e682_6a28_5762);
    }

    #[test]
    fn test_get_icon_path_nonexistent() {
        let cache = IconCache::new();
        let result = cache.get_icon_path(&test_item("nonexistent_id"));
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_cache_nonexistent() {
        let cache = IconCache::new();
        cache.delete_cache(&test_item("nonexistent_id"));
    }
}
