use figcraft_color::PaintKind;
use figcraft_doc::{NodeId, NodeKind};
use serde_json::json;

use super::helpers::{image_layer, snapshot};
use crate::Session;

fn donut_pixels(w: u32, h: u32) -> Vec<[u8; 4]> {
    let mut pixels = Vec::new();
    let cx = f64::from(w) / 2.0;
    let cy = f64::from(h) / 2.0;
    let r_out = cx * 0.8;
    let r_in = cx * 0.3;

    for y in 0..h {
        for x in 0..w {
            let dx = f64::from(x) + 0.5 - cx;
            let dy = f64::from(y) + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= r_out && dist >= r_in {
                pixels.push([0, 0, 0, 255]); // opaque black ring
            } else {
                pixels.push([0, 0, 0, 0]); // transparent
            }
        }
    }
    pixels
}

#[test]
fn trace_vectors_preserve_donut() {
    let mut s = Session::new();
    let donut = donut_pixels(20, 20);
    let src_id = image_layer(&mut s, 20, 20, &donut);
    s.set_selection(vec![src_id]);

    let before = snapshot(&s);

    let res = s
        .execute(
            "image.trace",
            &json!({
                "ids": [src_id],
                "mode": "monochrome",
                "output": "vectors"
            }),
        )
        .unwrap();

    let root_id = res["created"][0].as_u64().unwrap();
    let doc = s.doc().unwrap();
    let root = doc.node(NodeId(root_id)).expect("root node exists");
    assert!(matches!(root.kind, NodeKind::Group));
    assert_eq!(root.children.len(), 1);

    let child = doc.node(root.children[0]).expect("child vector node");
    match &child.kind {
        NodeKind::Vector { network } => {
            // Must have vertices and at least 2 loops (outer ring + inner hole)
            assert!(network.vertices.len() >= 8);
            assert!(!network.regions.is_empty());
            assert_eq!(network.regions[0].loops.len(), 2);
            assert_eq!(network.regions[0].winding, figcraft_doc::WindingRule::EvenOdd);
        }
        _ => panic!("expected vector node"),
    }

    // Source unchanged
    assert!(doc.contains(src_id));

    // 1 undo restores snapshot
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before);

    // Redo restores output
    assert!(s.redo().unwrap());
    let doc_redo = s.doc().unwrap();
    assert!(doc_redo.contains(NodeId(root_id)));
}

#[test]
fn trace_color_regions_and_alpha() {
    let mut s = Session::new();
    // 20x20 with two distinct colors and transparent margin
    let mut pixels = Vec::new();
    for y in 0..20 {
        for x in 0..20 {
            if !(4..16).contains(&y) || !(4..16).contains(&x) {
                pixels.push([0, 0, 0, 0]);
            } else if x < 10 {
                pixels.push([255, 0, 0, 255]); // red
            } else {
                pixels.push([0, 0, 255, 255]); // blue
            }
        }
    }
    let src_id = image_layer(&mut s, 20, 20, &pixels);
    s.set_selection(vec![src_id]);

    let res = s
        .execute(
            "image.trace",
            &json!({
                "ids": [src_id],
                "mode": "color",
                "colors": 4,
                "output": "vectors"
            }),
        )
        .unwrap();

    let root_id = res["created"][0].as_u64().unwrap();
    let doc = s.doc().unwrap();
    let root = doc.node(NodeId(root_id)).expect("root group");
    assert!(root.children.len() <= 2);
    assert!(!root.children.is_empty());
}

#[test]
fn trace_mask_does_not_mask_page_siblings() {
    let mut s = Session::new();
    let donut = donut_pixels(20, 20);
    let src_id = image_layer(&mut s, 20, 20, &donut);
    // Add a sibling above
    let sibling_id = image_layer(&mut s, 20, 20, &donut);
    s.set_selection(vec![src_id]);

    let res = s
        .execute(
            "image.trace",
            &json!({
                "ids": [src_id],
                "output": "mask"
            }),
        )
        .unwrap();

    let root_id = res["created"][0].as_u64().unwrap();
    let doc = s.doc().unwrap();
    let root = doc.node(NodeId(root_id)).expect("mask frame");
    assert!(matches!(root.kind, NodeKind::Frame));
    assert!(root.clip);
    assert_eq!(root.children.len(), 2);

    // Bottom child: vector mask
    let mask_child = doc.node(root.children[0]).expect("mask child");
    assert!(mask_child.is_mask);
    assert!(matches!(mask_child.kind, NodeKind::Vector { .. }));

    // Top child: cloned image
    let img_child = doc.node(root.children[1]).expect("img child");
    assert!(!img_child.is_mask);
    assert!(matches!(img_child.kind, NodeKind::Rectangle));

    // Sibling above output is still an ordinary node, NOT masked
    let sibling = doc.node(sibling_id).expect("sibling exists");
    assert!(!sibling.is_mask);
    assert!(doc.contains(src_id));
}

#[test]
fn trace_crop_and_parent_transform() {
    let mut s = Session::new();
    let donut = donut_pixels(20, 20);
    let src_id = image_layer(&mut s, 20, 20, &donut);

    // Set rotation on image paint and opacity on node
    {
        let doc = s.doc_mut().unwrap();
        let src = doc.get_mut(src_id).unwrap();
        src.opacity = 0.8;
        if let Some(p) = src.fills.get_mut(0)
            && let PaintKind::Image(ref mut img) = p.kind
        {
            img.rotation = 1;
        }
    }

    let res = s.execute("image.trace", &json!({"ids": [src_id], "output": "vectors"})).unwrap();
    let root_id = res["created"][0].as_u64().unwrap();
    let doc = s.doc().unwrap();
    let root = doc.node(NodeId(root_id)).unwrap();
    assert_eq!(root.opacity, 0.8);
}

#[test]
fn trace_batch_failure_is_atomic() {
    let mut s = Session::new();
    let donut = donut_pixels(20, 20);
    let id1 = image_layer(&mut s, 20, 20, &donut);
    let id2 = image_layer(&mut s, 20, 20, &donut);

    // Corrupt id2's image bytes
    {
        let doc = s.doc_mut().unwrap();
        let src2 = doc.get(id2).unwrap();
        let hash = match &src2.fills[0].kind {
            PaintKind::Image(img) => img.image.clone(),
            _ => panic!("expected image"),
        };
        doc.images.get_mut(&hash).unwrap().bytes = std::sync::Arc::new(vec![1, 2, 3]);
    }

    let before = snapshot(&s);
    let err = s.execute("image.trace", &json!({"ids": [id1, id2]}));
    assert!(err.is_err());
    assert_eq!(snapshot(&s), before);
}

#[test]
fn trace_rejects_invalid_limits() {
    let mut s = Session::new();
    let donut = donut_pixels(20, 20);
    let src_id = image_layer(&mut s, 20, 20, &donut);

    // resolution 513
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "resolution": 513})).is_err());
    // colors 17
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "colors": 17})).is_err());
    // alphaCutoff 0
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "alphaCutoff": 0})).is_err());
    // negative threshold
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "threshold": -5})).is_err());
    // fractional threshold
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "threshold": 12.5})).is_err());
    // invalid mode enum
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "mode": "magic"})).is_err());
    // invalid output enum
    assert!(s.execute("image.trace", &json!({"ids": [src_id], "output": "png"})).is_err());

    // 17 eligible sources error
    let mut seventeen_ids = vec![src_id];
    for _ in 0..16 {
        seventeen_ids.push(image_layer(&mut s, 20, 20, &donut));
    }
    assert!(s.execute("image.trace", &json!({"ids": seventeen_ids})).is_err());
}

#[test]
fn trace_empty_and_protected() {
    let mut s = Session::new();
    // 10x10 completely transparent image
    let transparent = vec![[0, 0, 0, 0]; 100];
    let src_id = image_layer(&mut s, 10, 10, &transparent);

    let res = s.execute("image.trace", &json!({"ids": [src_id]})).unwrap();
    assert_eq!(res["created"].as_array().unwrap().len(), 0);
    assert_eq!(res["skipped"].as_u64().unwrap(), 1);

    // Locked source is protected and skipped
    let donut = donut_pixels(20, 20);
    let locked_id = image_layer(&mut s, 20, 20, &donut);
    s.doc_mut().unwrap().get_mut(locked_id).unwrap().locked = true;

    let res_locked = s.execute("image.trace", &json!({"ids": [locked_id]})).unwrap();
    assert_eq!(res_locked["created"].as_array().unwrap().len(), 0);
    assert_eq!(res_locked["skipped"].as_u64().unwrap(), 1);
}

#[test]
fn trace_batch_places_each_result_above_its_live_source() {
    for output in ["vectors", "mask"] {
        for reverse in [false, true] {
            let mut s = Session::new();
            let a = image_layer(&mut s, 2, 2, &[[0, 0, 0, 255]; 4]);
            let b = image_layer(&mut s, 2, 2, &[[0, 0, 0, 255]; 4]);
            let ids = if reverse { vec![b, a] } else { vec![a, b] };
            let before = snapshot(&s);
            let out = s.execute("image.trace", &json!({"ids": ids, "output": output})).unwrap();
            for (i, src) in ids.iter().enumerate() {
                let result = crate::params::id_of(&out["created"][i]).unwrap();
                let doc = s.doc().unwrap();
                let parent = doc.node(doc.node(*src).unwrap().parent.unwrap()).unwrap();
                let pos = parent.children.iter().position(|id| id == src).unwrap();
                assert_eq!(parent.children.get(pos + 1), Some(&result));
            }
            let after = snapshot(&s);
            assert!(s.undo().unwrap());
            assert_eq!(snapshot(&s), before);
            assert!(s.redo().unwrap());
            assert_eq!(snapshot(&s), after);
        }
    }
}
