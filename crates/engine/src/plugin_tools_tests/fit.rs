use figcraft_color::{ImageScaleMode, Paint, PaintKind};
use serde_json::json;

use super::helpers::{first_image, image_layer, snapshot};
use crate::Session;

#[test]
fn fit_preserves_dimensions_and_image_settings() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    s.execute("node.set", &json!({"ids": [id], "props": {"width": 80, "height": 80}})).unwrap();
    s.execute("image.adjust", &json!({"ids": [id], "filters": {"contrast": 0.25}})).unwrap();
    let before = snapshot(&s);
    let paint = first_image(&s, id);
    let r = s.execute("image.fit", &json!({"ids": [id]})).unwrap();
    assert_eq!(r["changed"], 1);
    assert_eq!(r["created"], json!([]));
    let n = s.doc().unwrap().node(id).unwrap();
    assert_eq!((n.width, n.height), (80.0, 80.0));
    let after_paint = first_image(&s, id);
    assert_eq!(after_paint.scale_mode, ImageScaleMode::Fit);
    assert_eq!(after_paint.filters, paint.filters);
    assert_eq!(after_paint.image, paint.image);
    let after = snapshot(&s);
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before);
    assert!(s.redo().unwrap());
    assert_eq!(snapshot(&s), after);
}

#[test]
fn fit_fill_and_noop_counts() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    s.set_selection(vec![id]);

    // Fit produces changed: 1, skipped: 0
    let r1 = s.execute("image.fit", &json!({"mode": "fit"})).unwrap();
    assert_eq!(r1["changed"], 1);
    assert_eq!(r1["skipped"], 0);
    assert_eq!(s.selection(), vec![id]);

    // Fill produces changed: 1, skipped: 0
    let r2 = s.execute("image.fit", &json!({"mode": "fill"})).unwrap();
    assert_eq!(r2["changed"], 1);
    assert_eq!(r2["skipped"], 0);
    assert_eq!(s.selection(), vec![id]);

    // Repeating Fill produces changed: 0, skipped: 1
    let r3 = s.execute("image.fit", &json!({"mode": "fill"})).unwrap();
    assert_eq!(r3["changed"], 0);
    assert_eq!(r3["skipped"], 1);
    assert_eq!(s.selection(), vec![id]);
}

#[test]
fn fit_preserves_mixed_paints() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);

    // Add hidden image fill, solid fill, and stroke image
    let doc = s.doc_mut().unwrap();
    let hash = {
        let n = doc.node(id).unwrap();
        match &n.fills[0].kind {
            PaintKind::Image(img) => img.image.clone(),
            _ => String::new(),
        }
    };
    let n = doc.get_mut(id).unwrap();
    let mut hidden_fill = Paint::image(hash.clone(), ImageScaleMode::Fill);
    hidden_fill.visible = false;
    n.fills.push(hidden_fill.clone());
    let solid_fill = Paint::solid(figcraft_color::Color::from_rgba8(10, 20, 30, 255));
    n.fills.push(solid_fill.clone());
    let stroke_img = Paint::image(hash, ImageScaleMode::Fill);
    n.strokes.push(stroke_img.clone());

    let r = s.execute("image.fit", &json!({"ids": [id], "mode": "fit"})).unwrap();
    assert_eq!(r["changed"], 1);

    let doc = s.doc().unwrap();
    let n = doc.node(id).unwrap();
    // First fill changed
    assert!(matches!(&n.fills[0].kind, PaintKind::Image(img) if img.scale_mode == ImageScaleMode::Fit));
    // Hidden fill unchanged
    assert_eq!(n.fills[1], hidden_fill);
    // Solid fill unchanged
    assert_eq!(n.fills[2], solid_fill);
    // Stroke unchanged
    assert_eq!(n.strokes[0], stroke_img);
}

#[test]
fn fit_detaches_only_changed_fill_style() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.styles.fill = Some("style-fill-1".into());
    }

    // Change mode -> fill style detached
    let r1 = s.execute("image.fit", &json!({"ids": [id], "mode": "fit"})).unwrap();
    assert_eq!(r1["changed"], 1);
    assert_eq!(s.doc().unwrap().node(id).unwrap().styles.fill, None);

    // Reattach style, run no-op Fit -> fill style NOT detached
    {
        let doc = s.doc_mut().unwrap();
        let n = doc.get_mut(id).unwrap();
        n.styles.fill = Some("style-fill-2".into());
    }
    let r2 = s.execute("image.fit", &json!({"ids": [id], "mode": "fit"})).unwrap();
    assert_eq!(r2["changed"], 0);
    assert_eq!(s.doc().unwrap().node(id).unwrap().styles.fill, Some("style-fill-2".into()));
}

#[test]
fn targets_deduplicate_overlap() {
    let mut s = Session::new();
    let parent = s.execute("frame.create", &json!({"name": "Parent", "width": 200, "height": 200})).unwrap();
    let parent_id = crate::params::id_of(&parent["id"]).unwrap();
    let child_id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    s.execute("node.reparent", &json!({"ids": [child_id], "parent": parent_id})).unwrap();

    // Select both parent and child with descendants true
    let r = s.execute("image.fit", &json!({"ids": [parent_id, child_id], "descendants": true, "mode": "fit"})).unwrap();
    assert_eq!(r["visited"], 2);
    assert_eq!(r["changed"], 1);
    assert_eq!(r["skipped"], 1); // parent has no image fill
}

#[test]
fn targets_prune_protected_subtrees() {
    let mut s = Session::new();
    let parent = s.execute("frame.create", &json!({"name": "LockedParent", "width": 200, "height": 200})).unwrap();
    let parent_id = crate::params::id_of(&parent["id"]).unwrap();
    let child_id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    s.execute("node.reparent", &json!({"ids": [child_id], "parent": parent_id})).unwrap();

    // Lock parent
    s.execute("node.set", &json!({"ids": [parent_id], "props": {"locked": true}})).unwrap();

    // Explicitly targeting child: protected because ancestor is locked!
    let r = s.execute("image.fit", &json!({"ids": [child_id], "mode": "fit"})).unwrap();
    assert_eq!(r["changed"], 0);
    assert_eq!(r["skipped"], 1);

    // With descendants on locked parent: protected root counted, subtree not walked
    let r2 = s.execute("image.fit", &json!({"ids": [parent_id], "descendants": true, "mode": "fit"})).unwrap();
    assert_eq!(r2["visited"], 1); // Only locked parent inspected
    assert_eq!(r2["changed"], 0);
    assert_eq!(r2["skipped"], 1);
}

#[test]
fn targets_limit_and_stale_id_are_atomic() {
    let mut s = Session::new();
    let id1 = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let before = snapshot(&s);

    // Existing image followed by missing ID errors with unchanged snapshot
    let err = s.execute("image.fit", &json!({"ids": [id1, 999999], "mode": "fit"})).unwrap_err();
    assert!(matches!(err, crate::EngineError::NoNode(_) | crate::EngineError::BadParams { .. }));
    assert_eq!(snapshot(&s), before);

    // Limit check on collect_targets
    let id2 = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let id3 = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let doc = s.doc().unwrap();
    let res = crate::plugin_tools::targets::collect_targets(doc, &[id1, id2, id3], false, 2);
    assert!(res.is_err(), "limit of 2 must reject 3 unique nodes");
}

#[test]
fn fit_rejects_invalid_parameters() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    s.set_selection(vec![]);

    // Empty selection
    let err = s.execute("image.fit", &json!({})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));

    // mode: stretch
    let err = s.execute("image.fit", &json!({"ids": [id], "mode": "stretch"})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));

    // mode object
    let err = s.execute("image.fit", &json!({"ids": [id], "mode": {"x": 1}})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));

    // descendants object
    let err = s.execute("image.fit", &json!({"ids": [id], "descendants": {"x": 1}})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));

    // non-array ids
    let err = s.execute("image.fit", &json!({"ids": "not_an_array"})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));

    // malformed ids (e.g. string inside ids array)
    let err = s.execute("image.fit", &json!({"ids": ["not_a_number"]})).unwrap_err();
    assert!(matches!(err, crate::EngineError::BadParams { .. }));
}

#[test]
fn fit_no_eligible_layers_and_board() {
    let mut s = Session::new();
    let rect = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let rect_id = crate::params::id_of(&rect["id"]).unwrap();

    // Non-image selection returns changed 0 and skipped count
    let r = s.execute("image.fit", &json!({"ids": [rect_id], "mode": "fit"})).unwrap();
    assert_eq!(r["changed"], 0);
    assert_eq!(r["skipped"], 1);

    // Board document is Disabled
    s.execute("file.newBoard", &json!({})).unwrap();
    let err = s.execute("image.fit", &json!({"ids": [rect_id]})).unwrap_err();
    assert!(matches!(err, crate::EngineError::Disabled(..)));
}

#[test]
fn fit_permission_read_only_denied() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    let wasm = crate::plugins_tests::script(&[("image.fit", json!({"ids": [id], "mode": "fit"}))], crate::plugins_tests::OnRefusal::Fail, "");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.fit.ro", "read", &["image.fit"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.fit.ro"})).unwrap_err();
    assert!(err.to_string().contains("may only read it") || err.to_string().contains("read-only"));
}

#[test]
fn fit_permission_write_omitted_denied() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    let wasm = crate::plugins_tests::script(&[("image.fit", json!({"ids": [id], "mode": "fit"}))], crate::plugins_tests::OnRefusal::Fail, "");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.fit.no_perm", "write", &["shape.rectangle"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.fit.no_perm"})).unwrap_err();
    assert!(err.to_string().contains("not allowed") || err.to_string().contains("permission"));
}

#[test]
fn fit_trap_rolls_back_changes() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    let before = snapshot(&s);
    let wasm =
        crate::plugins_tests::script(&[("image.fit", json!({"ids": [id], "mode": "fit"}))], crate::plugins_tests::OnRefusal::Fail, "(unreachable)");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.fit.trap", "write", &["image.fit"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.fit.trap"})).unwrap_err();
    assert!(err.to_string().contains("trap") || err.to_string().contains("unreachable") || err.to_string().contains("failed"));
    assert_eq!(snapshot(&s), before);
}
