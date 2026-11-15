//! 应用背景图：解码与缩放。
//! 方案 A：解码后最长边 2560px 等比封顶，内存封顶约 15MB（2560×1440×4），
//! 1080p 壁纸约 8MB。解码用项目 image crate（png/jpeg），像素转入
//! SharedPixelBuffer 后由调用方以 Image::from_rgba8 包装（同 icon_cache 模式，
//! 避免在非 UI 线程持有 slint::Image 的线程边界问题）。

use slint::{Rgba8Pixel, SharedPixelBuffer};

/// 解码后最长边上限。
const MAX_EDGE: u32 = 2560;

pub struct DecodedBackground {
    pub buffer: SharedPixelBuffer<Rgba8Pixel>,
    pub width: u32,
    pub height: u32,
}

/// 解码图片文件为 RGBA8 像素缓冲；超过 MAX_EDGE 时等比缩小。
/// 失败（路径不存在/文件损坏/格式不符）返回 None，调用方降级为纯色背景。
pub fn decode(path: &str) -> Option<DecodedBackground> {
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
