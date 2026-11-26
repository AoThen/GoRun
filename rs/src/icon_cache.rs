use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config;
use crate::model::Item;

/// 图标 tmp 文件命名计数器：与进程 id 组合成唯一后缀（对齐 storage 的
/// TMP_SEQ 模式）。固定 `png.tmp` 后缀时，搜索防抖期间多次 spawn 的提取
/// 线程可对同一缓存文件并发写同一 tmp → PNG 损坏/ rename 竞争；唯一名
/// 消除竞态（各线程独立 tmp，rename 原子替换）。仅 Windows 提取路径使用。
#[cfg(windows)]
static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 图标像素内存缓存上限（P3-1）。超出时整体清空再插入，简单策略对齐
/// item_manager 的 PINYIN_CACHE_MAX：缓存可重建（未命中即读盘重解码），
/// 清空只是回退到读盘路径，无数据丢失。48×48×4≈9KB/项，512 项≈4.6MB 封顶。
const IMAGE_CACHE_MAX: usize = 512;

/// 当前会话的图标显示尺寸（设置项 icon_size，全局统一）。缓存键纳入该值，
/// 改尺寸后旧键缓存自动失效。进程内共享（提取在后台线程进行，须跨线程可见），
/// 启动时按配置初始化、改设置时更新。
static CURRENT_ICON_SIZE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(48);

/// 读取当前图标显示尺寸（全局设置）。
pub fn current_icon_size() -> i32 {
    CURRENT_ICON_SIZE.load(std::sync::atomic::Ordering::Relaxed)
}

/// 更新当前图标显示尺寸（改设置时调用；清空内存缓存由调用方负责）。
pub fn set_current_icon_size(size: i32) {
    CURRENT_ICON_SIZE.store(size, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Clone)]
pub struct IconCache {
    cache_dir: PathBuf,
    /// item_id -> 最近一次缓存文件名；编辑图标/目标/索引后按映射表清理旧文件
    item_keys: Arc<Mutex<HashMap<String, String>>>,
    /// 解码后的图标像素内存缓存（key = cache_key 的 FNV 值）。
    /// 存 SharedPixelBuffer（Send+Sync 的引用计数像素块）而非 slint::Image：
    /// Image 持有 Slint 线程本地缓存、不是 Send，放进本结构会破坏
    /// spawn_icon_refresh 的线程边界。UI 线程用时再以 Image::from_rgba8 轻量包装。
    /// 目的：搜索输入等高频刷新路径避免对同一 PNG 反复读盘解码。
    /// H3：值携带最近访问序号（LRU），超上限逐出最久未用单项——此前整体
    /// 清空使全部热缓存回退读盘重解码，影响面从整表降为一项。
    images: Arc<Mutex<HashMap<u64, (slint::SharedPixelBuffer<slint::Rgba8Pixel>, u64)>>>,
    /// LRU 访问序号（单调递增，Atomic 满足跨提取线程的存取）。
    seq: Arc<std::sync::atomic::AtomicU64>,
}

// 锁恢复说明：images/item_keys 均为可丢弃可重建的缓存（解码失败/缺失即重新读盘），
// 锁内只做 HashMap 的 get/insert/remove 简单操作、不会 panic，中毒仅在极端情况
// （如 OOM）发生。锁中毒时用 into_inner() 恢复继续用：缓存可能半更新，但下次访问
// 会按需重算/重读盘覆盖，风险可忽略；若改为丢弃重建会丢失已有缓存、收益为负。

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

impl IconCache {
    pub fn new() -> Self {
        let dir = config::icons_dir();
        let _ = fs::create_dir_all(&dir);
        log::info!("IconCache initialized: dir={:?}", dir);
        IconCache {
            cache_dir: dir,
            item_keys: Arc::new(Mutex::new(HashMap::new())),
            images: Arc::new(Mutex::new(HashMap::new())),
            seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// 从内存缓存取已解码像素；未命中返回 None（调用方再读盘并回填）。
    /// 返回 SharedPixelBuffer 的轻量克隆，调用方包装成 slint::Image。
    /// H3：命中时刷新该项的 LRU 访问序号。
    pub fn cached_image(&self, item: &Item) -> Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>> {
        let hash = Self::cache_key(&item.id, &item.icon_path, item.icon_index, &item.target, current_icon_size());
        let mut map = self.images.lock().unwrap_or_else(|e| e.into_inner());
        let seq = self
            .seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match map.get_mut(&hash) {
            Some((buffer, entry_seq)) => {
                *entry_seq = seq;
                Some(buffer.clone())
            }
            None => None,
        }
    }

    /// 回填内存缓存（读盘解码成功后调用）。
    /// H3：超上限逐出最久未访问单项（LRU）后插入——替代此前的整体清空，
    /// 避免全部热缓存一次性回退读盘重解码。淘汰扫描仅在满时插入触发，
    /// O(IMAGE_CACHE_MAX)，锁内微秒级。
    pub fn store_image(&self, item: &Item, buffer: slint::SharedPixelBuffer<slint::Rgba8Pixel>) {
        let hash = Self::cache_key(&item.id, &item.icon_path, item.icon_index, &item.target, current_icon_size());
        let mut map = self.images.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= IMAGE_CACHE_MAX {
            if let Some((&evict_hash, _)) = map.iter().min_by_key(|(_, (_, seq))| *seq) {
                map.remove(&evict_hash);
            }
        }
        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        map.insert(hash, (buffer, seq));
    }

    /// 失效某项的内存缓存（编辑图标/目标后）。
    pub fn invalidate_image(&self, item: &Item) {
        let hash = Self::cache_key(&item.id, &item.icon_path, item.icon_index, &item.target, current_icon_size());
        self.images
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&hash);
    }

    /// 清空全部图标像素内存缓存（改图标尺寸后：缓存键随尺寸变化，旧键项
    /// 不会再被访问，成为内存垃圾；显式清空立即释放）。
    pub fn clear_images(&self) {
        self.images
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    fn cache_key(id: &str, icon_path: &str, icon_index: i32, target: &str, icon_size: i32) -> u64 {
        // FNV-1a 64-bit：跨 Rust 版本稳定（DefaultHasher 不保证），且纳入图标/目标/尺寸信息，
        // 编辑图标或目标或改尺寸后自动失效缓存（不同尺寸提取的 PNG 不同，必须分键）
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |data: &[u8]| {
            for &b in data {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        feed(id.as_bytes());
        feed(&[0x1f]);
        feed(icon_path.as_bytes());
        feed(&[0x1f]);
        feed(&icon_index.to_le_bytes());
        feed(&[0x1f]);
        feed(target.as_bytes());
        feed(&[0x1f]);
        feed(&icon_size.to_le_bytes());
        hash
    }

    pub fn cache_path(&self, item: &Item) -> PathBuf {
        let hash = Self::cache_key(&item.id, &item.icon_path, item.icon_index, &item.target, current_icon_size());
        self.cache_dir.join(format!("{:x}.png", hash))
    }

    /// 由已落盘的 `item.cached_icon` 恢复图标（进程重启后 item_keys 为空，
    /// 无法靠内存映射清理旧文件，故以缓存键校验 + 文件存在判定）。
    ///
    /// 键校验（P1/R1）：stored 路径必须与当前 item 重算的 cache_path 一致才采纳。
    /// 此前只查文件存在，手改配置（target/icon_index/icon_path）后 cachedIcon 仍
    /// 指向旧键提取的文件，会显示 stale 图标；单机信任模型下（无 HMAC 拦截
    /// 手改配置）此风险实体化，故收紧为键匹配。路径形式不一致（跨机迁移等）
    /// 时 missed → 走提取自愈（重新提取并回写），无数据损失。
    pub fn restored_icon_path(&self, item: &Item) -> Option<PathBuf> {
        if item.cached_icon.is_empty() {
            return None;
        }
        let path = PathBuf::from(&item.cached_icon);
        if path == self.cache_path(item) && path.is_file() {
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
        // 内存缓存同步失效（旧键已随 item_keys 移除；新键待下次读盘回填）
        self.invalidate_image(item);
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

        // 提取尺寸 = 当前显示尺寸 × 2（下限 48 保证小图标清晰，上限 256 对齐
        // save_hicon_as_png 的位图尺寸上限）。放大提取再缩小显示可显著提升
        // 清晰度；缓存键已纳入显示尺寸，改尺寸后自动重提取。
        let extract_size = (current_icon_size() * 2).clamp(48, 256);

        let hicon = if !item.icon_path.is_empty() {
            log::debug!("Trying custom icon_path: {}", item.icon_path);
            extract_hicon(&item.icon_path, item.icon_index, extract_size)
        } else {
            None
        };

        let hicon = hicon.or_else(|| {
            if !item.target.is_empty() {
                log::debug!("Falling back to target file: {}", item.target);
                extract_hicon_from_target(&item.target, extract_size)
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

/// 启动时便携目录卫生检查（M-3/L-2）：清理提取原子写残留的 `*.tmp.*` 临时
/// 文件与孤儿缓存 PNG（删除条目/编辑图标后残留、进程重启后 item_keys 为空
/// 无法按映射清理的旧键文件）。`live_files` 为当前所有条目 `cached_icon`
/// 指向的文件名集合；tmp 仅清理修改时间早于本进程启动的残留。返回清理数量。
pub fn cleanup_and_gc(cache_dir: &Path, live_files: &std::collections::HashSet<String>) -> usize {
    let started = std::time::SystemTime::now();
    let mut removed = 0;
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let name = match entry.file_name().to_str() {
                Some(n) => n.to_string(),
                None => continue,
            };
            let is_tmp = name.contains(".tmp.");
            let is_png = name.ends_with(".png");
            if !is_tmp && !is_png {
                continue;
            }
            let stale = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| t < started)
                .unwrap_or(false);
            let orphan = is_png && !live_files.contains(&name);
            if ((is_tmp && stale) || orphan) && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    if removed > 0 {
        log::info!(
            "Icon cache GC: removed {} stale/orphan file(s) in {:?}",
            removed,
            cache_dir
        );
    }
    removed
}

#[cfg(windows)]
fn extract_hicon(
    icon_path: &str,
    icon_index: i32,
    icon_size: i32,
) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::WindowsAndMessaging::{PrivateExtractIconsW, HICON};

    // 提取尺寸由调用方传入（显示尺寸的 2 倍，放大提取再缩小显示更清晰）。
    // PrivateExtractIconsW 支持指定精确尺寸，对 exe/dll/ico 均有效；flags=0
    // 表示按传入 cx/cy 提取。

    let wide: Vec<u16> = std::ffi::OsStr::new(icon_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // PrivateExtractIconsW 要求路径缓冲区定长 [u16; 260]（MAX_PATH）；
    // 超长路径直接失败，避免截断后无 NUL 终止导致 FFI 越界读。
    if wide.len() > 260 {
        log::debug!(
            "Icon path too long ({}), skipping: {}",
            wide.len(),
            icon_path
        );
        return None;
    }
    let mut path_buf = [0u16; 260];
    path_buf[..wide.len()].copy_from_slice(&wide);

    let icon = HICON::default();
    let count = unsafe {
        PrivateExtractIconsW(
            &path_buf,
            icon_index,
            icon_size,
            icon_size,
            Some(&mut [icon]),
            None,
            0,
        )
    };

    if count == 0 || icon.is_invalid() {
        log::debug!("PrivateExtractIconsW returned {} for: {}", count, icon_path);
        return None;
    }
    Some(icon)
}

#[cfg(windows)]
fn extract_hicon_from_target(
    target: &str,
    icon_size: i32,
) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON};

    let wide: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // 优先 PrivateExtractIconsW 按传入尺寸提取（与 UI 显示 2x 对应）；目标可能是
    // 任意文件（非 exe/dll/ico 时该 API 返回 0），回退 SHGetFileInfoW 大图标
    // （32px，放大可接受）。
    if wide.len() <= 260 {
        let mut path_buf = [0u16; 260];
        path_buf[..wide.len()].copy_from_slice(&wide);
        let icon = windows::Win32::UI::WindowsAndMessaging::HICON::default();
        let count = unsafe {
            windows::Win32::UI::WindowsAndMessaging::PrivateExtractIconsW(
                &path_buf,
                0,
                icon_size,
                icon_size,
                Some(&mut [icon]),
                None,
                0,
            )
        };
        if count > 0 && !icon.is_invalid() {
            return Some(icon);
        }
        log::debug!(
            "PrivateExtractIconsW fallback to SHGetFileInfoW for: {}",
            target
        );
    }

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
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC,
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
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
        if GetObjectW(
            icon_info.hbmColor.into(),
            bmp_size,
            Some(&mut bmp as *mut BITMAP as *mut std::ffi::c_void),
        ) == 0
        {
            log::warn!("GetObjectW failed for icon bitmap");
            let _ = DeleteObject(icon_info.hbmColor.into());
            let _ = DeleteObject(icon_info.hbmMask.into());
            return false;
        }

        let width = bmp.bmWidth;
        let height = bmp.bmHeight;

        if width <= 0 || height <= 0 || width > 256 || height > 256 {
            log::warn!("Invalid icon dimensions: {}x{}", width, height);
            let _ = DeleteObject(icon_info.hbmColor.into());
            let _ = DeleteObject(icon_info.hbmMask.into());
            return false;
        }

        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

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
        // 否则保存后图标完全透明不可见。alpha 带噪声（近全透明）时同样按无效处理，
        // 否则 mask 重建被跳过、保存出的 PNG 接近全透明（用户看到"空白方框"）。
        // 仅检查 GetDIBits 实际返回的行。
        let mut mask_bits: Vec<u8> = Vec::new();
        let mut mask_row: usize = 0;
        let mask_needed = if lines != 0 && bmp.bmBitsPixel >= 32 {
            let valid_len = (lines as usize)
                .saturating_mul(width as usize)
                .saturating_mul(4)
                .min(pixels.len());
            let total = valid_len / 4;
            let opaque = pixels[..valid_len]
                .chunks_exact(4)
                .filter(|c| c[3] >= 8)
                .count();
            total == 0 || (opaque as f64) < (total as f64) * 0.02
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

        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        let _ = DeleteObject(icon_info.hbmColor.into());
        let _ = DeleteObject(icon_info.hbmMask.into());

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
                chunk[3] = if (byte >> (7 - col % 8)) & 1 == 0 {
                    255
                } else {
                    0
                };
            }
        }

        // 兜底自检：处理后的像素若仍无任何可见像素（alpha 噪声致 mask 重建未覆盖、
        // mask 数据缺失等边角情况），强制不透明，保证图标可见，避免"空白方框"。
        if !pixels.chunks_exact(4).any(|c| c[3] >= 8) {
            for chunk in pixels.chunks_exact_mut(4) {
                chunk[3] = 255;
            }
            log::warn!(
                "Icon pixels fully transparent after alpha pass; forced opaque: {:?}",
                path
            );
        }

        // P2-4：唯一 tmp 名（pid + 自增序号），并发提取线程各写各的 tmp，
        // 避免同写一个 `png.tmp` 导致 PNG 损坏/rename 竞争（对齐 storage 原子写）
        let tmp_name = format!(
            "{}.tmp.{}.{}",
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            std::process::id(),
            TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let tmp_path = path.with_file_name(tmp_name);
        // 显式指定 PNG 格式：tmp 扩展名无法让 save_buffer 按扩展名推断（会永远失败）
        if let Err(e) = image::save_buffer_with_format(
            &tmp_path,
            &pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
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
fn extract_hicon(_icon_path: &str, _icon_index: i32, _icon_size: i32) -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn extract_hicon_from_target(_target: &str, _icon_size: i32) -> Option<u32> {
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
        // FNV-1a 结果确定性（无随机种子），断言固定值防回归（键含显示尺寸）
        let h = IconCache::cache_key("abc", "", 0, "", 48);
        assert_eq!(h, 0xb422_0a8b_670a_ce47);
    }

    #[test]
    fn test_cache_key_changes_with_icon_size() {
        // 改图标显示尺寸后缓存键必须变化（不同尺寸提取的 PNG 不同，须分键）
        let h48 = IconCache::cache_key("abc", "", 0, "", 48);
        let h96 = IconCache::cache_key("abc", "", 0, "", 96);
        assert_ne!(h48, h96);
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

#[cfg(test)]
mod restore_tests {
    use super::*;
    use crate::model::Item;

    /// 构造测试条目（restore_tests 与 tests 模块隔离，无法复用其私有
    /// test_item，故本地定义等价辅助）。
    fn test_item(id: &str) -> Item {
        Item {
            id: id.to_string(),
            ..Item::default()
        }
    }

    /// restored_icon_path：键校验通过且文件存在时返回路径；缺文件/空字段返回 None。
    #[test]
    fn test_restored_icon_path_hit_and_miss() {
        let cache = IconCache::new();
        let item = Item {
            id: "r1".into(),
            ..Default::default()
        };
        let p = cache.cache_path(&item);
        let _ = std::fs::remove_file(&p); // 自愈清理：上次运行中断残留不影响本测试
        // 命中：cached_icon 等于重算键路径且文件存在
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"png-bytes").unwrap();
        let mut hit = item.clone();
        hit.cached_icon = p.to_string_lossy().to_string();
        assert_eq!(cache.restored_icon_path(&hit), Some(p.clone()));
        // 未命中：键匹配但文件已删除
        let mut missing = item.clone();
        missing.cached_icon = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&p);
        assert!(cache.restored_icon_path(&missing).is_none());
        // 空 cached_icon
        let mut empty = item.clone();
        empty.cached_icon = String::new();
        assert!(cache.restored_icon_path(&empty).is_none());
    }

    /// R1：key 不匹配的 cachedIcon（手改 target/icon_path 后残留的旧键路径）必须拒绝。
    #[test]
    fn test_restored_icon_path_rejects_stale_key() {
        let cache = IconCache::new();
        let mut item = Item {
            id: "s1".into(),
            ..Default::default()
        };
        // 造旧键缓存文件，模拟"改 target 前提取结果已回写 cachedIcon"
        let old_p = cache.cache_path(&item);
        std::fs::create_dir_all(old_p.parent().unwrap()).unwrap();
        std::fs::write(&old_p, b"stale-png").unwrap();
        item.cached_icon = old_p.to_string_lossy().to_string();
        // target 变化 → 键变化 → cachedIcon 指向旧键文件 → 拒绝（走提取自愈）
        item.target = "other.exe".into();
        assert!(cache.restored_icon_path(&item).is_none());
        let _ = std::fs::remove_file(&old_p);
    }

    /// get_icon_path：缓存文件存在时返回 cache_path（按内容键定位），不存在返回 None。
    #[test]
    fn test_get_icon_path_after_cache_file_created() {
        let cache = IconCache::new();
        let item = Item {
            id: "r2".into(),
            ..Default::default()
        };
        // 自愈清理：上次运行中断残留（r2 键文件）不影响"初始无缓存"断言
        let p = cache.cache_path(&item);
        let _ = std::fs::remove_file(&p);
        assert!(cache.get_icon_path(&item).is_none(), "清理后初始无缓存");
        // 手动在 cache_path 落一个文件，模拟提取完成
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"png").unwrap();
        assert_eq!(cache.get_icon_path(&item), Some(p.clone()));
        // 删除后恢复 None
        cache.delete_cache(&item);
        assert!(
            cache.get_icon_path(&item).is_none(),
            "delete_cache 后应无缓存"
        );
    }

    /// 同一 item 的 cache_path 稳定；编辑 icon_path 后键变化（缓存失效语义）。
    #[test]
    fn test_cache_path_changes_on_icon_edit() {
        let cache = IconCache::new();
        let mut item = Item {
            id: "r3".into(),
            ..Default::default()
        };
        let p1 = cache.cache_path(&item);
        item.icon_path = "C:\\new_icon.ico".into();
        let p2 = cache.cache_path(&item);
        assert_ne!(p1, p2, "编辑 icon_path 后 cache_path 应变化");
    }

    /// H3：超上限逐出最久未访问单项（LRU），热项保留。
    #[test]
    fn test_image_cache_lru_evicts_least_recently_used() {
        let cache = IconCache::new();
        let items: Vec<Item> = (0..IMAGE_CACHE_MAX + 2)
            .map(|i| test_item(format!("id_{i}")))
            .collect();
        let buf = slint::SharedPixelBuffer::new(1, 1);
        for item in &items {
            cache.store_image(item, buf.clone());
        }
        // 访问 id_0 刷新其 LRU 序号，使其成为最新
        assert!(cache.cached_image(&items[0]).is_some());
        // 再插入一项触发逐出：此时最久未用为 id_1（序号未被刷新）
        cache.store_image(&items[IMAGE_CACHE_MAX + 1], buf.clone());
        assert!(cache.cached_image(&items[1]).is_none(), "id_1 应被逐出");
        assert!(cache.cached_image(&items[0]).is_some(), "刚访问的 id_0 应保留");
        assert!(
            cache.cached_image(&items[IMAGE_CACHE_MAX + 1]).is_some(),
            "刚插入的项应保留"
        );
    }

    /// H3：cached_image 命中刷新访问序号，参与后续逐出决策。
    #[test]
    fn test_image_cache_lru_hit_refreshes_seq() {
        let cache = IconCache::new();
        let a = test_item("a");
        let b = test_item("b");
        let buf = slint::SharedPixelBuffer::new(1, 1);
        cache.store_image(&a, buf.clone());
        cache.store_image(&b, buf.clone());
        // 持续访问 a 并插入新项至满：b 序号最旧，应先被逐出
        for i in 0..IMAGE_CACHE_MAX {
            let _ = cache.cached_image(&a);
            cache.store_image(&test_item(format!("new_{i}")), buf.clone());
        }
        assert!(cache.cached_image(&a).is_some(), "反复访问的 a 应保留");
        assert!(cache.cached_image(&b).is_none(), "未再访问的 b 应被逐出");
    }
}
