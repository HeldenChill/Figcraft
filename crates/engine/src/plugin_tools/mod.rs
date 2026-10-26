use figcraft_doc::NodeId;
use serde::{Deserialize, Serialize};

use crate::spec;

pub mod contours;
pub mod fit;
pub mod nine_slice;
pub mod quantize;
pub mod targets;
pub mod trace;

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolReport {
    pub visited: usize,
    pub changed: usize,
    pub skipped: usize,
    pub protected_fields: usize,
    pub created: Vec<NodeId>,
    pub warnings: Vec<String>,
}

pub(crate) fn specs() -> Vec<crate::cmd::CommandSpec> {
    vec![
        spec!("image.fit", "Fit images", "{ids?, mode? (fit|fill, default fit), descendants? (default false)}", true, fit::fit),
        spec!("image.nineSlice", "Nine-slice scaling", "{ids?, left?, right?, top?, bottom?}", true, nine_slice::nine_slice),
        spec!(
            "image.trace",
            "Trace bitmap image to vector shapes",
            "{ids?, mode?, threshold?, invert?, colors?, alphaCutoff?, resolution?, output?}",
            true,
            trace::trace
        ),
    ]
}
