use figcraft_color::{ColorProfile, ImagePaint, ImageScaleMode, Paint, PaintKind};
use figcraft_doc::Document;
use std::io::Cursor;
use tiny_skia::{FilterQuality, Paint as TsPaint, Pixmap, Rect as TsRect, SpreadMode, Transform};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>, // straight RGBA8 in the document color space
}

fn covers_box(img: &ImagePaint, iw: f64, ih: f64, w: f64, h: f64) -> bool {
    match img.scale_mode {
        ImageScaleMode::Fill => true,
        ImageScaleMode::Crop => {
            let m = img.effective_crop(iw, ih, w, h);
            let (x0, y0, x1, y1) = ImagePaint::crop_rect(m);
            m[1] == 0.0 && m[2] == 0.0 && x0.min(x1) <= 0.0 && y0.min(y1) <= 0.0 && x0.max(x1) >= 1.0 && y0.max(y1) >= 1.0
        }
        ImageScaleMode::Fit | ImageScaleMode::Tile => false,
    }
}

pub fn raster_image_paint(doc: &Document, paint: &Paint, layer_size: (f64, f64), max_side: u32) -> Result<RasterImage, String> {
    if !paint.visible || paint.opacity <= 0.0 {
        return Err("paint is invisible or has zero opacity".into());
    }
    let img = match &paint.kind {
        PaintKind::Image(img) => img,
        _ => return Err("paint is not an image paint".into()),
    };
    if img.scale_mode == ImageScaleMode::Tile {
        return Err("tile scale mode is not supported for tracing".into());
    }

    let (w, h) = layer_size;
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err("layer size must be finite and positive".into());
    }
    if w > 1_000_000.0 || h > 1_000_000.0 {
        return Err("layer dimensions exceed maximum bounds".into());
    }

    let data = doc.images.get(&img.image).ok_or_else(|| "image not found in document".to_string())?;

    // Preflight actual encoded bytes and headers before decoding.
    if data.bytes.is_empty() {
        return Err("empty image data".into());
    }
    if data.bytes.len() > 32 * 1024 * 1024 {
        return Err("image compressed bytes exceed 32 MiB limit".into());
    }

    let reader =
        image::ImageReader::new(Cursor::new(data.bytes.as_slice())).with_guessed_format().map_err(|e| format!("corrupt image format: {e}"))?;
    let (header_w, header_h) = reader.into_dimensions().map_err(|e| format!("corrupt image header: {e}"))?;
    if header_w == 0 || header_h == 0 {
        return Err("image dimensions must be nonzero".into());
    }
    let total_pixels = (header_w as u64).checked_mul(header_h as u64).ok_or("pixel count overflow")?;
    if total_pixels > 16_777_216 {
        return Err("source image exceeds 16,777,216 pixels ceiling".into());
    }
    let decoded_bytes = total_pixels.checked_mul(4).ok_or("decoded byte overflow")?;
    if decoded_bytes > 64 * 1024 * 1024 {
        return Err("source image exceeds 64 MiB decoded output limit".into());
    }
    if (data.width != 0 && data.width != header_w) || (data.height != 0 && data.height != header_h) {
        return Err("stored image dimensions do not match encoded header".into());
    }

    // Decode with thumbnail up to max_side.
    let max_decode_side = max_side.clamp(16, 512);
    let (mut pm, srgb) = crate::images::decode_within(&data.bytes, doc.color_profile, max_decode_side, 64 << 20)
        .ok_or_else(|| "failed to decode image".to_string())?;

    crate::images::finish(&mut pm, &img.filters, if srgb { doc.color_profile } else { ColorProfile::Srgb });

    // Target raster dimensions.
    let max_dim = w.max(h);
    let s = (f64::from(max_side) / max_dim).min(1.0);
    let rw = ((w * s).ceil() as u32).clamp(1, 512);
    let rh = ((h * s).ceil() as u32).clamp(1, 512);

    let mut target_pixmap = Pixmap::new(rw, rh).ok_or_else(|| "failed to allocate raster pixmap".to_string())?;

    let base_transform = crate::paint::image_transform(img, f64::from(pm.width()), f64::from(pm.height()), w, h);
    let op = paint.opacity.clamp(0.0, 1.0);

    let padded_holder;
    let (pm_ref, pat_transform) = if covers_box(img, f64::from(pm.width()), f64::from(pm.height()), w, h) {
        (&pm, base_transform)
    } else {
        padded_holder = crate::images::pad(&pm).ok_or_else(|| "failed to pad image".to_string())?;
        let t = Transform::from_translate(-1.0, -1.0).post_concat(base_transform);
        (&padded_holder, t)
    };

    let pattern = tiny_skia::Pattern::new(pm_ref.as_ref(), SpreadMode::Pad, FilterQuality::Bicubic, op, pat_transform);

    let ts_paint = TsPaint { shader: pattern, anti_alias: true, blend_mode: tiny_skia::BlendMode::SourceOver, ..Default::default() };

    let ts_scale = Transform::from_scale((f64::from(rw) / w) as f32, (f64::from(rh) / h) as f32);
    let local_rect = TsRect::from_xywh(0.0, 0.0, w as f32, h as f32).ok_or_else(|| "invalid local rect".to_string())?;
    target_pixmap.fill_rect(local_rect, &ts_paint, ts_scale, None);

    // Convert premultiplied pixels to straight RGBA.
    let total = (rw * rh) as usize;
    let mut pixels = Vec::with_capacity(total);
    for chunk in target_pixmap.data().chunks_exact(4) {
        let a = u32::from(chunk.get(3).copied().unwrap_or(0));
        if a == 0 {
            pixels.push([0, 0, 0, 0]);
        } else {
            let unpremul = |c: u8| -> u8 {
                let v = u32::from(c);
                ((v * 255 + a / 2) / a).min(255) as u8
            };
            pixels.push([
                unpremul(chunk.first().copied().unwrap_or(0)),
                unpremul(chunk.get(1).copied().unwrap_or(0)),
                unpremul(chunk.get(2).copied().unwrap_or(0)),
                a as u8,
            ]);
        }
    }

    Ok(RasterImage { width: rw, height: rh, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use figcraft_color::{ImageFilters, ImagePaint, ImageScaleMode, Paint, PaintKind};
    use figcraft_doc::{Document, ImageData};
    use image::{ImageBuffer, Rgba};
    use std::io::Cursor;
    use std::sync::Arc;

    fn make_png(w: u32, h: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
        let mut img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::new(w, h);
        for (i, p) in pixels.iter().enumerate() {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            img.put_pixel(x, y, Rgba(*p));
        }
        let mut cursor = Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        cursor.into_inner()
    }

    fn test_paint(hash: String, scale_mode: ImageScaleMode, rotation: i32, filters: ImageFilters, opacity: f32) -> Paint {
        Paint {
            kind: PaintKind::Image(ImagePaint { image: hash, scale_mode, scale: 1.0, rotation, filters, crop: None }),
            visible: true,
            opacity,
            blend: figcraft_color::BlendMode::Normal,
            color_alpha: 1.0,
        }
    }

    #[test]
    fn raster_keeps_alpha_and_aspect() {
        let mut doc = Document::new("test");
        let png = make_png(
            4,
            2,
            &[[255, 0, 0, 255], [255, 0, 0, 255], [255, 0, 0, 255], [255, 0, 0, 255], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        );
        let hash = "img_alpha_aspect".to_string();
        doc.images.insert(hash.clone(), ImageData { bytes: Arc::new(png), width: 4, height: 2 });

        let paint = test_paint(hash, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        let raster = raster_image_paint(&doc, &paint, (80.0, 40.0), 32).unwrap();
        assert_eq!(raster.width, 32);
        assert_eq!(raster.height, 16);
        assert_eq!(raster.pixels.len(), 512);

        let bottom_row_sample = raster.pixels[15 * 32 + 16];
        assert_eq!(bottom_row_sample, [0, 0, 0, 0]);

        let top_row_sample = raster.pixels[0];
        assert_eq!(top_row_sample[3], 255);
        assert!(top_row_sample[0] > 200);
    }

    #[test]
    fn raster_unpremultiplies() {
        let mut doc = Document::new("test");
        let png = make_png(1, 1, &[[255, 0, 0, 128]]);
        let hash = "img_unpremul".to_string();
        doc.images.insert(hash.clone(), ImageData { bytes: Arc::new(png), width: 1, height: 1 });

        let paint = test_paint(hash, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        let raster = raster_image_paint(&doc, &paint, (10.0, 10.0), 16).unwrap();
        let p = raster.pixels[0];
        assert_eq!(p[0], 255);
        assert_eq!(p[1], 0);
        assert_eq!(p[2], 0);
        assert_eq!(p[3], 128);
    }

    #[test]
    fn raster_fit_and_rotated_crop() {
        let mut doc = Document::new("test");
        let mut pixels = Vec::new();
        for _y in 0..10 {
            for x in 0..20 {
                if x < 10 {
                    pixels.push([0, 0, 255, 255]);
                } else {
                    pixels.push([0, 255, 0, 255]);
                }
            }
        }
        let png = make_png(20, 10, &pixels);
        let hash = "img_fit_crop".to_string();
        doc.images.insert(hash.clone(), ImageData { bytes: Arc::new(png), width: 20, height: 10 });

        let paint_fit = test_paint(hash.clone(), ImageScaleMode::Fit, 0, ImageFilters::default(), 1.0);
        let raster_fit = raster_image_paint(&doc, &paint_fit, (32.0, 32.0), 32).unwrap();
        assert_eq!(raster_fit.width, 32);
        assert_eq!(raster_fit.height, 32);
        assert_eq!(raster_fit.pixels[0], [0, 0, 0, 0]);
        assert_eq!(raster_fit.pixels[31 * 32 + 16], [0, 0, 0, 0]);
        assert!(raster_fit.pixels[16 * 32 + 16][3] > 200);

        let paint_rot = test_paint(hash, ImageScaleMode::Fill, 1, ImageFilters::default(), 1.0);
        let raster_rot = raster_image_paint(&doc, &paint_rot, (32.0, 32.0), 32).unwrap();
        assert_eq!(raster_rot.width, 32);
        assert_eq!(raster_rot.height, 32);
        assert!(raster_rot.pixels[0][3] > 200);
    }

    #[test]
    fn raster_limits_before_decode() {
        let mut doc = Document::new("test");

        // 1. Corrupt data
        let hash_corrupt = "corrupt".to_string();
        doc.images.insert(hash_corrupt.clone(), ImageData { bytes: Arc::new(vec![1, 2, 3, 4]), width: 10, height: 10 });
        let p_corrupt = test_paint(hash_corrupt, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        assert!(raster_image_paint(&doc, &p_corrupt, (32.0, 32.0), 32).is_err());

        // 2. Mismatched stored dimensions
        let valid_png = make_png(2, 2, &[[255, 0, 0, 255]; 4]);
        let hash_mismatch = "mismatch".to_string();
        doc.images.insert(hash_mismatch.clone(), ImageData { bytes: Arc::new(valid_png), width: 50, height: 50 });
        let p_mismatch = test_paint(hash_mismatch, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        assert!(raster_image_paint(&doc, &p_mismatch, (32.0, 32.0), 32).is_err());

        // 3. Compressed bytes > 32 MiB
        let hash_huge = "huge".to_string();
        doc.images.insert(hash_huge.clone(), ImageData { bytes: Arc::new(vec![0u8; 33 * 1024 * 1024]), width: 10, height: 10 });
        let p_huge = test_paint(hash_huge, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        assert!(raster_image_paint(&doc, &p_huge, (32.0, 32.0), 32).is_err());
    }

    #[test]
    fn raster_checks_warm_cache() {
        let mut doc = Document::new("test");
        let mut png = Vec::new();
        png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        png.extend_from_slice(&[0, 0, 0, 13]);
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(b"IHDR");
        ihdr.extend_from_slice(&5000u32.to_be_bytes());
        ihdr.extend_from_slice(&5000u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        let crc = crc32fast::hash(&ihdr);
        png.extend_from_slice(&ihdr);
        png.extend_from_slice(&crc.to_be_bytes());

        let hash = "oversized_header".to_string();
        doc.images.insert(hash.clone(), ImageData { bytes: Arc::new(png), width: 5000, height: 5000 });
        let paint = test_paint(hash, ImageScaleMode::Fill, 0, ImageFilters::default(), 1.0);
        assert!(raster_image_paint(&doc, &paint, (32.0, 32.0), 32).is_err());
    }

    #[test]
    fn raster_filters_and_paint_opacity_once() {
        let mut doc = Document::new("test");
        let png = make_png(1, 1, &[[200, 200, 200, 255]]);
        let hash = "img_filters".to_string();
        doc.images.insert(hash.clone(), ImageData { bytes: Arc::new(png), width: 1, height: 1 });

        let filters = ImageFilters { exposure: 0.5, ..Default::default() };

        let paint = test_paint(hash, ImageScaleMode::Fill, 0, filters, 0.5);
        let raster = raster_image_paint(&doc, &paint, (10.0, 10.0), 16).unwrap();
        let p = raster.pixels[0];
        assert!((i32::from(p[3]) - 128).abs() <= 2);
    }
}
