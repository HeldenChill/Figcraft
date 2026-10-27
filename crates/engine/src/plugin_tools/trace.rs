use figcraft_color::{BlendMode, Color, ImageScaleMode, Paint, PaintKind};
use figcraft_doc::{Corners, MaskType, Node, NodeId, NodeKind, VectorNetwork, WindingRule};
use figcraft_geom::Affine;
use serde_json::Value;

use super::ToolReport;
use super::contours::{ContourBudget, trace_label};
use super::quantize::{LabelImage, monochrome, quantize};
use super::targets::{parse_targets, protected_node};
use crate::{EngineError, Result, Session};

fn bad(msg: impl Into<String>) -> EngineError {
    crate::bad("image.trace", msg)
}

fn parse_u8(p: &Value, key: &str, default: u8, min: u8, max: u8) -> Result<u8> {
    match p.get(key) {
        None => Ok(default),
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_u64().filter(|&v| (min as u64..=max as u64).contains(&v)) {
                return Ok(v as u8);
            }
            Err(bad(format!("`{key}` must be an integer between {min} and {max}")))
        }
        Some(_) => Err(bad(format!("`{key}` must be an integer between {min} and {max}"))),
    }
}

fn parse_u32(p: &Value, key: &str, default: u32, min: u32, max: u32) -> Result<u32> {
    match p.get(key) {
        None => Ok(default),
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_u64().filter(|&v| (min as u64..=max as u64).contains(&v)) {
                return Ok(v as u32);
            }
            Err(bad(format!("`{key}` must be an integer between {min} and {max}")))
        }
        Some(_) => Err(bad(format!("`{key}` must be an integer between {min} and {max}"))),
    }
}

struct SourceTrace {
    src_id: NodeId,
    parent_id: NodeId,
    parent_is_auto_layout: bool,
    src_name: String,
    src_width: f64,
    src_height: f64,
    src_transform: Affine,
    src_opacity: f32,
    resolved_paint: Paint,
    vector_layers: Vec<([u8; 4], VectorNetwork)>,
    mask_network: Option<VectorNetwork>,
}

pub(crate) fn trace(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "image.trace";
    if crate::board::is_board(s.doc()?) {
        return Err(EngineError::Disabled(cmd.into(), "Design documents only".into()));
    }

    let mode_str = match p.get("mode") {
        None => "monochrome",
        Some(Value::String(s)) => {
            if s == "monochrome" || s == "color" {
                s.as_str()
            } else {
                return Err(bad("`mode` must be 'monochrome' or 'color'"));
            }
        }
        Some(_) => return Err(bad("`mode` must be a string")),
    };

    let output_str = match p.get("output") {
        None => "vectors",
        Some(Value::String(s)) => {
            if s == "vectors" || s == "mask" {
                s.as_str()
            } else {
                return Err(bad("`output` must be 'vectors' or 'mask'"));
            }
        }
        Some(_) => return Err(bad("`output` must be a string")),
    };

    let threshold = parse_u8(p, "threshold", 128, 0, 255)?;
    let invert = match p.get("invert") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad("`invert` must be a boolean")),
    };
    let colors = parse_u8(p, "colors", 8, 2, 16)?;
    let alpha_cutoff = parse_u8(p, "alphaCutoff", 16, 1, 255)?;
    let resolution = parse_u32(p, "resolution", 256, 16, 512)?;

    let targets = parse_targets(cmd, s, p)?;
    if targets.is_empty() {
        return Err(bad("no layers selected to trace"));
    }
    if targets.len() > 16 {
        return Err(bad("maximum 16 layers may be traced per call"));
    }

    let doc = s.doc()?;
    let mut eligible_traces = Vec::new();
    let mut skipped = 0;
    let mut warnings = Vec::new();

    let mut budget = ContourBudget { vertices_left: 65536, loops_left: 4096 };

    for &src_id in &targets {
        if !doc.contains(src_id) {
            return Err(EngineError::NoNode(src_id));
        }
        if protected_node(doc, src_id) {
            skipped += 1;
            continue;
        }

        let src = doc.node(src_id).ok_or(EngineError::NoNode(src_id))?;
        if !matches!(src.kind, NodeKind::Rectangle) {
            return Err(bad("source layer must be a rectangle"));
        }
        if src.is_mask {
            return Err(bad("mask nodes not supported as trace source"));
        }
        if !src.children.is_empty() {
            return Err(bad("source layer must not have children"));
        }

        let visible_fills: Vec<&Paint> = src.fills.iter().filter(|f| f.visible).collect();
        if visible_fills.len() != 1 {
            return Err(bad("source must have exactly one visible fill"));
        }
        let fill = visible_fills[0];
        let img = match &fill.kind {
            PaintKind::Image(img) => img,
            _ => return Err(bad("source fill must be an image")),
        };
        if img.scale_mode == ImageScaleMode::Tile {
            return Err(bad("tile scale mode not supported for tracing"));
        }
        if img.scale_mode != ImageScaleMode::Fill && img.scale_mode != ImageScaleMode::Fit && img.scale_mode != ImageScaleMode::Crop {
            return Err(bad("unsupported image scale mode"));
        }
        if src.strokes.iter().any(|st| st.visible) {
            return Err(bad("strokes not supported on trace source"));
        }
        if !src.effects.is_empty() {
            return Err(bad("effects not supported on trace source"));
        }
        if src.corners != Corners::default() {
            return Err(bad("corner radii not supported on trace source"));
        }
        if src.blend != BlendMode::Normal && src.blend != BlendMode::PassThrough {
            return Err(bad("blend modes not supported on trace source"));
        }
        if fill.blend != BlendMode::Normal {
            return Err(bad("fill blend modes not supported on trace source"));
        }
        if !src.transform.is_finite() || src.transform.determinant().abs() <= 1e-9 {
            return Err(bad("source transform must be finite and nonsingular"));
        }
        if src.width <= 0.0 || src.height <= 0.0 || src.width > 1_000_000.0 || src.height > 1_000_000.0 {
            return Err(bad("source layer dimensions out of bounds"));
        }

        // Sample image paint to local raster
        let raster = figcraft_render::image_tools::raster_image_paint(doc, fill, (src.width, src.height), resolution)
            .map_err(|e| bad(format!("failed to rasterize image for tracing: {e}")))?;

        let label_image: LabelImage =
            if mode_str == "monochrome" { monochrome(&raster, threshold, invert, alpha_cutoff)? } else { quantize(&raster, colors, alpha_cutoff)? };

        if label_image.palette.is_empty() {
            skipped += 1;
            warnings.push(format!("Layer `{}` produced no trace contours (empty or below alpha cutoff)", src.name));
            continue;
        }

        let sx = src.width / f64::from(raster.width);
        let sy = src.height / f64::from(raster.height);
        let scale_xf = Affine::scale_non_uniform(sx, sy);

        let mut vector_layers = Vec::new();
        let mut compound_mask_path = figcraft_geom::BezPath::new();

        for (idx, color) in label_image.palette.iter().enumerate() {
            let path = trace_label(&label_image, idx as u8, &mut budget)?;
            if path.elements().is_empty() {
                continue;
            }

            let mut scaled_path = path.clone();
            scaled_path.apply_affine(scale_xf);

            let mut net = VectorNetwork::from_path(&scaled_path);
            for r in &mut net.regions {
                r.winding = WindingRule::EvenOdd;
            }

            if output_str == "mask" {
                compound_mask_path.extend(scaled_path);
            }
            vector_layers.push((*color, net));
        }

        if vector_layers.is_empty() {
            skipped += 1;
            warnings.push(format!("Layer `{}` produced no trace contours", src.name));
            continue;
        }

        let mask_network = if output_str == "mask" {
            let mut m_net = VectorNetwork::from_path(&compound_mask_path);
            for r in &mut m_net.regions {
                r.winding = WindingRule::EvenOdd;
            }
            Some(m_net)
        } else {
            None
        };

        let parent_id = src.parent.unwrap_or_else(|| doc.first_open_page().unwrap_or(doc.pages[0]));
        let parent_node = doc.node(parent_id).ok_or_else(|| bad("parent missing"))?;
        let parent_is_auto_layout = parent_node.layout.is_some();

        eligible_traces.push(SourceTrace {
            src_id,
            parent_id,
            parent_is_auto_layout,
            src_name: src.name.clone(),
            src_width: src.width,
            src_height: src.height,
            src_transform: src.transform,
            src_opacity: src.opacity,
            resolved_paint: fill.clone(),
            vector_layers,
            mask_network,
        });
    }

    if eligible_traces.is_empty() {
        let report = ToolReport { visited: targets.len(), changed: 0, skipped, protected_fields: 0, created: Vec::new(), warnings };
        return serde_json::to_value(&report).map_err(|e| bad(e.to_string()));
    }

    // Insert output nodes into document
    let mut created_roots = Vec::new();
    let doc_mut = s.doc_mut()?;

    for t in eligible_traces {
        // Earlier outputs may have shifted this source's sibling index.
        let insert_idx = doc_mut
            .node(t.parent_id)
            .and_then(|parent| parent.children.iter().position(|&id| id == t.src_id))
            .ok_or_else(|| bad("source missing from parent"))?
            + 1;
        if output_str == "vectors" {
            let group_id = doc_mut.alloc_id();
            let mut group_node = Node::new(group_id, NodeKind::Group, format!("{} / Trace", t.src_name), t.src_width, t.src_height);
            group_node.transform = t.src_transform;
            group_node.opacity = t.src_opacity;
            if t.parent_is_auto_layout {
                group_node.child_layout.absolute = true;
            }

            doc_mut.insert(group_node, t.parent_id, Some(insert_idx))?;

            for (idx, (rgba, net)) in t.vector_layers.into_iter().enumerate() {
                let color_name = if mode_str == "monochrome" { "Silhouette".to_string() } else { format!("Color {idx}") };
                let cid = doc_mut.alloc_id();
                let mut v_node = Node::new(cid, NodeKind::Vector { network: net }, color_name, t.src_width, t.src_height);
                v_node.fills = vec![Paint::solid(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]))];
                doc_mut.insert(v_node, group_id, None)?;
            }

            created_roots.push(group_id);
        } else {
            // output: mask
            let frame_id = doc_mut.alloc_id();
            let mut frame_node = Node::new(frame_id, NodeKind::Frame, format!("{} / Trace mask", t.src_name), t.src_width, t.src_height);
            frame_node.transform = t.src_transform;
            frame_node.clip = true;
            frame_node.opacity = t.src_opacity;
            frame_node.fills = Vec::new();
            if t.parent_is_auto_layout {
                frame_node.child_layout.absolute = true;
            }

            doc_mut.insert(frame_node, t.parent_id, Some(insert_idx))?;

            if let Some(mask_net) = t.mask_network {
                // Bottom child: vector mask
                let mask_id = doc_mut.alloc_id();
                let mut mask_node =
                    Node::new(mask_id, NodeKind::Vector { network: mask_net }, format!("{} / Mask", t.src_name), t.src_width, t.src_height);
                mask_node.is_mask = true;
                mask_node.mask_type = MaskType::Vector;
                mask_node.fills = vec![Paint::solid(Color::WHITE)];
                doc_mut.insert(mask_node, frame_id, None)?;
            }

            // Top child: cloned image rectangle
            let clone_id = doc_mut.alloc_id();
            let mut img_clone = Node::new(clone_id, NodeKind::Rectangle, t.src_name, t.src_width, t.src_height);
            img_clone.transform = Affine::IDENTITY;
            img_clone.fills = vec![t.resolved_paint];
            img_clone.opacity = 1.0;
            img_clone.blend = BlendMode::Normal;
            img_clone.is_mask = false;
            doc_mut.insert(img_clone, frame_id, None)?;

            created_roots.push(frame_id);
        }
    }

    s.set_selection(created_roots.clone());

    let report = ToolReport { visited: targets.len(), changed: created_roots.len(), skipped, protected_fields: 0, created: created_roots, warnings };
    serde_json::to_value(&report).map_err(|e| bad(e.to_string()))
}
