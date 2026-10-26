use base64::Engine as _;
use figcraft_doc::NodeId;
use serde_json::Value;

use crate::Session;

pub(super) fn image_layer(s: &mut Session, w: u32, h: u32, rgba: &[[u8; 4]]) -> NodeId {
    assert_eq!(rgba.len(), (w * h) as usize, "rgba slice must match w*h");
    let mut img_buf = image::RgbaImage::new(w, h);
    for (i, p) in rgba.iter().enumerate() {
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        img_buf.put_pixel(x, y, image::Rgba(*p));
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    img_buf.write_to(&mut bytes, image::ImageFormat::Png).expect("PNG encode failed");
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    let r = s.execute("image.place", &serde_json::json!({"data": b64})).expect("image.place failed");
    crate::params::id_of(&r["id"]).expect("missing id in image.place output")
}

pub(super) fn snapshot(s: &Session) -> Value {
    serde_json::to_value(s.doc().expect("no document in snapshot")).expect("document serialization failed")
}

pub(super) fn first_image(s: &Session, id: NodeId) -> figcraft_color::ImagePaint {
    let node = s.doc().expect("no doc").node(id).unwrap_or_else(|| panic!("no node {id:?}"));
    for p in &node.fills {
        if let figcraft_color::PaintKind::Image(ref img) = p.kind {
            return img.clone();
        }
    }
    panic!("no image fill on node {id:?}");
}
