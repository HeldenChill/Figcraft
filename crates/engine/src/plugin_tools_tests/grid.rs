use figcraft_doc::{AutoLayout, ContentAlign, Corners, MainAlign, Sizing};
use serde_json::json;

use super::helpers::snapshot;
use crate::Session;
use crate::plugin_tools::grid::snap_scalar;

#[test]
fn grid_rounds_requested_literals_and_undoes_once() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": -6, "y": 6, "width": 19, "height": 27})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    let before = snapshot(&s);
    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "positions": true,
                    "dimensions": true,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();
    assert_eq!(out["changed"], 1);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!((n.width, n.height), (20.0, 28.0));
    let c = s.doc().unwrap().frame_relative(id).as_coeffs();
    assert_eq!((c[4], c[5]), (-8.0, 8.0));
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before);
}

#[test]
fn grid_rounding_edges() {
    assert_eq!(snap_scalar(13.0, 4.0).unwrap(), 12.0);
    assert_eq!(snap_scalar(14.0, 4.0).unwrap(), 16.0);
    assert_eq!(snap_scalar(-14.0, 4.0).unwrap(), -16.0);
    assert_eq!(snap_scalar(0.0, 4.0).unwrap(), 0.0);

    assert_eq!(snap_scalar(19.0, 8.0).unwrap(), 16.0);
    assert_eq!(snap_scalar(20.0, 8.0).unwrap(), 24.0);

    assert!(snap_scalar(10.0, 0.0).is_err());
    assert!(snap_scalar(10.0, -4.0).is_err());
    assert!(snap_scalar(f64::NAN, 4.0).is_err());
    assert!(snap_scalar(f64::INFINITY, 4.0).is_err());
    assert!(snap_scalar(1_000_001.0, 4.0).is_err());
}

#[test]
fn grid_defaults_leave_geometry() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 13, "y": 15, "width": 101, "height": 103})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    // Give node corners and auto layout
    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.corners = Corners { radii: [7.0, 11.0, 13.0, 15.0], smoothing: 0.0 };
        n.layout = Some(AutoLayout { padding: [9.0, 13.0, 17.0, 21.0], gap: 7.0, cross_gap: 11.0, ..Default::default() });
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 1);
    let n = s.doc().unwrap().node(id).unwrap();
    // Default: positions: false, dimensions: false -> width, height, x, y untouched
    assert_eq!((n.width, n.height), (101.0, 103.0));
    let c = s.doc().unwrap().frame_relative(id).as_coeffs();
    assert_eq!((c[4], c[5]), (13.0, 15.0));

    // Padding, gaps, radii snapped to step 4
    let layout = n.layout.as_ref().unwrap();
    assert_eq!(layout.padding, [8.0, 12.0, 16.0, 20.0]);
    assert_eq!(layout.gap, 8.0);
    assert_eq!(layout.cross_gap, 12.0);
    assert_eq!(n.corners.radii, [8.0, 12.0, 12.0, 16.0]);
}

#[test]
fn grid_keeps_hug_fill_and_flow_positions() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 13, "y": 15, "width": 21, "height": 33})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    // Put into an auto-layout parent frame
    let p_res = s.execute("frame.create", &json!({"width": 200, "height": 200})).unwrap();
    let pid = crate::params::id_of(&p_res["id"]).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(pid).unwrap().layout = Some(AutoLayout::default());
        doc.reparent(id, pid, None).unwrap();
        let n = doc.get_mut(id).unwrap();
        n.child_layout.sizing_h = Sizing::Hug;
        n.child_layout.sizing_v = Sizing::Fill;
        n.child_layout.absolute = false;
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "positions": true,
                    "dimensions": true,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();

    // Both positions (flow child) and both dimensions (Hug/Fill) are protected
    assert_eq!(out["changed"], 0);
    assert_eq!(out["protectedFields"], 4);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!(n.child_layout.sizing_h, Sizing::Hug);
    assert_eq!(n.child_layout.sizing_v, Sizing::Fill);
    assert_eq!((n.width, n.height), (21.0, 33.0));
}

#[test]
fn grid_auto_gaps_are_protected() {
    let mut s = Session::new();
    let f_res = s.execute("frame.create", &json!({"width": 200, "height": 200})).unwrap();
    let fid = crate::params::id_of(&f_res["id"]).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        let frame = doc.get_mut(fid).unwrap();
        frame.layout = Some(AutoLayout {
            gap: 15.0,
            cross_gap: 15.0,
            main_align: MainAlign::SpaceBetween,
            wrap: true,
            content_align: ContentAlign::SpaceBetween,
            ..Default::default()
        });
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [fid],
                "step": 4,
                "properties": {
                    "positions": false,
                    "dimensions": false,
                    "padding": false,
                    "gaps": true,
                    "radii": false
                }
            }),
        )
        .unwrap();

    // Auto gaps protected
    assert_eq!(out["changed"], 0);
    assert_eq!(out["protectedFields"], 2);
    let layout = s.doc().unwrap().node(fid).unwrap().layout.as_ref().unwrap();
    assert_eq!(layout.gap, 15.0);
    assert_eq!(layout.cross_gap, 15.0);
}

#[test]
fn grid_absolute_child_is_eligible() {
    let mut s = Session::new();
    let p_res = s.execute("frame.create", &json!({"width": 200, "height": 200})).unwrap();
    let pid = crate::params::id_of(&p_res["id"]).unwrap();
    let r = s.execute("shape.rectangle", &json!({"x": 13, "y": 15, "width": 21, "height": 33})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(pid).unwrap().layout = Some(AutoLayout::default());
        doc.reparent(id, pid, None).unwrap();
        let n = doc.get_mut(id).unwrap();
        n.child_layout.absolute = true;
        n.child_layout.sizing_h = Sizing::Fixed;
        n.child_layout.sizing_v = Sizing::Fixed;
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "positions": true,
                    "dimensions": true,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 1);
    assert_eq!(out["protectedFields"], 0);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!((n.width, n.height), (20.0, 32.0));
    let c = s.doc().unwrap().frame_relative(id).as_coeffs();
    assert_eq!((c[4], c[5]), (12.0, 16.0));
}

#[test]
fn grid_bound_aliases_are_protected() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 20, "height": 20})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.corners = Corners { radii: [7.0, 11.0, 13.0, 15.0], smoothing: 0.0 };
        // Attach aggregate cornerRadius binding
        n.bound.insert("cornerRadius".to_string(), "fake-var-id".to_string());
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "radii": true
                },
                "replaceBindings": false
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 0);
    assert_eq!(out["protectedFields"], 4);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!(n.corners.radii, [7.0, 11.0, 13.0, 15.0]);
    assert_eq!(n.bound.get("cornerRadius"), Some(&"fake-var-id".to_string()));
}

#[test]
fn grid_property_toggles_are_exact() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 13, "y": 15, "width": 21, "height": 33})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    // Toggle only dimensions
    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "positions": false,
                    "dimensions": true,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 1);
    assert_eq!(out["protectedFields"], 0);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!((n.width, n.height), (20.0, 32.0));
    let c = s.doc().unwrap().frame_relative(id).as_coeffs();
    assert_eq!((c[4], c[5]), (13.0, 15.0)); // positions untouched
}

#[test]
fn grid_nested_selection_is_single_pass() {
    let mut s = Session::new();
    let p_res = s.execute("frame.create", &json!({"width": 201, "height": 201})).unwrap();
    let pid = crate::params::id_of(&p_res["id"]).unwrap();
    let r_res = s.execute("shape.rectangle", &json!({"x": 11, "y": 11, "width": 21, "height": 21})).unwrap();
    let cid = crate::params::id_of(&r_res["id"]).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        doc.reparent(cid, pid, None).unwrap();
    }

    // Call snapToGrid with BOTH parent and child selected
    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [pid, cid],
                "step": 4,
                "properties": {
                    "positions": true,
                    "dimensions": true,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 2);
    let doc = s.doc().unwrap();
    let parent = doc.node(pid).unwrap();
    let child = doc.node(cid).unwrap();
    assert_eq!((parent.width, parent.height), (200.0, 200.0));
    assert_eq!((child.width, child.height), (20.0, 20.0));
}

#[test]
fn grid_dimensions_respect_minima() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 2, "height": 2})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "properties": {
                    "dimensions": true,
                    "positions": false,
                    "padding": false,
                    "gaps": false,
                    "radii": false
                }
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 1);
    let n = s.doc().unwrap().node(id).unwrap();
    // width 2 with step 4 snaps to at least 1 step = 4
    assert_eq!((n.width, n.height), (4.0, 4.0));
}

#[test]
fn grid_bad_params_are_atomic() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    let before = snapshot(&s);

    // Invalid step (3)
    assert!(s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 3})).is_err());
    // Invalid property key
    assert!(s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "properties": {"unknown": true}})).is_err());
    // Non-boolean property value
    assert!(s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "properties": {"positions": 12}})).is_err());
    // Stale node ID
    assert!(s.execute("arrange.snapToGrid", &json!({"ids": [999999], "step": 4})).is_err());

    assert_eq!(snapshot(&s), before);
}

#[test]
fn grid_protected_sources_and_board() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();

    // Lock node
    s.doc_mut().unwrap().get_mut(id).unwrap().locked = true;
    let before = snapshot(&s);

    let res = s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4})).unwrap();
    assert_eq!(res["changed"], 0);
    assert_eq!(res["skipped"], 1);
    assert_eq!(snapshot(&s), before);
}

#[test]
fn grid_tokens_are_opt_in() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [9.0, 13.0, 17.0, 21.0], ..Default::default() });
    }
    let out = s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "bindVariables": false})).unwrap();
    assert_eq!(out["changed"], 1);
    let doc = s.doc().unwrap();
    assert!(doc.variables.collections.is_empty());
    assert!(doc.variables.variables.is_empty());
    let first_page = doc.node(doc.first_open_page().unwrap_or(doc.pages[0])).unwrap();
    assert!(!first_page.plugin_data.contains_key("figcraft.tools.gridCleanup"));
    let n = doc.node(id).unwrap();
    assert!(n.bound.is_empty());
    assert_eq!(n.layout.as_ref().unwrap().padding, [8.0, 12.0, 16.0, 20.0]);
}

#[test]
fn grid_tokens_reuse_values() {
    let mut s = Session::new();
    let r1 = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id1 = crate::params::id_of(&r1["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id1).unwrap().layout = Some(AutoLayout { padding: [15.0, 17.0, 15.0, 17.0], ..Default::default() });
    }
    let out1 = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id1],
                "step": 4,
                "bindVariables": true,
                "properties": {"padding": true, "positions": false, "dimensions": false, "gaps": false, "radii": false}
            }),
        )
        .unwrap();
    assert_eq!(out1["changed"], 1);
    let var_id = {
        let doc = s.doc().unwrap();
        assert_eq!(doc.variables.collections.len(), 1);
        assert_eq!(doc.variables.variables.len(), 1);
        let var = &doc.variables.variables[0];
        assert_eq!(var.name, "step-4/value-16");
        let n1 = doc.node(id1).unwrap();
        assert_eq!(n1.bound.get("paddingTop"), Some(&var.id));
        assert_eq!(n1.bound.get("paddingRight"), Some(&var.id));
        assert_eq!(n1.bound.get("paddingBottom"), Some(&var.id));
        assert_eq!(n1.bound.get("paddingLeft"), Some(&var.id));
        var.id.clone()
    };

    let r2 = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id2 = crate::params::id_of(&r2["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id2).unwrap().layout = Some(AutoLayout { padding: [16.0, 16.0, 16.0, 16.0], ..Default::default() });
    }
    let out2 = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id2],
                "step": 4,
                "bindVariables": true,
                "properties": {"padding": true, "positions": false, "dimensions": false, "gaps": false, "radii": false}
            }),
        )
        .unwrap();
    assert_eq!(out2["changed"], 1);
    let doc2 = s.doc().unwrap();
    assert_eq!(doc2.variables.collections.len(), 1);
    assert_eq!(doc2.variables.variables.len(), 1);
    let n2 = doc2.node(id2).unwrap();
    assert_eq!(n2.bound.get("paddingTop"), Some(&var_id));
}

#[test]
fn grid_tokens_bind_already_snapped_value() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [16.0, 16.0, 16.0, 16.0], ..Default::default() });
    }
    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "bindVariables": true,
                "properties": {"padding": true, "positions": false, "dimensions": false, "gaps": false, "radii": false}
            }),
        )
        .unwrap();
    assert_eq!(out["changed"], 1);
    let doc = s.doc().unwrap();
    let n = doc.node(id).unwrap();
    assert!(n.bound.contains_key("paddingTop"));
}

#[test]
fn grid_tokens_preserve_user_collision() {
    let mut s = Session::new();
    s.execute("variable.createCollection", &json!({"name": "FigCraft / Grid Cleanup"})).unwrap();
    s.execute(
        "variable.create",
        &json!({
            "collection": "FigCraft / Grid Cleanup",
            "name": "step-4/value-16",
            "type": "number",
            "value": 999.0
        }),
    )
    .unwrap();

    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
    }
    let out = s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "bindVariables": true})).unwrap();
    assert_eq!(out["changed"], 1);

    let doc = s.doc().unwrap();
    assert_eq!(doc.variables.collections.len(), 2);
    let user_coll = &doc.variables.collections[0];
    assert_eq!(user_coll.name, "FigCraft / Grid Cleanup");
    let user_var = doc.variables.variables.iter().find(|v| v.collection == user_coll.id).unwrap();
    assert_eq!(user_var.name, "step-4/value-16");
    let user_val = user_var.values.values().next().unwrap();
    assert_eq!(user_val, &figcraft_doc::VariableValue::Number(999.0));

    let tool_coll = &doc.variables.collections[1];
    assert_eq!(tool_coll.name, "FigCraft / Grid Cleanup 2");
    let tool_var = doc.variables.variables.iter().find(|v| v.collection == tool_coll.id).unwrap();
    assert_eq!(tool_var.name, "step-4/value-16");
    let tool_val = tool_var.values.values().next().unwrap();
    assert_eq!(tool_val, &figcraft_doc::VariableValue::Number(16.0));
}

#[test]
fn grid_tokens_preserve_edited_owned_token() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
    }
    s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "bindVariables": true})).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        let var = doc.variables.variables.iter_mut().find(|v| v.name == "step-4/value-16").unwrap();
        for val in var.values.values_mut() {
            *val = figcraft_doc::VariableValue::Number(18.0);
        }
    }

    let r2 = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id2 = crate::params::id_of(&r2["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id2).unwrap().layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
    }
    s.execute("arrange.snapToGrid", &json!({"ids": [id2], "step": 4, "bindVariables": true})).unwrap();

    let doc = s.doc().unwrap();
    let edited_token = doc.variables.variables.iter().find(|v| v.name == "step-4/value-16").unwrap();
    assert_eq!(edited_token.values.values().next().unwrap(), &figcraft_doc::VariableValue::Number(18.0));

    let new_token = doc.variables.variables.iter().find(|v| v.name == "step-4/value-16 2").unwrap();
    assert_eq!(new_token.values.values().next().unwrap(), &figcraft_doc::VariableValue::Number(16.0));
}

#[test]
fn grid_tokens_all_modes_equal_target() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
    }
    s.execute("arrange.snapToGrid", &json!({"ids": [id], "step": 4, "bindVariables": true})).unwrap();

    {
        let doc = s.doc_mut().unwrap();
        let coll = doc.variables.collections.first_mut().unwrap();
        let mid = format!("{}:mode:2", coll.id);
        coll.modes.push(figcraft_doc::Mode { id: mid, name: "Dark".to_string() });
    }

    let r2 = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id2 = crate::params::id_of(&r2["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id2).unwrap().layout = Some(AutoLayout { padding: [23.0, 23.0, 23.0, 23.0], ..Default::default() });
    }
    s.execute("arrange.snapToGrid", &json!({"ids": [id2], "step": 4, "bindVariables": true})).unwrap();

    let doc = s.doc().unwrap();
    let tok24 = doc.variables.variables.iter().find(|v| v.name == "step-4/value-24").unwrap();
    assert_eq!(tok24.values.len(), 2);
    for v in tok24.values.values() {
        assert_eq!(v, &figcraft_doc::VariableValue::Number(24.0));
    }
}

#[test]
fn grid_existing_bindings_require_opt_in() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
        n.bound.insert("paddingTop".to_string(), "ext-var".to_string());
    }

    let out1 = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id], "step": 4, "bindVariables": true, "replaceBindings": false,
                "properties": {"padding": true}
            }),
        )
        .unwrap();
    assert_eq!(out1["protectedFields"], 1);
    assert_eq!(s.doc().unwrap().node(id).unwrap().bound.get("paddingTop"), Some(&"ext-var".to_string()));
    assert_eq!(s.doc().unwrap().node(id).unwrap().layout.as_ref().unwrap().padding[0], 15.0);

    let out2 = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id], "step": 4, "bindVariables": true, "replaceBindings": true,
                "properties": {"padding": true}
            }),
        )
        .unwrap();
    assert_eq!(out2["changed"], 1);
    let tok_id = s.doc().unwrap().node(id).unwrap().bound.get("paddingTop").cloned().unwrap();
    assert_ne!(tok_id, "ext-var");
    assert_eq!(s.doc().unwrap().node(id).unwrap().layout.as_ref().unwrap().padding[0], 16.0);

    let out3 = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id], "step": 4, "bindVariables": false, "replaceBindings": true,
                "properties": {"padding": true}
            }),
        )
        .unwrap();
    assert_eq!(out3["changed"], 1);
    assert_eq!(s.doc().unwrap().node(id).unwrap().bound.get("paddingTop"), None);
    assert_eq!(s.doc().unwrap().node(id).unwrap().layout.as_ref().unwrap().padding[0], 16.0);
}

#[test]
fn grid_aggregate_binding_unrequested_side() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
        n.corners = Corners { radii: [7.0, 7.0, 7.0, 7.0], smoothing: 0.0 };
        n.bound.insert("cornerRadius".to_string(), "ext-corner-var".to_string());
        n.bound.insert("paddingHorizontal".to_string(), "ext-pad-var".to_string());
    }

    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "replaceBindings": true,
                "properties": {
                    "positions": false,
                    "dimensions": false,
                    "padding": false,
                    "gaps": false,
                    "radii": true
                }
            }),
        )
        .unwrap();

    assert_eq!(out["changed"], 1);
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!(n.bound.get("cornerRadius"), None);
    assert_eq!(n.corners.radii, [8.0, 8.0, 8.0, 8.0]);

    assert_eq!(n.bound.get("paddingHorizontal"), Some(&"ext-pad-var".to_string()));
    assert_eq!(n.layout.as_ref().unwrap().padding, [15.0, 15.0, 15.0, 15.0]);
}

#[test]
fn grid_tokens_cap_failure_rolls_back() {
    let mut s = Session::new();
    let mut ids = Vec::new();
    for i in 0..520 {
        let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
        let id = crate::params::id_of(&r["id"]).unwrap();
        {
            let doc = s.doc_mut().unwrap();
            doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [i as f64 * 4.0, 0.0, 0.0, 0.0], ..Default::default() });
        }
        ids.push(id);
    }
    let before = snapshot(&s);
    let depth_before = s.undo_depth();

    let res = s.execute(
        "arrange.snapToGrid",
        &json!({
            "ids": ids,
            "step": 4,
            "bindVariables": true,
            "properties": {"padding": true}
        }),
    );

    assert!(res.is_err());
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.undo_depth(), depth_before);
}

#[test]
fn grid_tokens_undo_redo() {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let id = crate::params::id_of(&r["id"]).unwrap();
    {
        let doc = s.doc_mut().unwrap();
        doc.get_mut(id).unwrap().layout = Some(AutoLayout { padding: [15.0, 15.0, 15.0, 15.0], ..Default::default() });
    }
    let before = snapshot(&s);
    let out = s
        .execute(
            "arrange.snapToGrid",
            &json!({
                "ids": [id],
                "step": 4,
                "bindVariables": true
            }),
        )
        .unwrap();
    assert_eq!(out["changed"], 1);

    let created_var_id = s.doc().unwrap().variables.variables[0].id.clone();
    let created_coll_id = s.doc().unwrap().variables.collections[0].id.clone();

    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before);
    assert!(s.doc().unwrap().variables.collections.is_empty());
    assert!(s.doc().unwrap().variables.variables.is_empty());

    assert!(s.redo().unwrap());
    let doc = s.doc().unwrap();
    assert_eq!(doc.variables.collections[0].id, created_coll_id);
    assert_eq!(doc.variables.variables[0].id, created_var_id);
    let n = doc.node(id).unwrap();
    assert_eq!(n.bound.get("paddingTop"), Some(&created_var_id));
    assert_eq!(n.layout.as_ref().unwrap().padding, [16.0, 16.0, 16.0, 16.0]);
}
