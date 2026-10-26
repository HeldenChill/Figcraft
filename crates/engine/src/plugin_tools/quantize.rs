use figcraft_render::image_tools::RasterImage;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelImage {
    pub width: u32,
    pub height: u32,
    pub labels: Vec<Option<u8>>,
    pub palette: Vec<[u8; 4]>,
}

fn bad(msg: impl Into<String>) -> crate::EngineError {
    crate::bad("image.trace", msg)
}

pub(crate) fn monochrome(raster: &RasterImage, threshold: u8, invert: bool, alpha_cutoff: u8) -> crate::Result<LabelImage> {
    let total = (raster.width as usize).checked_mul(raster.height as usize).ok_or_else(|| bad("image dimensions overflow"))?;

    if raster.pixels.len() != total {
        return Err(bad("raster pixel buffer length does not match dimensions"));
    }

    let mut labels = Vec::with_capacity(total);
    let mut has_foreground = false;

    for p in &raster.pixels {
        let a = p[3];
        if a < alpha_cutoff {
            labels.push(None);
            continue;
        }
        let lum = ((54 * u32::from(p[0]) + 183 * u32::from(p[1]) + 19 * u32::from(p[2]) + 128) / 256) as u8;
        let included = if invert { lum > threshold } else { lum <= threshold };
        if included {
            labels.push(Some(0));
            has_foreground = true;
        } else {
            labels.push(None);
        }
    }

    let palette = if has_foreground { vec![[0, 0, 0, 255]] } else { Vec::new() };

    Ok(LabelImage { width: raster.width, height: raster.height, labels, palette })
}

#[derive(Clone, Debug)]
struct ColorBox {
    colors: Vec<([u8; 4], u64)>,
    population: u64,
}

impl ColorBox {
    fn new(colors: Vec<([u8; 4], u64)>) -> Self {
        let population = colors.iter().map(|(_, c)| *c).sum();
        Self { colors, population }
    }

    fn channel_ranges(&self) -> ([u8; 2], [u8; 2], [u8; 2], [u8; 2]) {
        let mut min_r = 255;
        let mut max_r = 0;
        let mut min_g = 255;
        let mut max_g = 0;
        let mut min_b = 255;
        let mut max_b = 0;
        let mut min_a = 255;
        let mut max_a = 0;

        for (rgba, _) in &self.colors {
            min_r = min_r.min(rgba[0]);
            max_r = max_r.max(rgba[0]);
            min_g = min_g.min(rgba[1]);
            max_g = max_g.max(rgba[1]);
            min_b = min_b.min(rgba[2]);
            max_b = max_b.max(rgba[2]);
            min_a = min_a.min(rgba[3]);
            max_a = max_a.max(rgba[3]);
        }
        ([min_r, max_r], [min_g, max_g], [min_b, max_b], [min_a, max_a])
    }

    fn best_split_channel(&self) -> usize {
        let (r, g, b, a) = self.channel_ranges();
        let range_r = r[1] - r[0];
        let range_g = g[1] - g[0];
        let range_b = b[1] - b[0];
        let range_a = a[1] - a[0];

        let max_range = range_r.max(range_g).max(range_b).max(range_a);
        if max_range == range_r {
            0
        } else if max_range == range_g {
            1
        } else if max_range == range_b {
            2
        } else {
            3
        }
    }

    fn channel_max_range(&self) -> u8 {
        let (r, g, b, a) = self.channel_ranges();
        (r[1] - r[0]).max(g[1] - g[0]).max(b[1] - b[0]).max(a[1] - a[0])
    }

    fn min_color(&self) -> [u8; 4] {
        self.colors.iter().map(|(c, _)| *c).min().unwrap_or([0, 0, 0, 0])
    }

    fn mean_rgba(&self) -> [u8; 4] {
        if self.population == 0 {
            return [0, 0, 0, 0];
        }
        let mut sum_r: u64 = 0;
        let mut sum_g: u64 = 0;
        let mut sum_b: u64 = 0;
        let mut sum_a: u64 = 0;
        for (rgba, count) in &self.colors {
            sum_r = sum_r.saturating_add(u64::from(rgba[0]) * count);
            sum_g = sum_g.saturating_add(u64::from(rgba[1]) * count);
            sum_b = sum_b.saturating_add(u64::from(rgba[2]) * count);
            sum_a = sum_a.saturating_add(u64::from(rgba[3]) * count);
        }
        let half = self.population / 2;
        [
            ((sum_r + half) / self.population).min(255) as u8,
            ((sum_g + half) / self.population).min(255) as u8,
            ((sum_b + half) / self.population).min(255) as u8,
            ((sum_a + half) / self.population).min(255) as u8,
        ]
    }
}

pub(crate) fn quantize(raster: &RasterImage, colors: u8, alpha_cutoff: u8) -> crate::Result<LabelImage> {
    if !(2..=16).contains(&colors) {
        return Err(bad(format!("colors must be between 2 and 16, got {colors}")));
    }

    let total = (raster.width as usize).checked_mul(raster.height as usize).ok_or_else(|| bad("image dimensions overflow"))?;

    if raster.pixels.len() != total {
        return Err(bad("raster pixel buffer length does not match dimensions"));
    }

    // Build frequency histogram of included pixels.
    let mut histogram: BTreeMap<[u8; 4], u64> = BTreeMap::new();
    for p in &raster.pixels {
        if p[3] >= alpha_cutoff {
            *histogram.entry(*p).or_insert(0) += 1;
        }
    }

    if histogram.is_empty() {
        return Ok(LabelImage { width: raster.width, height: raster.height, labels: vec![None; total], palette: Vec::new() });
    }

    let initial_colors: Vec<([u8; 4], u64)> = histogram.into_iter().collect();

    let mut boxes = vec![ColorBox::new(initial_colors)];

    while boxes.len() < colors as usize {
        // Find best box to split: greatest population, then channel range, then min color.
        let mut best_idx = None;
        let mut best_score = (0u64, 0u8, [255u8, 255u8, 255u8, 255u8]);

        for (idx, b) in boxes.iter().enumerate() {
            if b.colors.len() < 2 || b.channel_max_range() == 0 {
                continue;
            }
            let score = (b.population, b.channel_max_range(), b.min_color());
            if best_idx.is_none() || score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }

        let Some(idx_to_split) = best_idx else {
            break;
        };

        let mut b = boxes.swap_remove(idx_to_split);
        let ch = b.best_split_channel();

        // Stable-sort by chosen channel, then full RGBA.
        b.colors.sort_by(|a, b| a.0[ch].cmp(&b.0[ch]).then_with(|| a.0.cmp(&b.0)));

        // Split at weighted median, requiring nonempty halves.
        let half_pop = b.population / 2;
        let mut acc: u64 = 0;
        let mut split_at = 1;
        let mut min_diff = u64::MAX;

        for i in 1..b.colors.len() {
            let count = b.colors.get(i - 1).map(|(_, c)| *c).unwrap_or(0);
            acc = acc.saturating_add(count);
            let diff = acc.abs_diff(half_pop);
            if diff < min_diff {
                min_diff = diff;
                split_at = i;
            }
        }

        let right_colors = b.colors.split_off(split_at);
        boxes.push(ColorBox::new(b.colors));
        boxes.push(ColorBox::new(right_colors));
    }

    // Sort boxes deterministically by mean RGBA.
    let mut box_means: Vec<([u8; 4], usize)> = boxes.iter().enumerate().map(|(i, b)| (b.mean_rgba(), i)).collect();
    box_means.sort_by_key(|a| a.0);

    // Deduplicate palette if identical rounded colors exist.
    let mut palette: Vec<[u8; 4]> = Vec::new();
    let mut box_to_palette_idx = vec![0u8; boxes.len()];

    for (mean, original_box_idx) in box_means {
        let pal_idx = match palette.iter().position(|&p| p == mean) {
            Some(existing) => existing as u8,
            None => {
                let idx = palette.len() as u8;
                palette.push(mean);
                idx
            }
        };
        if let Some(target) = box_to_palette_idx.get_mut(original_box_idx) {
            *target = pal_idx;
        }
    }

    // Build color -> palette index lookup map.
    let mut color_to_idx: BTreeMap<[u8; 4], u8> = BTreeMap::new();
    for (box_idx, b) in boxes.iter().enumerate() {
        let pal_idx = box_to_palette_idx.get(box_idx).copied().unwrap_or(0);
        for (rgba, _) in &b.colors {
            color_to_idx.insert(*rgba, pal_idx);
        }
    }

    let mut labels = Vec::with_capacity(total);
    for p in &raster.pixels {
        if p[3] < alpha_cutoff {
            labels.push(None);
        } else {
            labels.push(color_to_idx.get(p).copied());
        }
    }

    Ok(LabelImage { width: raster.width, height: raster.height, labels, palette })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_and_alpha_edges() {
        let pixels = vec![
            [100, 100, 100, 15],  // alpha 15 -> excluded at cutoff 16
            [100, 100, 100, 16],  // alpha 16 -> included at cutoff 16
            [128, 128, 128, 255], // lum ~128 -> included at thresh 128 (non-inverted)
            [129, 129, 129, 255], // lum ~129 -> excluded at thresh 128 (non-inverted)
        ];
        let raster = RasterImage { width: 4, height: 1, pixels };

        let non_inverted = monochrome(&raster, 128, false, 16).unwrap();
        assert_eq!(non_inverted.labels[0], None);
        assert_eq!(non_inverted.labels[1], Some(0));
        assert_eq!(non_inverted.labels[2], Some(0));
        assert_eq!(non_inverted.labels[3], None);

        let inverted = monochrome(&raster, 128, true, 16).unwrap();
        assert_eq!(inverted.labels[0], None);
        assert_eq!(inverted.labels[2], None);
        assert_eq!(inverted.labels[3], Some(0));
    }

    #[test]
    fn quantize_is_deterministic_and_bounded() {
        let mut pixels = Vec::new();
        // Gradient and noise
        for i in 0..100 {
            let v = (i * 2) as u8;
            pixels.push([v, 255 - v, (v / 2), 255]);
        }
        let raster = RasterImage { width: 10, height: 10, pixels };

        let q1 = quantize(&raster, 4, 16).unwrap();
        let q2 = quantize(&raster, 4, 16).unwrap();

        assert_eq!(q1.palette, q2.palette);
        assert_eq!(q1.labels, q2.labels);
        assert!(q1.palette.len() <= 4);
    }
}
