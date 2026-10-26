use std::collections::BTreeSet;

use figcraft_color::{BlendMode, ImagePaint, ImageScaleMode, Paint, PaintKind};
use figcraft_doc::props::{Constraints, HConstraint, VConstraint};
use figcraft_doc::{Corners, Document, Node, NodeId, NodeKind};
use figcraft_geom::{Affine, Rect};
use serde_json::Value;

use super::ToolReport;
use super::targets::{parse_targets, protected_node};
use crate::{EngineError, Result, Session, bad};

pub(crate) const NINE_SLICE_KEY: &str = "figcraft.tools.nineSlice";
pub(crate) const NINE_SLICE_CELL_KEY: &str = "figcraft.tools.nineSliceCell";

const CELL_NAMES: [&str; 9] = ["Top left", "Top", "Top right", "Left", "Center", "Right", "Bottom left", "Bottom", "Bottom right"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SliceInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

pub(crate) fn minimum_size(node: &Node) -> Option<(f64, f64)> {
    if !matches!(node.kind, NodeKind::Frame) {
        return None;
    }
    let insets = parse_insets_metadata(node)?;
    Some((insets.left + insets.right + 1e-6, insets.top + insets.bottom + 1e-6))
}

pub(crate) fn parse_insets_metadata(node: &Node) -> Option<SliceInsets> {
    if !matches!(node.kind, NodeKind::Frame) {
        return None;
    }
    let raw = node.plugin_data.get(NINE_SLICE_KEY)?;
    let parsed: Value = serde_json::from_str(raw).ok()?;
    if parsed.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return None;
    }
    let arr = parsed.get("insets").and_then(|v| v.as_array())?;
    if arr.len() != 4 {
        return None;
    }
    let top = arr[0].as_f64()?;
    let right = arr[1].as_f64()?;
    let bottom = arr[2].as_f64()?;
    let left = arr[3].as_f64()?;
    if !top.is_finite() || !right.is_finite() || !bottom.is_finite() || !left.is_finite() {
        return None;
    }
    if top <= 0.0 || right <= 0.0 || bottom <= 0.0 || left <= 0.0 {
        return None;
    }
    Some(SliceInsets { top, right, bottom, left })
}

pub(crate) fn affine_mul(m: [f64; 6], q: [f64; 6]) -> [f64; 6] {
    [
        m[0] * q[0] + m[2] * q[1],
        m[1] * q[0] + m[3] * q[1],
        m[0] * q[2] + m[2] * q[3],
        m[1] * q[2] + m[3] * q[3],
        m[0] * q[4] + m[2] * q[5] + m[4],
        m[1] * q[4] + m[3] * q[5] + m[5],
    ]
}

pub(crate) fn slice_paints(
    paint: &ImagePaint,
    image_size: (u32, u32),
    layer_size: (f64, f64),
    insets: SliceInsets,
) -> Result<Vec<(Rect, ImagePaint)>> {
    let (iw, ih) = (image_size.0 as f64, image_size.1 as f64);
    let (w, h) = layer_size;
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 || w > 1_000_000.0 || h > 1_000_000.0 {
        return Err(bad("image.nineSlice", "invalid layer dimensions"));
    }
    if insets.left <= 0.0 || insets.right <= 0.0 || insets.top <= 0.0 || insets.bottom <= 0.0 {
        return Err(bad("image.nineSlice", "insets must be positive"));
    }
    if !insets.left.is_finite() || !insets.right.is_finite() || !insets.top.is_finite() || !insets.bottom.is_finite() {
        return Err(bad("image.nineSlice", "insets must be finite"));
    }
    if insets.left + insets.right >= w {
        return Err(bad("image.nineSlice", "horizontal insets sum must be less than layer width"));
    }
    if insets.top + insets.bottom >= h {
        return Err(bad("image.nineSlice", "vertical insets sum must be less than layer height"));
    }

    let m = match paint.scale_mode {
        ImageScaleMode::Crop => paint.effective_crop(iw, ih, w, h),
        ImageScaleMode::Fill => {
            let (rw, rh) = paint.rotated_size(iw.max(1.0), ih.max(1.0));
            let s = (w / rw).max(h / rh);
            let (ox, oy) = ((w - rw * s) / 2.0, (h - rh * s) / 2.0);
            [w / (rw * s), 0.0, 0.0, h / (rh * s), -ox / (rw * s), -oy / (rh * s)]
        }
        ImageScaleMode::Fit => {
            let (rw, rh) = paint.rotated_size(iw.max(1.0), ih.max(1.0));
            let s = (w / rw).min(h / rh);
            let (ox, oy) = ((w - rw * s) / 2.0, (h - rh * s) / 2.0);
            [w / (rw * s), 0.0, 0.0, h / (rh * s), -ox / (rw * s), -oy / (rh * s)]
        }
        _ => return Err(bad("image.nineSlice", "unsupported image scale mode")),
    };

    let col_widths = [insets.left, w - insets.left - insets.right, insets.right];
    let col_xs = [0.0, insets.left, w - insets.right];
    let row_heights = [insets.top, h - insets.top - insets.bottom, insets.bottom];
    let row_ys = [0.0, insets.top, h - insets.bottom];

    let mut out = Vec::with_capacity(9);
    for r in 0..3 {
        for c in 0..3 {
            let cx = col_xs[c];
            let cy = row_ys[r];
            let cw = col_widths[c];
            let ch = row_heights[r];
            let q = [cw / w, 0.0, 0.0, ch / h, cx / w, cy / h];
            let cell_crop = affine_mul(m, q);
            let mut cp = paint.clone();
            cp.scale_mode = ImageScaleMode::Crop;
            cp.crop = Some(cell_crop);
            out.push((Rect::new(cx, cy, cx + cw, cy + ch), cp));
        }
    }
    Ok(out)
}

pub(crate) fn settle_minimums(doc: &mut Document, before: &Document, candidates: &BTreeSet<NodeId>) {
    let touched: Vec<NodeId> = candidates.iter().copied().filter(|&id| doc.contains(id)).collect();
    for id in touched {
        let (insets, current_w, current_h) = {
            let Some(n) = doc.node(id) else { continue };
            if !matches!(n.kind, NodeKind::Frame) {
                continue;
            }
            let Some(insets) = parse_insets_metadata(n) else { continue };
            let before_dim = before.node(id).map(|b| (b.width, b.height));
            if before_dim == Some((n.width, n.height)) {
                continue;
            }
            (insets, n.width, n.height)
        };

        let min_w = insets.left + insets.right + 1e-6;
        let min_h = insets.top + insets.bottom + 1e-6;
        let clamped_w = current_w.max(min_w);
        let clamped_h = current_h.max(min_h);

        if ((clamped_w - current_w).abs() > 1e-6 || (clamped_h - current_h).abs() > 1e-6)
            && let Ok(frame_mut) = doc.get_mut(id)
        {
            frame_mut.width = clamped_w;
            frame_mut.height = clamped_h;
        }

        let children = match doc.node(id) {
            Some(frame) => frame.children.clone(),
            None => continue,
        };
        if children.len() != 9 {
            continue;
        }

        let mut cells: [Option<NodeId>; 9] = [None; 9];
        let mut valid_cells = true;
        for &cid in &children {
            let Some(cn) = doc.node(cid) else {
                valid_cells = false;
                break;
            };
            if !matches!(cn.kind, NodeKind::Rectangle) {
                valid_cells = false;
                break;
            }
            let Some(tag) = cn.plugin_data.get(NINE_SLICE_CELL_KEY) else {
                valid_cells = false;
                break;
            };
            let Ok(idx) = tag.parse::<usize>() else {
                valid_cells = false;
                break;
            };
            if idx >= 9 || cells[idx].is_some() {
                valid_cells = false;
                break;
            }
            cells[idx] = Some(cid);
        }

        if !valid_cells || cells.iter().any(|c| c.is_none()) {
            continue;
        }

        let col_widths = [insets.left, (clamped_w - insets.left - insets.right).max(0.0), insets.right];
        let col_xs = [0.0, insets.left, clamped_w - insets.right];
        let row_heights = [insets.top, (clamped_h - insets.top - insets.bottom).max(0.0), insets.bottom];
        let row_ys = [0.0, insets.top, clamped_h - insets.bottom];

        for (idx, cell_id) in cells.iter().enumerate() {
            let child_id = match cell_id {
                Some(cid) => *cid,
                None => continue,
            };
            let r = idx / 3;
            let c = idx % 3;
            let cx = col_xs[c];
            let cy = row_ys[r];
            let cw = col_widths[c];
            let ch = row_heights[r];
            if let Ok(child_mut) = doc.get_mut(child_id) {
                child_mut.transform = Affine::translate((cx, cy));
                child_mut.width = cw;
                child_mut.height = ch;
            }
        }
    }
}

pub(crate) fn nine_slice(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "image.nineSlice";
    if crate::board::is_board(s.doc()?) {
        return Err(EngineError::Disabled(cmd.into(), "Design documents only".into()));
    }

    let targets = parse_targets(cmd, s, p)?;
    if targets.len() != 1 {
        return Err(bad(cmd, "exactly one layer must be selected"));
    }
    let src_id = targets[0];
    let doc = s.doc()?;
    if !doc.contains(src_id) {
        return Err(EngineError::NoNode(src_id));
    }
    if protected_node(doc, src_id) {
        return Err(bad(cmd, "selected layer is locked or inside a protected component instance"));
    }

    let src = doc.node(src_id).ok_or(EngineError::NoNode(src_id))?;
    if !matches!(src.kind, NodeKind::Rectangle) {
        return Err(bad(cmd, "source layer must be a rectangle"));
    }
    if !src.children.is_empty() {
        return Err(bad(cmd, "source layer must not have children"));
    }
    let visible_fills: Vec<&Paint> = src.fills.iter().filter(|f| f.visible).collect();
    if visible_fills.len() != 1 {
        return Err(bad(cmd, "source must have exactly one visible fill"));
    }
    let fill = visible_fills[0];
    let img = match &fill.kind {
        PaintKind::Image(img) => img,
        _ => return Err(bad(cmd, "source fill must be an image")),
    };
    if img.scale_mode != ImageScaleMode::Fill && img.scale_mode != ImageScaleMode::Fit && img.scale_mode != ImageScaleMode::Crop {
        return Err(bad(cmd, "unsupported image scale mode"));
    }
    if src.strokes.iter().any(|st| st.visible) {
        return Err(bad(cmd, "strokes not supported on nine-slice source"));
    }
    if !src.effects.is_empty() {
        return Err(bad(cmd, "effects not supported on nine-slice source"));
    }
    if src.corners != Corners::default() {
        return Err(bad(cmd, "corner radii not supported on nine-slice source"));
    }
    if src.is_mask {
        return Err(bad(cmd, "mask nodes not supported as nine-slice source"));
    }
    if src.blend != BlendMode::Normal && src.blend != BlendMode::PassThrough {
        return Err(bad(cmd, "blend modes not supported on nine-slice source"));
    }
    if fill.blend != BlendMode::Normal {
        return Err(bad(cmd, "fill blend modes not supported on nine-slice source"));
    }
    if src.width <= 0.0 || src.height <= 0.0 || src.width > 1_000_000.0 || src.height > 1_000_000.0 {
        return Err(bad(cmd, "source layer dimensions out of bounds"));
    }

    let parse_inset = |key: &str, default_val: f64| -> Result<f64> {
        match p.get(key) {
            None => Ok(default_val),
            Some(v) if v.is_null() => Err(bad(cmd, format!("`{key}` cannot be null"))),
            Some(Value::Number(n)) => {
                if let Some(val) = n.as_f64().filter(|f| f.is_finite() && *f > 0.0 && *f <= 1_000_000.0) {
                    Ok(val)
                } else {
                    Err(bad(cmd, format!("`{key}` must be a positive finite number")))
                }
            }
            Some(Value::String(s)) => {
                if let Ok(val) = s.trim().trim_end_matches("px").parse::<f64>() {
                    if val.is_finite() && val > 0.0 && val <= 1_000_000.0 {
                        Ok(val)
                    } else {
                        Err(bad(cmd, format!("`{key}` must be a positive finite number")))
                    }
                } else {
                    Err(bad(cmd, format!("`{key}` must be a positive finite number")))
                }
            }
            Some(_) => Err(bad(cmd, format!("`{key}` must be a positive finite number"))),
        }
    };

    let left = parse_inset("left", src.width / 3.0)?;
    let right = parse_inset("right", src.width / 3.0)?;
    let top = parse_inset("top", src.height / 3.0)?;
    let bottom = parse_inset("bottom", src.height / 3.0)?;

    let insets = SliceInsets { top, right, bottom, left };

    let image_data = doc.images.get(&img.image).ok_or_else(|| bad(cmd, "image data missing"))?;
    let image_size = (image_data.width, image_data.height);

    let pairs = slice_paints(img, image_size, (src.width, src.height), insets)?;

    let parent_id = src.parent.unwrap_or_else(|| doc.first_open_page().unwrap_or(doc.pages[0]));
    let parent_node = doc.node(parent_id).ok_or_else(|| bad(cmd, "parent missing"))?;
    let src_index = parent_node.children.iter().position(|&c| c == src_id).unwrap_or(parent_node.children.len());
    let insert_index = src_index + 1;
    let is_parent_auto_layout = parent_node.layout.is_some();

    let src_name = src.name.clone();
    let src_width = src.width;
    let src_height = src.height;
    let src_transform = src.transform;
    let src_opacity = src.opacity;
    let fill_opacity = visible_fills[0].opacity;

    let doc_mut = s.doc_mut()?;
    let frame_id = doc_mut.alloc_id();
    let mut frame = Node::new(frame_id, NodeKind::Frame, format!("{src_name} / Nine Slice"), src_width, src_height);
    frame.transform = src_transform;
    frame.opacity = src_opacity;
    frame.clip = true;
    frame.fills = Vec::new();
    frame.layout = None;
    if is_parent_auto_layout {
        frame.child_layout.absolute = true;
    }
    frame.child_layout.min_w = Some(insets.left + insets.right + 1e-6);
    frame.child_layout.min_h = Some(insets.top + insets.bottom + 1e-6);
    frame.plugin_data.insert(
        NINE_SLICE_KEY.into(),
        serde_json::json!({
            "version": 1,
            "insets": [insets.top, insets.right, insets.bottom, insets.left]
        })
        .to_string(),
    );

    doc_mut.insert(frame, parent_id, Some(insert_index))?;

    for idx in 0..9 {
        let (rect, cell_paint) = &pairs[idx];
        let r = idx / 3;
        let c = idx % 3;
        let cid = doc_mut.alloc_id();
        let mut child = Node::new(cid, NodeKind::Rectangle, CELL_NAMES[idx], rect.width(), rect.height());
        child.transform = Affine::translate((rect.x0, rect.y0));
        child.fills = vec![Paint {
            kind: PaintKind::Image(cell_paint.clone()),
            opacity: fill_opacity,
            visible: true,
            blend: BlendMode::Normal,
            color_alpha: 1.0,
        }];
        child.constraints = Constraints {
            horizontal: match c {
                0 => HConstraint::Left,
                1 => HConstraint::LeftRight,
                _ => HConstraint::Right,
            },
            vertical: match r {
                0 => VConstraint::Top,
                1 => VConstraint::TopBottom,
                _ => VConstraint::Bottom,
            },
        };
        child.plugin_data.insert(NINE_SLICE_CELL_KEY.into(), idx.to_string());
        doc_mut.insert(child, frame_id, None)?;
    }

    s.set_selection(vec![frame_id]);

    let report = ToolReport { visited: 1, changed: 1, skipped: 0, protected_fields: 0, created: vec![frame_id], warnings: Vec::new() };
    serde_json::to_value(&report).map_err(|e| bad(cmd, e.to_string()))
}
