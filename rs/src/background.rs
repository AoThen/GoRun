//! 应用背景图：解码与缩放。
//! 方案 A：解码后最长边 2560px 等比封顶，内存封顶约 15MB（2560×1440×4），
//! 1080p 壁纸约 8MB。解码用项目 image crate（png/jpeg/ico/bmp），像素转入
//! SharedPixelBuffer 后由调用方以 Image::from_rgba8 包装（同 icon_cache 模式，
//! 避免在非 UI 线程持有 slint::Image 的线程边界问题）。
//! 防御：解码前先读尺寸（不解码像素），异常大的图直接拒绝——image crate
//! 的解码模型须把整张图解压为像素后才有缩图操作，无法"先缩后解码"，
//! 硬上限拒绝可避免极端图在解码阶段耗尽内存或长时间卡死。

use slint::{Rgba8Pixel, SharedPixelBuffer};

/// 解码后最长边上限。
const MAX_EDGE: u32 = 2560;

/// 解码前拒绝的边长硬上限（MAX_EDGE 的 4 倍 = 10240）。
/// 正常 4K（3840）/8K（7680）壁纸均远低于此，仅异常大的图被拒绝，
/// 防止解码阶段整图展开导致 OOM 或长时间卡死。
const MAX_DECODE_EDGE: u32 = MAX_EDGE * 4;

pub struct DecodedBackground {
    pub buffer: SharedPixelBuffer<Rgba8Pixel>,
    pub width: u32,
    pub height: u32,
}

/// 解码图片文件为 RGBA8 像素缓冲；超过 MAX_EDGE 时等比缩小。
/// 失败（路径不存在/文件损坏/格式不符/尺寸异常大）返回 None，调用方降级为纯色背景。
pub fn decode(path: &str) -> Option<DecodedBackground> {
    // 先读尺寸（不解码像素）：异常大的图直接拒绝，避免解码阶段 OOM/卡死。
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let (dw, dh) = reader.into_dimensions().ok()?;
    if dw == 0 || dh == 0 || dw.max(dh) > MAX_DECODE_EDGE {
        return None;
    }

    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }

    let img = if w.max(h) > MAX_EDGE {
        let scale = MAX_EDGE as f32 / w.max(h) as f32;
        let nw = ((w as f32 * scale).round() as u32).max(1);
        let nh = ((h as f32 * scale).round() as u32).max(1);
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let (w, h) = (img.width(), img.height());

    // image::Rgba<u8> 与 slint::Rgba8Pixel 同为 4 字节 RGBA，逐像素安全转换
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    for (dst, src) in buffer
        .make_mut_slice()
        .iter_mut()
        .zip(img.as_raw().chunks_exact(4))
    {
        *dst = Rgba8Pixel {
            r: src[0],
            g: src[1],
            b: src[2],
            a: src[3],
        };
    }
    Some(DecodedBackground {
        buffer,
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEQ: AtomicU32 = AtomicU32::new(0);

    fn temp_png(name: &str, width: u32, height: u32, pixel: [u8; 4]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gorun_bg_test_{}_{}_{}.png",
            std::process::id(),
            name,
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let img = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn decode_png_normal_keeps_dimensions_and_pixels() {
        let path = temp_png("normal", 64, 48, [255, 128, 0, 255]);
        let mut decoded = decode(path.to_str().unwrap()).expect("应解码成功");
        assert_eq!(decoded.width, 64);
        assert_eq!(decoded.height, 48);
        // 首像素与写入像素一致（未缩放时像素应精确）
        let px = decoded.buffer.make_mut_slice();
        assert_eq!(
            px[0],
            Rgba8Pixel {
                r: 255,
                g: 128,
                b: 0,
                a: 255
            }
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn decode_scales_down_over_limit_keeps_aspect() {
        // 超上限（2560）：5120×2880（16:9）→ 缩放到最长边 2560
        let path = temp_png("large", 5120, 2880, [10, 20, 30, 255]);
        let decoded = decode(path.to_str().unwrap()).expect("应解码成功");
        assert_eq!(decoded.width, 2560);
        assert_eq!(decoded.height, 1440);
        // 缩放后仍保持 16:9
        assert_eq!(decoded.width * 9, decoded.height * 16);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn decode_nonexistent_returns_none() {
        let path = std::env::temp_dir().join("gorun_bg_definitely_missing.png");
        assert!(decode(path.to_str().unwrap()).is_none());
    }

    #[test]
    fn decode_oversized_returns_none() {
        // 边长超过 MAX_DECODE_EDGE（10240）的图在解码前即被拒绝，不进入解码阶段
        let path = temp_png("huge", MAX_DECODE_EDGE + 1, 64, [1, 2, 3, 255]);
        assert!(
            decode(path.to_str().unwrap()).is_none(),
            "超硬上限的图应返回 None"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn decode_corrupt_returns_none() {
        let path =
            std::env::temp_dir().join(format!("gorun_bg_corrupt_{}.png", std::process::id()));
        fs::write(&path, b"not a real png").unwrap();
        assert!(decode(path.to_str().unwrap()).is_none());
        fs::remove_file(&path).ok();
    }
}
