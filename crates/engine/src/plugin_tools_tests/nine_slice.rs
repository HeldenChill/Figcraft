use figcraft_color::{ImagePaint, ImageScaleMode};
use figcraft_doc::NodeKind;
use serde_json::json;

use super::helpers::{first_image, image_layer, snapshot};
use crate::Session;
use crate::plugin_tools::nine_slice::SliceInsets;

fn assert_matrix_close(actual: [f64; 6], expected: [f64; 6], tolerance: f64) {
    for i in 0..6 {
        assert!(
            (actual[i] - expected[i]).abs() <= tolerance,
            "matrix mismatch at [{i}]: actual={:?}, expected={:?}, diff={}",
            actual,
            expected,
            (actual[i] - expected[i]).abs()
        );
    }
}

#[test]
fn slice_integration_creates_frame_with_nine_shared_rectangles() {
    let mut s = Session::new();
    let pixels: Vec<[u8; 4]> = vec![
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [255, 0, 255, 255],
        [0, 255, 255, 255],
        [128, 0, 0, 255],
        [0, 128, 0, 255],
        [0, 0, 128, 255],
    ];
    let src = image_layer(&mut s, 3, 3, &pixels);
    let original_hash = first_image(&s, src).image;

    let res = s
        .execute(
            "image.nineSlice",
            &json!({
                "ids": [src],
                "left": 1.0,
                "right": 1.0,
                "top": 1.0,
                "bottom": 1.0
            }),
        )
        .unwrap();

    assert_eq!(res["changed"], 1);
    let created = res["created"].as_array().unwrap();
    assert_eq!(created.len(), 1);
    let frame_id = crate::params::id_of(&created[0]).unwrap();

    let doc = s.doc().unwrap();
    let frame = doc.node(frame_id).unwrap();
    assert_eq!(frame.kind, NodeKind::Frame);
    assert_eq!(frame.children.len(), 9);

    for &child_id in &frame.children {
        let child = doc.node(child_id).unwrap();
        assert_eq!(child.kind, NodeKind::Rectangle);
        let paint = first_image(&s, child_id);
        assert_eq!(paint.image, original_hash);
        assert_eq!(paint.scale_mode, ImageScaleMode::Crop);
        assert!(paint.crop.is_some());
    }
}

#[test]
fn slice_fill_fit_crop_mapping() {
    let paint =
        ImagePaint { image: "img1".into(), scale_mode: ImageScaleMode::Fit, scale: 1.0, rotation: 0, crop: None, filters: Default::default() };
    let insets = SliceInsets { top: 20.0, right: 20.0, bottom: 20.0, left: 20.0 };
    let pairs = crate::plugin_tools::nine_slice::slice_paints(&paint, (100, 50), (100.0, 100.0), insets).unwrap();
    assert_eq!(pairs.len(), 9);
    let top_left = &pairs[0].1;
    assert_matrix_close(top_left.crop.unwrap(), [0.2, 0.0, 0.0, 0.4, 0.0, -0.5], 1e-9);

    let center = &pairs[4].1;
    assert_matrix_close(center.crop.unwrap(), [0.6, 0.0, 0.0, 1.2, 0.2, -0.1], 1e-9);
}

#[test]
fn slice_unequal_insets() {
    let paint =
        ImagePaint { image: "img1".into(), scale_mode: ImageScaleMode::Fill, scale: 1.0, rotation: 0, crop: None, filters: Default::default() };
    let insets = SliceInsets { top: 8.0, right: 20.0, bottom: 12.0, left: 10.0 };
    let pairs = crate::plugin_tools::nine_slice::slice_paints(&paint, (100, 80), (100.0, 80.0), insets).unwrap();
    assert_eq!(pairs[0].0.width(), 10.0);
    assert_eq!(pairs[0].0.height(), 8.0);
    assert_eq!(pairs[4].0.width(), 70.0);
    assert_eq!(pairs[4].0.height(), 60.0);
    assert_eq!(pairs[8].0.width(), 20.0);
    assert_eq!(pairs[8].0.height(), 12.0);
}

#[test]
fn slice_invalid_insets() {
    let paint =
        ImagePaint { image: "img1".into(), scale_mode: ImageScaleMode::Fill, scale: 1.0, rotation: 0, crop: None, filters: Default::default() };
    assert!(
        crate::plugin_tools::nine_slice::slice_paints(
            &paint,
            (100, 100),
            (100.0, 100.0),
            SliceInsets { top: 0.0, right: 10.0, bottom: 10.0, left: 10.0 }
        )
        .is_err()
    );
    assert!(
        crate::plugin_tools::nine_slice::slice_paints(
            &paint,
            (100, 100),
            (100.0, 100.0),
            SliceInsets { top: -5.0, right: 10.0, bottom: 10.0, left: 10.0 }
        )
        .is_err()
    );
    assert!(
        crate::plugin_tools::nine_slice::slice_paints(
            &paint,
            (100, 100),
            (100.0, 100.0),
            SliceInsets { top: 50.0, right: 10.0, bottom: 50.0, left: 10.0 }
        )
        .is_err()
    );
    assert!(
        crate::plugin_tools::nine_slice::slice_paints(
            &paint,
            (100, 100),
            (100.0, 100.0),
            SliceInsets { top: 10.0, right: 60.0, bottom: 10.0, left: 50.0 }
        )
        .is_err()
    );
    assert!(
        crate::plugin_tools::nine_slice::slice_paints(
            &paint,
            (100, 100),
            (100.0, 100.0),
            SliceInsets { top: f64::NAN, right: 10.0, bottom: 10.0, left: 10.0 }
        )
        .is_err()
    );
}

#[test]
fn slice_resize_preserves_borders() {
    let mut s = Session::new();
    let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255]; 100 * 80];
    let src = image_layer(&mut s, 100, 80, &pixels);
    let res = s
        .execute(
            "image.nineSlice",
            &json!({
                "ids": [src],
                "left": 10.0,
                "right": 20.0,
                "top": 8.0,
                "bottom": 12.0
            }),
        )
        .unwrap();
    let frame_id = crate::params::id_of(&res["created"][0]).unwrap();
    s.execute("node.set", &json!({"ids": [frame_id], "props": {"width": 180.0, "height": 120.0}})).unwrap();
    let doc = s.doc().unwrap();
    let frame = doc.node(frame_id).unwrap();
    assert_eq!((frame.width, frame.height), (180.0, 120.0));
    let tl = doc.node(frame.children[0]).unwrap();
    assert_eq!((tl.width, tl.height), (10.0, 8.0));
    let center = doc.node(frame.children[4]).unwrap();
    assert_eq!((center.width, center.height), (150.0, 100.0));
    let br = doc.node(frame.children[8]).unwrap();
    assert_eq!((br.width, br.height), (20.0, 12.0));
    let coeffs = br.transform.as_coeffs();
    assert_eq!((coeffs[4], coeffs[5]), (160.0, 108.0));
}

#[test]
fn slice_manual_undersize_rolls_back() {
    let mut s = Session::new();
    let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255]; 100 * 80];
    let src = image_layer(&mut s, 100, 80, &pixels);
    let res = s
        .execute(
            "image.nineSlice",
            &json!({
                "ids": [src],
                "left": 10.0,
                "right": 20.0,
                "top": 8.0,
                "bottom": 12.0
            }),
        )
        .unwrap();
    let frame_id = crate::params::id_of(&res["created"][0]).unwrap();
    let before = snapshot(&s);
    let err = s.execute("node.set", &json!({"ids": [frame_id], "props": {"width": 29.0}}));
    assert!(err.is_err());
    assert_eq!(snapshot(&s), before);
}

#[test]
fn slice_interactive_undersize_clamps() {
    let mut s = Session::new();
    let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255]; 100 * 80];
    let src = image_layer(&mut s, 100, 80, &pixels);
    let res = s
        .execute(
            "image.nineSlice",
            &json!({
                "ids": [src],
                "left": 10.0,
                "right": 20.0,
                "top": 8.0,
                "bottom": 12.0
            }),
        )
        .unwrap();
    let frame_id = crate::params::id_of(&res["created"][0]).unwrap();
    let before = s.doc().unwrap().clone();
    {
        let doc_mut = s.doc_mut().unwrap();
        let frame = doc_mut.get_mut(frame_id).unwrap();
        frame.width = 15.0;
        frame.height = 10.0;
    }
    crate::normalize(s.doc_mut().unwrap(), &before);
    let doc = s.doc().unwrap();
    let frame = doc.node(frame_id).unwrap();
    assert!(frame.width >= 30.0);
    assert!(frame.height >= 20.0);
    let center = doc.node(frame.children[4]).unwrap();
    assert!(center.width >= 0.0);
    assert!(center.height >= 0.0);
}

#[test]
fn slice_malformed_metadata_is_safe() {
    let mut node = figcraft_doc::Node::new(figcraft_doc::NodeId(1), NodeKind::Frame, "test", 100.0, 100.0);
    node.plugin_data.insert(crate::plugin_tools::nine_slice::NINE_SLICE_KEY.into(), "{\"version\":2}".into());
    assert!(crate::plugin_tools::nine_slice::minimum_size(&node).is_none());
    node.plugin_data.insert(crate::plugin_tools::nine_slice::NINE_SLICE_KEY.into(), "{\"version\":1,\"insets\":[1,2,3]}".into());
    assert!(crate::plugin_tools::nine_slice::minimum_size(&node).is_none());
    node.plugin_data.insert(crate::plugin_tools::nine_slice::NINE_SLICE_KEY.into(), "{\"version\":1,\"insets\":[\"NaN\",1,2,3]}".into());
    assert!(crate::plugin_tools::nine_slice::minimum_size(&node).is_none());
}

#[test]
fn slice_oversized_metadata_cannot_corrupt_geometry() {
    for inset in [1e308, 1_000_001.0] {
        let mut s = Session::new();
        let out = s.execute("frame.create", &json!({"width": 100, "height": 100})).unwrap();
        let id = crate::params::id_of(&out["id"]).unwrap();
        s.execute(
            "node.set",
            &json!({"ids": [id], "props": {
                "width": 120,
                "pluginData": {"figcraft.tools.nineSlice": {"version": 1, "insets": [inset, inset, inset, inset]}}
            }}),
        )
        .unwrap();
        let node = s.doc().unwrap().node(id).unwrap();
        assert_eq!((node.width, node.height), (120.0, 100.0));
        assert!(crate::plugin_tools::nine_slice::minimum_size(node).is_none());
        let saved = figcraft_format::save_bytes(s.doc().unwrap(), None).unwrap();
        let (reopened, _) = figcraft_format::load_bytes(&saved).unwrap();
        assert_eq!(reopened.node(id).unwrap().width, 120.0);
    }
}

#[test]
fn slice_scale_updates_borders_directly_and_nested() {
    for nested in [false, true] {
        let mut s = Session::new();
        let pixels: Vec<_> = (0..10_000).map(|i| if i % 100 < 10 && i / 100 < 10 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }).collect();
        let src = image_layer(&mut s, 100, 100, &pixels);
        let out = s.execute("image.nineSlice", &json!({"ids": [src], "left": 10, "right": 10, "top": 10, "bottom": 10})).unwrap();
        let frame = crate::params::id_of(&out["created"][0]).unwrap();
        let target = if nested {
            let parent = s.execute("frame.create", &json!({"width": 300, "height": 300})).unwrap();
            let parent = crate::params::id_of(&parent["id"]).unwrap();
            s.execute("node.reparent", &json!({"ids": [frame], "parent": parent})).unwrap();
            parent
        } else {
            frame
        };
        let before = snapshot(&s);
        s.execute("transform.scale", &json!({"ids": [target], "factor": 2})).unwrap();
        let scaled = snapshot(&s);
        let n = s.doc().unwrap().node(frame).unwrap();
        let corner = s.doc().unwrap().node(n.children[0]).unwrap();
        assert_eq!((corner.width, corner.height), (20.0, 20.0));
        let insets = crate::plugin_tools::nine_slice::parse_insets_metadata(n).unwrap();
        assert_eq!(insets.left, 20.0);
        assert!(s.undo().unwrap());
        assert_eq!(snapshot(&s), before);
        assert!(s.redo().unwrap());
        assert_eq!(snapshot(&s), scaled);
        s.execute("node.set", &json!({"ids": [frame], "props": {"width": 300, "height": 300}})).unwrap();
        let n = s.doc().unwrap().node(frame).unwrap();
        let corner = s.doc().unwrap().node(n.children[0]).unwrap();
        assert_eq!((corner.width, corner.height), (20.0, 20.0));
        let rendered = figcraft_render::render_node(s.doc().unwrap(), frame, 1.0).unwrap();
        let red = rendered.pixel(15, 15).unwrap();
        assert_eq!((red.red(), red.green(), red.blue(), red.alpha()), (255, 0, 0, 255));
        let blue = rendered.pixel(25, 25).unwrap();
        assert_eq!((blue.red(), blue.green(), blue.blue(), blue.alpha()), (0, 0, 255, 255));
    }
}

#[test]
fn slice_scale_out_of_range_rolls_back() {
    let mut s = Session::new();
    let src = image_layer(&mut s, 3, 3, &[[255, 0, 0, 255]; 9]);
    let out = s.execute("image.nineSlice", &json!({"ids": [src], "left": 1, "right": 1, "top": 1, "bottom": 1})).unwrap();
    let frame = crate::params::id_of(&out["created"][0]).unwrap();
    let before = snapshot(&s);
    assert!(s.execute("transform.scale", &json!({"ids": [frame], "factor": 1_000_001})).is_err());
    assert_eq!(snapshot(&s), before);
}

#[test]
fn slice_unrenderable_crops_are_rejected_atomically() {
    for zoomed in [false, true] {
        let mut s = Session::new();
        let src = image_layer(&mut s, 100, 100, &vec![[255, 0, 0, 255]; 10_000]);
        let inset = if zoomed {
            let n = s.doc_mut().unwrap().get_mut(src).unwrap();
            if let figcraft_color::PaintKind::Image(img) = &mut n.fills[0].kind {
                img.scale_mode = ImageScaleMode::Crop;
                img.crop = Some([2e-9, 0.0, 0.0, 2e-9, 0.5, 0.5]);
            }
            10.0
        } else {
            49.99999999
        };
        let before = snapshot(&s);
        let out = s.execute("image.nineSlice", &json!({"ids": [src], "left": inset, "right": inset, "top": 10, "bottom": 10}));
        assert!(out.is_err(), "unsupported crop must not silently become Fill");
        assert_eq!(snapshot(&s), before);
    }
}

#[test]
fn slice_nested_transform() {
    let mut s = Session::new();
    let parent = s.execute("frame.create", &json!({"width": 200, "height": 200, "rotation": 45})).unwrap();
    let pid = crate::params::id_of(&parent["id"]).unwrap();
    let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255]; 50 * 50];
    let src = image_layer(&mut s, 50, 50, &pixels);
    s.execute("node.reparent", &json!({"ids": [src], "parent": pid})).unwrap();
    s.execute("node.set", &json!({"ids": [src], "props": {"rotation": 30}})).unwrap();
    let before = snapshot(&s);
    let res = s.execute("image.nineSlice", &json!({"ids": [src], "left": 10, "right": 10, "top": 10, "bottom": 10})).unwrap();
    assert_eq!(res["changed"], 1);
    let frame_id = crate::params::id_of(&res["created"][0]).unwrap();
    assert_eq!(s.selection(), vec![frame_id]);
    let doc = s.doc().unwrap();
    assert_eq!(doc.world_transform(frame_id), doc.world_transform(src));
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before);
    assert!(s.redo().unwrap());
    assert_eq!(s.selection(), vec![frame_id]);
}

#[test]
fn slice_permission_read_only_denied() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 2, &[[255, 0, 0, 255]; 4]);
    let wasm = crate::plugins_tests::script(&[("image.nineSlice", json!({"ids": [id]}))], crate::plugins_tests::OnRefusal::Fail, "");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.slice.ro", "read", &["image.nineSlice"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.slice.ro"})).unwrap_err();
    assert!(err.to_string().contains("may only read it"));
}

#[test]
fn slice_permission_write_omitted_denied() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 2, &[[255, 0, 0, 255]; 4]);
    let wasm = crate::plugins_tests::script(&[("image.nineSlice", json!({"ids": [id]}))], crate::plugins_tests::OnRefusal::Fail, "");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.slice.unperm", "write", &["image.fit"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.slice.unperm"})).unwrap_err();
    assert!(err.to_string().contains("not allowed") || err.to_string().contains("permission"));
}

#[test]
fn slice_trap_rolls_back_changes() {
    let mut s = Session::new();
    let id = image_layer(&mut s, 2, 2, &[[255, 0, 0, 255]; 4]);
    let before = snapshot(&s);
    let wasm = crate::plugins_tests::script(&[("image.nineSlice", json!({"ids": [id]}))], crate::plugins_tests::OnRefusal::Fail, "(unreachable)");
    crate::plugins_tests::install(&mut s, crate::plugins_tests::manifest("test.slice.trap", "write", &["image.nineSlice"]), &wasm);
    let err = s.execute("plugin.run", &json!({"id": "test.slice.trap"})).unwrap_err();
    assert!(err.to_string().contains("trap") || err.to_string().contains("unreachable") || err.to_string().contains("failed"));
    assert_eq!(snapshot(&s), before);
}
