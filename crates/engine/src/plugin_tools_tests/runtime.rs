use figcraft_color::ImageScaleMode;
use serde_json::json;
use std::path::PathBuf;

use super::helpers::{first_image, image_layer, snapshot};
use crate::Session;

pub(super) fn dist_manifest(plugin_name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().expect("crates").parent().expect("repo root").to_path_buf();
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let manifest = target.join("plugins/dist").join(plugin_name).join("figcraft-plugin.json");
    if !manifest.exists() {
        panic!("missing plugin artifact at {}; build with: cargo xtask plugins", manifest.display());
    }
    manifest
}

#[test]
#[ignore = "requires pre-built plugin artifact: cargo xtask plugins"]
fn built_fit_images() {
    let mut s = Session::new();
    let id1 = image_layer(&mut s, 2, 1, &[[255, 0, 0, 255], [0, 255, 0, 255]]);
    let id2 = image_layer(&mut s, 2, 1, &[[0, 0, 255, 255], [255, 255, 0, 255]]);
    s.set_selection(vec![id1]);

    let manifest_path = dist_manifest("fit-images");
    let manifest_str = manifest_path.to_str().expect("utf8 path");

    // 1. Import plugin
    let imported = s.execute("plugin.import", &json!({"path": manifest_str})).unwrap();
    assert_eq!(imported["id"], "org.figcraft.tools.fit-images");

    let before_panel = snapshot(&s);

    // 2. Open panel, assert no document or history change
    let res = s.execute("plugin.run", &json!({"id": "org.figcraft.tools.fit-images", "command": "panel"})).unwrap();
    assert_eq!(snapshot(&s), before_panel);
    assert!(res["panel"].is_object(), "panel must be open");

    // 3. Change mode field to fit -> assert still no document change
    s.execute("plugin.event", &json!({"id": "org.figcraft.tools.fit-images", "control": "mode", "value": "fit"})).unwrap();
    assert_eq!(snapshot(&s), before_panel);

    // 4. Change selected fixture after panel opens -> Apply must affect NEW selection (id2) only!
    s.set_selection(vec![id2]);
    let before_apply = snapshot(&s);
    let apply_res = s.execute("plugin.event", &json!({"id": "org.figcraft.tools.fit-images", "control": "apply"})).unwrap();
    assert_eq!(apply_res["output"]["changed"], 1);

    // id2 scale_mode became Fit
    let p2 = first_image(&s, id2);
    assert_eq!(p2.scale_mode, ImageScaleMode::Fit);

    // 5. One undo restores pre-Apply document
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s), before_apply);

    // 6. Close and reopen panel, verify last mode setting ("fit") is remembered
    s.execute("plugin.close", &json!({"id": "org.figcraft.tools.fit-images"})).unwrap();
    let reopen = s.execute("plugin.run", &json!({"id": "org.figcraft.tools.fit-images", "command": "panel"})).unwrap();
    assert_eq!(reopen["panel"]["values"]["mode"], "fit");

    // 7. Headless apply with explicit params
    s.set_selection(vec![id1]);
    let headless = s
        .execute(
            "plugin.run",
            &json!({
                "id": "org.figcraft.tools.fit-images",
                "command": "apply",
                "params": {"mode": "fit", "descendants": false}
            }),
        )
        .unwrap();
    assert_eq!(headless["output"]["changed"], 1);
    let p1 = first_image(&s, id1);
    assert_eq!(p1.scale_mode, ImageScaleMode::Fit);
}
