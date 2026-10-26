use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::{Collection, Mode, NodeId, Variable, VariableType, VariableValue};
use serde_json::json;

use super::grid::ScalarEdit;
use crate::{EngineError, Result, Session};

fn bad(msg: impl Into<String>) -> EngineError {
    crate::bad("arrange.snapToGrid", msg)
}

fn format_value(v: f64) -> String {
    if v.fract() == 0.0 { format!("{:.0}", v) } else { format!("{v}") }
}

fn free_token_name(occupied: &BTreeSet<String>, base: &str) -> String {
    if !occupied.contains(base) {
        return base.to_string();
    }
    for k in 2..10_000 {
        let name = format!("{base} {k}");
        if !occupied.contains(&name) {
            return name;
        }
    }
    format!("{base} 9999")
}

pub(crate) fn bind_planned_values(s: &mut Session, edits: &[ScalarEdit], step: u8) -> Result<BTreeSet<NodeId>> {
    if step != 4 && step != 8 {
        return Err(bad("step must be 4 or 8"));
    }

    let bindable_fields = [
        "width",
        "height",
        "paddingTop",
        "paddingRight",
        "paddingBottom",
        "paddingLeft",
        "gap",
        "crossGap",
        "topLeftRadius",
        "topRightRadius",
        "bottomRightRadius",
        "bottomLeftRadius",
    ];

    let eligible_edits: Vec<&ScalarEdit> = edits.iter().filter(|e| bindable_fields.contains(&e.field)).collect();

    if eligible_edits.is_empty() {
        return Ok(BTreeSet::new());
    }

    let doc = s.doc_mut()?;
    let first_page_id = doc.first_open_page().unwrap_or(doc.pages[0]);
    let page_node = doc.node(first_page_id).ok_or_else(|| bad("first page missing"))?;

    // Check ownership anchor
    let owned_coll_id = page_node
        .plugin_data
        .get("figcraft.tools.gridCleanup")
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|v| v.get("collection").and_then(|c| c.as_str().map(str::to_string)))
        .filter(|id| doc.variables.collections.iter().any(|c| c.id == *id && c.extends.is_none()));

    let coll_id = match owned_coll_id {
        Some(id) => id,
        None => {
            let base_name = "FigCraft / Grid Cleanup";
            let coll_name = crate::cmd::variables::free_collection_name(doc, base_name);
            let cid = doc.new_collection_id();
            let mid = doc.new_mode_id(&cid).unwrap_or_else(|| format!("{cid}:mode:1"));

            let coll = Collection { id: cid.clone(), name: coll_name, modes: vec![Mode { id: mid, name: "Default".to_string() }], extends: None };
            doc.variables.collections.push(coll);

            // Record ownership on first page
            let page_mut = doc.get_mut(first_page_id)?;
            page_mut.plugin_data.insert("figcraft.tools.gridCleanup".to_string(), json!({ "version": 1, "collection": cid }).to_string());

            cid
        }
    };

    let coll = doc.variables.collections.iter().find(|c| c.id == coll_id).ok_or_else(|| bad("owned collection not found"))?.clone();

    if coll.modes.len() > 64 {
        return Err(bad("owned collection exceeds 64 modes"));
    }

    let coll_var_count = doc.variables.variables.iter().filter(|v| v.collection == coll_id).count();

    // Index reusable tokens in this collection
    let mut reusable_tokens: BTreeMap<u64, String> = BTreeMap::new();
    let mut occupied_names: BTreeSet<String> = BTreeSet::new();

    for v in &doc.variables.variables {
        if v.collection != coll_id {
            continue;
        }
        occupied_names.insert(v.name.clone());

        if v.kind != VariableType::Number || v.extends.is_some() {
            continue;
        }

        // Check if description has our exact marker prefix
        let prefix = format!("figcraft.tools.grid-cleanup/v1;step={step};value=");
        if let Some(val_str) = v.description.strip_prefix(&prefix)
            && let Ok(expected_val) = val_str.parse::<f64>()
        {
            // Must match expected_val in every mode
            let matches_all_modes = coll.modes.iter().all(|m| match v.values.get(&m.id) {
                Some(VariableValue::Number(n)) => (n - expected_val).abs() < 1e-6,
                _ => false,
            });
            if matches_all_modes {
                reusable_tokens.insert(expected_val.to_bits(), v.id.clone());
            }
        }
    }

    let mut newly_created_count = 0;
    let mut changed_nodes = BTreeSet::new();

    for edit in eligible_edits {
        let val = edit.after;
        let bits = val.to_bits();

        let var_id = if let Some(vid) = reusable_tokens.get(&bits) {
            vid.clone()
        } else {
            newly_created_count += 1;
            if newly_created_count > 512 {
                return Err(bad("token creation limit of 512 per call exceeded"));
            }
            if coll_var_count + newly_created_count > 4096 {
                return Err(bad("collection variable limit of 4096 exceeded"));
            }

            let val_str = format_value(val);
            let base_name = format!("step-{step}/value-{val_str}");
            let token_name = free_token_name(&occupied_names, &base_name);
            occupied_names.insert(token_name.clone());

            let marker = format!("figcraft.tools.grid-cleanup/v1;step={step};value={val_str}");
            let vid = doc.new_variable_id();

            let mut values = BTreeMap::new();
            for m in &coll.modes {
                values.insert(m.id.clone(), VariableValue::Number(val));
            }

            let var = Variable {
                id: vid.clone(),
                name: token_name,
                collection: coll_id.clone(),
                kind: VariableType::Number,
                values,
                description: marker,
                scopes: None,
                hidden_from_publishing: false,
                extends: None,
                overrides: BTreeSet::new(),
                code_syntax: BTreeMap::new(),
            };

            doc.variables.variables.push(var);
            reusable_tokens.insert(bits, vid.clone());
            vid
        };

        let node = doc.get_mut(edit.node)?;
        if node.bound.get(edit.field) != Some(&var_id) {
            node.bound.insert(edit.field.to_string(), var_id);
            changed_nodes.insert(edit.node);
        }
    }

    Ok(changed_nodes)
}
