use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::{ContentAlign, Document, NodeId, NodeKind, Sizing, TextAutoResize};
use figcraft_geom::Affine;
use serde_json::Value;

use super::ToolReport;
use super::targets::{collect_targets, parse_targets, protected_node};
use crate::{EngineError, Result, Session};

fn bad(msg: impl Into<String>) -> EngineError {
    crate::bad("arrange.snapToGrid", msg)
}

fn parent_id(doc: &Document, id: NodeId) -> Option<NodeId> {
    doc.node(id).and_then(|n| n.parent)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GridOptions {
    pub step: f64,
    pub descendants: bool,
    pub positions: bool,
    pub dimensions: bool,
    pub padding: bool,
    pub gaps: bool,
    pub radii: bool,
    pub bind_variables: bool,
    pub replace_bindings: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScalarEdit {
    pub node: NodeId,
    pub field: &'static str,
    pub before: f64,
    pub after: f64,
}

pub(crate) struct GridPlan {
    pub edits: Vec<ScalarEdit>,
    pub report: ToolReport,
}

pub(crate) fn snap_scalar(value: f64, step: f64) -> Result<f64> {
    if !value.is_finite() || !step.is_finite() || step <= 0.0 || value.abs() > 1_000_000.0 {
        return Err(bad("non-finite or out-of-range value or step"));
    }
    let quotient = value / step;
    if !quotient.is_finite() {
        return Err(bad("quotient overflow"));
    }
    let snapped = quotient.round() * step;
    if !snapped.is_finite() || snapped.abs() > 1_000_000.0 {
        return Err(bad("snapped value outside absolute 1,000,000"));
    }
    if snapped == 0.0 { Ok(0.0) } else { Ok(snapped) }
}

fn snap_dimension(val: f64, step: f64) -> Result<f64> {
    let s = snap_scalar(val, step)?;
    if val > 0.0 { Ok(s.max(step)) } else { Ok(0.0) }
}

fn supports_corners(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Rectangle
            | NodeKind::Frame
            | NodeKind::Component { .. }
            | NodeKind::ComponentSet { .. }
            | NodeKind::Instance { .. }
            | NodeKind::Section { .. }
    )
}

pub(crate) fn parse_options(p: &Value) -> Result<GridOptions> {
    let step = match p.get("step") {
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_f64() {
                if (v - 4.0).abs() < 1e-6 {
                    4.0
                } else if (v - 8.0).abs() < 1e-6 {
                    8.0
                } else {
                    return Err(bad("`step` must be 4 or 8"));
                }
            } else {
                return Err(bad("`step` must be 4 or 8"));
            }
        }
        Some(_) => return Err(bad("`step` must be a number")),
        None => 4.0,
    };

    let descendants = match p.get("descendants") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Null) | None => false,
        _ => return Err(bad("`descendants` must be a boolean")),
    };

    let bind_variables = match p.get("bindVariables") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Null) | None => false,
        _ => return Err(bad("`bindVariables` must be a boolean")),
    };

    let replace_bindings = match p.get("replaceBindings") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Null) | None => false,
        _ => return Err(bad("`replaceBindings` must be a boolean")),
    };

    let mut positions = false;
    let mut dimensions = false;
    let mut padding = true;
    let mut gaps = true;
    let mut radii = true;

    if let Some(props_val) = p.get("properties") {
        let obj = props_val.as_object().ok_or_else(|| bad("`properties` must be an object"))?;
        let known = ["positions", "dimensions", "padding", "gaps", "radii"];
        for (k, v) in obj {
            if !known.contains(&k.as_str()) {
                return Err(bad(format!("unknown property key `{k}`")));
            }
            let b = v.as_bool().ok_or_else(|| bad(format!("property `{k}` must be a boolean")))?;
            match k.as_str() {
                "positions" => positions = b,
                "dimensions" => dimensions = b,
                "padding" => padding = b,
                "gaps" => gaps = b,
                "radii" => radii = b,
                _ => {}
            }
        }
    }

    Ok(GridOptions { step, descendants, positions, dimensions, padding, gaps, radii, bind_variables, replace_bindings })
}

pub(crate) fn plan_edits(doc: &Document, target_ids: &[NodeId], options: &GridOptions) -> Result<GridPlan> {
    let targets = collect_targets(doc, target_ids, options.descendants, 10_000)?;

    let mut edits = Vec::new();
    let mut skipped = 0;
    let mut protected_fields = 0;

    // Check if any property toggle is active
    let any_active = options.positions || options.dimensions || options.padding || options.gaps || options.radii;
    if !any_active {
        return Ok(GridPlan {
            edits: Vec::new(),
            report: ToolReport { visited: targets.len(), changed: 0, skipped: 0, protected_fields: 0, created: Vec::new(), warnings: Vec::new() },
        });
    }

    for &id in &targets {
        if protected_node(doc, id) {
            skipped += 1;
            continue;
        }

        let Some(n) = doc.node(id) else {
            continue;
        };

        // 1. Positions
        if options.positions {
            let is_flow_child = parent_id(doc, id).and_then(|pid| doc.node(pid)).is_some_and(|pn| pn.layout.is_some() && !n.child_layout.absolute);

            if is_flow_child {
                protected_fields += 2;
            } else {
                let group = n.parent.filter(|p| doc.node(*p).is_some_and(|pn| pn.kind.bounds_follow_children()));
                let space = group.map_or(Affine::IDENTITY, |g| doc.frame_relative(g));
                let c = (space * n.transform).as_coeffs();
                let (cur_x, cur_y) = (c[4], c[5]);

                let snap_x = snap_scalar(cur_x, options.step)?;
                let snap_y = snap_scalar(cur_y, options.step)?;

                if options.bind_variables || (cur_x - snap_x).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "x", before: cur_x, after: snap_x });
                }
                if options.bind_variables || (cur_y - snap_y).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "y", before: cur_y, after: snap_y });
                }
            }
        }

        // 2. Dimensions
        if options.dimensions {
            let is_w_derived = match n.child_layout.sizing_h {
                Sizing::Hug | Sizing::Fill => true,
                Sizing::Fixed => {
                    if let NodeKind::Text(t) = &n.kind {
                        t.auto_resize == TextAutoResize::WidthAndHeight
                    } else {
                        false
                    }
                }
            };

            let is_h_derived = match n.child_layout.sizing_v {
                Sizing::Hug | Sizing::Fill => true,
                Sizing::Fixed => {
                    if let NodeKind::Text(t) = &n.kind {
                        t.auto_resize == TextAutoResize::WidthAndHeight || t.auto_resize == TextAutoResize::Height
                    } else {
                        false
                    }
                }
            };

            let is_w_bound = n.bound.contains_key("width") || n.bound.contains_key("w");
            let is_h_bound = n.bound.contains_key("height") || n.bound.contains_key("h");

            // Width
            if is_w_derived || (is_w_bound && !options.replace_bindings) {
                protected_fields += 1;
            } else {
                let snap_w = snap_dimension(n.width, options.step)?;
                let below_min = n.child_layout.min_w.is_some_and(|m| snap_w < m);
                let above_max = n.child_layout.max_w.is_some_and(|m| snap_w > m);
                let below_nine_slice = crate::plugin_tools::nine_slice::minimum_size(n).is_some_and(|(nw, _)| snap_w < nw);

                if below_min || above_max || below_nine_slice {
                    protected_fields += 1;
                } else if options.bind_variables || (is_w_bound && options.replace_bindings) || (n.width - snap_w).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "width", before: n.width, after: snap_w });
                }
            }

            // Height
            if is_h_derived || (is_h_bound && !options.replace_bindings) {
                protected_fields += 1;
            } else {
                let snap_h = snap_dimension(n.height, options.step)?;
                let below_min = n.child_layout.min_h.is_some_and(|m| snap_h < m);
                let above_max = n.child_layout.max_h.is_some_and(|m| snap_h > m);
                let below_nine_slice = crate::plugin_tools::nine_slice::minimum_size(n).is_some_and(|(_, nh)| snap_h < nh);

                if below_min || above_max || below_nine_slice {
                    protected_fields += 1;
                } else if options.bind_variables || (is_h_bound && options.replace_bindings) || (n.height - snap_h).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "height", before: n.height, after: snap_h });
                }
            }
        }

        // 3. Padding
        if options.padding
            && let Some(layout) = &n.layout
        {
            let bound_padding = n.bound.contains_key("padding");
            let bound_pv = n.bound.contains_key("paddingVertical");
            let bound_ph = n.bound.contains_key("paddingHorizontal");

            let bound_pt = bound_padding || bound_pv || n.bound.contains_key("paddingTop");
            let bound_pr = bound_padding || bound_ph || n.bound.contains_key("paddingRight");
            let bound_pb = bound_padding || bound_pv || n.bound.contains_key("paddingBottom");
            let bound_pl = bound_padding || bound_ph || n.bound.contains_key("paddingLeft");

            let fields = [
                ("paddingTop", layout.padding[0], bound_pt),
                ("paddingRight", layout.padding[1], bound_pr),
                ("paddingBottom", layout.padding[2], bound_pb),
                ("paddingLeft", layout.padding[3], bound_pl),
            ];

            for (field, val, is_bound) in fields {
                if is_bound && !options.replace_bindings {
                    protected_fields += 1;
                } else {
                    let snap_p = snap_scalar(val, options.step)?.max(0.0);
                    if options.bind_variables || (is_bound && options.replace_bindings) || (val - snap_p).abs() > 1e-6 {
                        edits.push(ScalarEdit { node: id, field, before: val, after: snap_p });
                    }
                }
            }
        }

        // 4. Gaps
        if options.gaps
            && let Some(layout) = &n.layout
        {
            // Gap
            let is_gap_bound = n.bound.contains_key("gap") || n.bound.contains_key("columnGap");
            if layout.main_align.is_auto_gap() || (is_gap_bound && !options.replace_bindings) {
                protected_fields += 1;
            } else {
                let snap_g = snap_scalar(layout.gap, options.step)?;
                if options.bind_variables || (is_gap_bound && options.replace_bindings) || (layout.gap - snap_g).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "gap", before: layout.gap, after: snap_g });
                }
            }

            // Cross gap
            let is_cross_bound = n.bound.contains_key("crossGap") || n.bound.contains_key("rowGap");
            if (layout.wrap && layout.content_align == ContentAlign::SpaceBetween) || (is_cross_bound && !options.replace_bindings) {
                protected_fields += 1;
            } else {
                let snap_cg = snap_scalar(layout.cross_gap, options.step)?;
                if options.bind_variables || (is_cross_bound && options.replace_bindings) || (layout.cross_gap - snap_cg).abs() > 1e-6 {
                    edits.push(ScalarEdit { node: id, field: "crossGap", before: layout.cross_gap, after: snap_cg });
                }
            }
        }

        // 5. Radii
        if options.radii && supports_corners(&n.kind) {
            let bound_cr = n.bound.contains_key("cornerRadius");
            let bound_tl = bound_cr || n.bound.contains_key("topLeftRadius");
            let bound_tr = bound_cr || n.bound.contains_key("topRightRadius");
            let bound_br = bound_cr || n.bound.contains_key("bottomRightRadius");
            let bound_bl = bound_cr || n.bound.contains_key("bottomLeftRadius");

            let corner_fields = [
                ("topLeftRadius", n.corners.radii[0], bound_tl),
                ("topRightRadius", n.corners.radii[1], bound_tr),
                ("bottomRightRadius", n.corners.radii[2], bound_br),
                ("bottomLeftRadius", n.corners.radii[3], bound_bl),
            ];

            for (field, r, is_bound) in corner_fields {
                if is_bound && !options.replace_bindings {
                    protected_fields += 1;
                } else {
                    let snap_r = snap_scalar(r, options.step)?.max(0.0);
                    if options.bind_variables || (is_bound && options.replace_bindings) || (r - snap_r).abs() > 1e-6 {
                        edits.push(ScalarEdit { node: id, field, before: r, after: snap_r });
                    }
                }
            }
        }
    }

    Ok(GridPlan {
        edits,
        report: ToolReport { visited: targets.len(), changed: 0, skipped, protected_fields, created: Vec::new(), warnings: Vec::new() },
    })
}

pub(crate) fn snap_to_grid(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "arrange.snapToGrid";
    if crate::board::is_board(s.doc()?) {
        return Err(EngineError::Disabled(cmd.into(), "Design documents only".into()));
    }

    let options = parse_options(p)?;
    let target_ids = parse_targets(cmd, s, p)?;

    let plan = plan_edits(s.doc()?, &target_ids, &options)?;

    // Calculate depth of nodes so ancestors apply before descendants
    let mut node_depth = BTreeMap::new();
    let doc = s.doc()?;
    for e in &plan.edits {
        node_depth.entry(e.node).or_insert_with(|| {
            let mut d = 0;
            let mut cur = parent_id(doc, e.node);
            while let Some(pid) = cur {
                d += 1;
                cur = parent_id(doc, pid);
            }
            d
        });
    }

    let mut nodes_in_order: Vec<NodeId> = node_depth.keys().copied().collect();
    nodes_in_order.sort_by_key(|nid| node_depth.get(nid).copied().unwrap_or(0));

    let mut literal_changed_nodes = BTreeSet::new();
    let mut binding_detached_nodes = BTreeSet::new();

    for &nid in &nodes_in_order {
        let node_edits: Vec<&ScalarEdit> = plan.edits.iter().filter(|e| e.node == nid).collect();
        if node_edits.is_empty() {
            continue;
        }

        let mut edit_x = None;
        let mut edit_y = None;
        let mut edit_w = None;
        let mut edit_h = None;
        let mut edit_pt = None;
        let mut edit_pr = None;
        let mut edit_pb = None;
        let mut edit_pl = None;
        let mut edit_gap = None;
        let mut edit_cg = None;
        let mut edit_tl = None;
        let mut edit_tr = None;
        let mut edit_br = None;
        let mut edit_bl = None;

        for e in &node_edits {
            if (e.before - e.after).abs() > 1e-6 {
                literal_changed_nodes.insert(nid);
            }
            match e.field {
                "x" => edit_x = Some(e.after),
                "y" => edit_y = Some(e.after),
                "width" => edit_w = Some(e.after),
                "height" => edit_h = Some(e.after),
                "paddingTop" => edit_pt = Some(e.after),
                "paddingRight" => edit_pr = Some(e.after),
                "paddingBottom" => edit_pb = Some(e.after),
                "paddingLeft" => edit_pl = Some(e.after),
                "gap" => edit_gap = Some(e.after),
                "crossGap" => edit_cg = Some(e.after),
                "topLeftRadius" => edit_tl = Some(e.after),
                "topRightRadius" => edit_tr = Some(e.after),
                "bottomRightRadius" => edit_br = Some(e.after),
                "bottomLeftRadius" => edit_bl = Some(e.after),
                _ => {}
            }
        }

        // Apply position
        if edit_x.is_some() || edit_y.is_some() {
            let doc_ref = s.doc()?;
            let n = doc_ref.get(nid)?;
            let group = n.parent.filter(|p| doc_ref.node(*p).is_some_and(|pn| pn.kind.bounds_follow_children()));
            let space = group.map_or(Affine::IDENTITY, |g| doc_ref.frame_relative(g));
            let c = (space * n.transform).as_coeffs();
            let tx = edit_x.unwrap_or(c[4]);
            let ty = edit_y.unwrap_or(c[5]);

            let doc_mut = s.doc_mut()?;
            doc_mut.get_mut(nid)?.transform = space.inverse() * Affine::new([c[0], c[1], c[2], c[3], tx, ty]);
        }

        // Apply dimensions
        if edit_w.is_some() || edit_h.is_some() {
            let doc_ref = s.doc()?;
            let n = doc_ref.get(nid)?;
            let w = edit_w.unwrap_or(n.width);
            let h = edit_h.unwrap_or(n.height);
            if (w - n.width).abs() > 1e-6 || (h - n.height).abs() > 1e-6 {
                let doc_mut = s.doc_mut()?;
                crate::cmd::props::resize_node(doc_mut, nid, w, h)?;
            }
        }

        // Apply padding & gaps
        if edit_pt.is_some() || edit_pr.is_some() || edit_pb.is_some() || edit_pl.is_some() || edit_gap.is_some() || edit_cg.is_some() {
            let doc_mut = s.doc_mut()?;
            if let Some(ref mut layout) = doc_mut.get_mut(nid)?.layout {
                if let Some(v) = edit_pt {
                    layout.padding[0] = v;
                }
                if let Some(v) = edit_pr {
                    layout.padding[1] = v;
                }
                if let Some(v) = edit_pb {
                    layout.padding[2] = v;
                }
                if let Some(v) = edit_pl {
                    layout.padding[3] = v;
                }
                if let Some(v) = edit_gap {
                    layout.gap = v;
                }
                if let Some(v) = edit_cg {
                    layout.cross_gap = v;
                }
            }
        }

        // Apply radii
        if edit_tl.is_some() || edit_tr.is_some() || edit_br.is_some() || edit_bl.is_some() {
            let doc_mut = s.doc_mut()?;
            let n = doc_mut.get_mut(nid)?;
            if let Some(v) = edit_tl {
                n.corners.radii[0] = v;
            }
            if let Some(v) = edit_tr {
                n.corners.radii[1] = v;
            }
            if let Some(v) = edit_br {
                n.corners.radii[2] = v;
            }
            if let Some(v) = edit_bl {
                n.corners.radii[3] = v;
            }
        }

        // Detach replaced bindings if requested
        if options.replace_bindings {
            let doc_mut = s.doc_mut()?;
            let n = doc_mut.get_mut(nid)?;
            let mut detached = false;

            if edit_w.is_some() {
                detached |= n.bound.remove("width").is_some();
                detached |= n.bound.remove("w").is_some();
            }
            if edit_h.is_some() {
                detached |= n.bound.remove("height").is_some();
                detached |= n.bound.remove("h").is_some();
            }

            // Only remove aggregate padding bindings if all covered sides were edited
            if edit_pt.is_some() && edit_pr.is_some() && edit_pb.is_some() && edit_pl.is_some() {
                detached |= n.bound.remove("padding").is_some();
            }
            if edit_pt.is_some() && edit_pb.is_some() {
                detached |= n.bound.remove("paddingVertical").is_some();
            }
            if edit_pr.is_some() && edit_pl.is_some() {
                detached |= n.bound.remove("paddingHorizontal").is_some();
            }
            if edit_pt.is_some() {
                detached |= n.bound.remove("paddingTop").is_some();
            }
            if edit_pr.is_some() {
                detached |= n.bound.remove("paddingRight").is_some();
            }
            if edit_pb.is_some() {
                detached |= n.bound.remove("paddingBottom").is_some();
            }
            if edit_pl.is_some() {
                detached |= n.bound.remove("paddingLeft").is_some();
            }

            if edit_gap.is_some() {
                detached |= n.bound.remove("gap").is_some();
                detached |= n.bound.remove("columnGap").is_some();
            }
            if edit_cg.is_some() {
                detached |= n.bound.remove("crossGap").is_some();
                detached |= n.bound.remove("rowGap").is_some();
            }

            // Only remove aggregate cornerRadius if all 4 corners were edited
            if edit_tl.is_some() && edit_tr.is_some() && edit_br.is_some() && edit_bl.is_some() {
                detached |= n.bound.remove("cornerRadius").is_some();
            }
            if edit_tl.is_some() {
                detached |= n.bound.remove("topLeftRadius").is_some();
            }
            if edit_tr.is_some() {
                detached |= n.bound.remove("topRightRadius").is_some();
            }
            if edit_br.is_some() {
                detached |= n.bound.remove("bottomRightRadius").is_some();
            }
            if edit_bl.is_some() {
                detached |= n.bound.remove("bottomLeftRadius").is_some();
            }

            if detached {
                binding_detached_nodes.insert(nid);
            }
        }
    }

    let bound_nodes =
        if options.bind_variables { super::grid_variables::bind_planned_values(s, &plan.edits, options.step as u8)? } else { BTreeSet::new() };

    let mut total_changed = literal_changed_nodes;
    for dn in binding_detached_nodes {
        total_changed.insert(dn);
    }
    for bn in bound_nodes {
        total_changed.insert(bn);
    }

    let targets = collect_targets(s.doc()?, &target_ids, options.descendants, 10_000)?;
    let report = ToolReport {
        visited: targets.len(),
        changed: total_changed.len(),
        skipped: plan.report.skipped,
        protected_fields: plan.report.protected_fields,
        created: Vec::new(),
        warnings: Vec::new(),
    };

    serde_json::to_value(&report).map_err(|e| bad(e.to_string()))
}
