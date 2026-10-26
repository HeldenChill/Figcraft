use figcraft_color::{ImageScaleMode, PaintKind};
use serde_json::Value;

use super::ToolReport;
use super::targets::{collect_targets, parse_targets, protected_node};
use crate::{Result, Session, bad};

pub(crate) fn fit(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "image.fit";
    if crate::board::is_board(s.doc()?) {
        return Err(crate::EngineError::Disabled(cmd.into(), "Design documents only".into()));
    }

    let mode_str = match p.get("mode") {
        None | Some(Value::Null) => "fit".to_string(),
        Some(Value::String(m)) => {
            let lower = m.to_ascii_lowercase();
            if lower != "fit" && lower != "fill" {
                return Err(bad(cmd, "`mode` must be \"fit\" or \"fill\""));
            }
            lower
        }
        Some(_) => return Err(bad(cmd, "`mode` must be a string")),
    };
    let target_mode = if mode_str == "fill" { ImageScaleMode::Fill } else { ImageScaleMode::Fit };

    let descendants = match p.get("descendants") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad(cmd, "`descendants` must be a boolean")),
    };

    let raw_targets = parse_targets(cmd, s, p)?;
    let doc = s.doc()?;
    let targets = collect_targets(doc, &raw_targets, descendants, 10_000)?;

    let mut report = ToolReport::default();
    let mut changed_nodes = Vec::new();

    for id in targets {
        report.visited += 1;
        if protected_node(doc, id) {
            report.skipped += 1;
            continue;
        }
        let Some(node) = doc.node(id) else {
            report.skipped += 1;
            continue;
        };

        let mut has_visible_image = false;
        let mut needs_change = false;

        for fill in &node.fills {
            if !fill.visible {
                continue;
            }
            if let PaintKind::Image(ref img) = fill.kind {
                has_visible_image = true;
                if img.scale_mode != target_mode {
                    needs_change = true;
                }
            }
        }

        if !has_visible_image || !needs_change {
            report.skipped += 1;
            continue;
        }

        changed_nodes.push(id);
    }

    if !changed_nodes.is_empty() {
        let doc_mut = s.doc_mut()?;
        for id in changed_nodes {
            let node = doc_mut.get_mut(id)?;
            let mut modified = false;
            for fill in &mut node.fills {
                if !fill.visible {
                    continue;
                }
                if let PaintKind::Image(ref mut img) = fill.kind
                    && img.scale_mode != target_mode
                {
                    img.scale_mode = target_mode;
                    modified = true;
                }
            }
            if modified {
                node.styles.fill = None;
                report.changed += 1;
            }
        }
    }

    serde_json::to_value(&report).map_err(|e| bad(cmd, e.to_string()))
}
