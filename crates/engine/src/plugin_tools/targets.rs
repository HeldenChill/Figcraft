use figcraft_doc::{Document, NodeId, NodeKind};
use serde_json::Value;
use std::collections::{BTreeSet, VecDeque};

use crate::params::{id_of, shown};
use crate::{EngineError, Result, Session, bad};

pub(crate) fn parse_targets(cmd: &str, s: &Session, p: &Value) -> Result<Vec<NodeId>> {
    match p.get("ids") {
        Some(Value::Array(a)) => {
            if a.len() > 10_000 {
                return Err(bad(cmd, "`ids`: array exceeds 10,000 entries limit"));
            }
            let mut ids = Vec::with_capacity(a.len());
            for v in a {
                let id = id_of(v).ok_or_else(|| bad(cmd, format!("`ids`: every entry must be a node id (got {})", shown(v))))?;
                ids.push(id);
            }
            if ids.is_empty() {
                return Err(bad(cmd, "no targets specified"));
            }
            Ok(ids)
        }
        Some(Value::Null) | None => {
            let sel = s.selection();
            if sel.is_empty() {
                return Err(bad(cmd, "no selection (pass `ids`)"));
            }
            Ok(sel)
        }
        Some(v) => Err(bad(cmd, format!("`ids` must be an array (got {})", shown(v)))),
    }
}

pub(crate) fn protected_node(doc: &Document, id: NodeId) -> bool {
    let Some(node) = doc.node(id) else {
        return true;
    };
    if !node.visible || node.locked {
        return true;
    }
    if crate::widgets::widget_of(doc, id).is_some() {
        return true;
    }
    let mut seen = BTreeSet::new();
    seen.insert(id);
    let mut cur = node.parent;
    let mut depth = 0;
    while let Some(pid) = cur {
        depth += 1;
        if depth > 10_000 || !seen.insert(pid) {
            return true;
        }
        let Some(p) = doc.node(pid) else {
            break;
        };
        if !p.visible || p.locked {
            return true;
        }
        if matches!(p.kind, NodeKind::Instance { .. }) {
            return true;
        }
        cur = p.parent;
    }
    false
}

pub(crate) fn collect_targets(doc: &Document, roots: &[NodeId], descendants: bool, limit: usize) -> Result<Vec<NodeId>> {
    for &r in roots {
        if !doc.contains(r) {
            return Err(EngineError::NoNode(r));
        }
    }

    let mut visited = BTreeSet::new();
    let mut order = Vec::new();

    for &root in roots {
        if descendants {
            let mut queue = VecDeque::new();
            queue.push_back(root);
            while let Some(nid) = queue.pop_front() {
                if visited.insert(nid) {
                    if visited.len() > limit {
                        return Err(bad("collect_targets", format!("target limit of {limit} exceeded")));
                    }
                    order.push(nid);
                    if protected_node(doc, nid) {
                        continue;
                    }
                    if let Some(n) = doc.node(nid) {
                        for &c in &n.children {
                            queue.push_back(c);
                        }
                    }
                }
            }
        } else if visited.insert(root) {
            if visited.len() > limit {
                return Err(bad("collect_targets", format!("target limit of {limit} exceeded")));
            }
            order.push(root);
        }
    }

    Ok(order)
}
