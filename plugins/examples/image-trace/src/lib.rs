use figcraft_plugin_sdk::{Event, Input, Result, Value, call, close_ui, entry, json, notify, selection, set_setting, setting, show_ui};

pub fn operation_params(values: &Value, ids: &[u64]) -> Result<Value> {
    if ids.is_empty() {
        return Err("No layer selected: please select an image rectangle".into());
    }
    if ids.len() > 16 {
        return Err("At most 16 image layers can be traced at once".into());
    }

    let mode = match values.get("mode") {
        Some(Value::String(s)) => {
            if s == "monochrome" || s == "color" {
                s.clone()
            } else {
                return Err("`mode` must be 'monochrome' or 'color'".into());
            }
        }
        None | Some(Value::Null) => "monochrome".to_string(),
        _ => return Err("`mode` must be a string".into()),
    };

    let parse_u8 = |key: &str, default: u8, min: u8, max: u8| -> Result<u8> {
        match values.get(key) {
            None | Some(Value::Null) => Ok(default),
            Some(v) => {
                let n = if let Some(num) = v.as_u64() {
                    num
                } else if let Some(s) = v.as_str() {
                    s.trim().parse::<u64>().map_err(|_| format!("`{key}` must be an integer"))?
                } else if let Some(f) = v.as_f64() {
                    if f.fract() != 0.0 {
                        return Err(format!("`{key}` must be an integer"));
                    }
                    f as u64
                } else {
                    return Err(format!("`{key}` must be an integer"));
                };
                if n < min as u64 || n > max as u64 {
                    return Err(format!("`{key}` must be between {min} and {max}"));
                }
                Ok(n as u8)
            }
        }
    };

    let parse_u32 = |key: &str, default: u32, min: u32, max: u32| -> Result<u32> {
        match values.get(key) {
            None | Some(Value::Null) => Ok(default),
            Some(v) => {
                let n = if let Some(num) = v.as_u64() {
                    num
                } else if let Some(s) = v.as_str() {
                    s.trim().parse::<u64>().map_err(|_| format!("`{key}` must be an integer"))?
                } else if let Some(f) = v.as_f64() {
                    if f.fract() != 0.0 {
                        return Err(format!("`{key}` must be an integer"));
                    }
                    f as u64
                } else {
                    return Err(format!("`{key}` must be an integer"));
                };
                if n < min as u64 || n > max as u64 {
                    return Err(format!("`{key}` must be between {min} and {max}"));
                }
                Ok(n as u32)
            }
        }
    };

    let threshold = parse_u8("threshold", 128, 0, 255)?;
    let colors = parse_u8("colors", 8, 2, 16)?;
    let alpha_cutoff = parse_u8("alphaCutoff", 16, 1, 255)?;
    let resolution = parse_u32("resolution", 256, 16, 512)?;

    let invert = match values.get("invert") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        None | Some(Value::Null) => false,
        _ => return Err("`invert` must be a boolean".into()),
    };

    let output = match values.get("output") {
        Some(Value::String(s)) => {
            if s == "vectors" || s == "mask" {
                s.clone()
            } else {
                return Err("`output` must be 'vectors' or 'mask'".into());
            }
        }
        None | Some(Value::Null) => "vectors".to_string(),
        _ => return Err("`output` must be a string".into()),
    };

    Ok(json!({
        "ids": ids,
        "mode": mode,
        "threshold": threshold,
        "invert": invert,
        "colors": colors,
        "alphaCutoff": alpha_cutoff,
        "resolution": resolution,
        "output": output
    }))
}

fn show_panel(mode: &str, threshold: u8, invert: bool, colors: u8, alpha_cutoff: u8, resolution: u32, output: &str) -> Result<Value> {
    show_ui(&json!({
        "title": "Image Trace",
        "controls": [
            {
                "type": "list",
                "name": "mode",
                "label": "Mode",
                "options": ["monochrome", "color"],
                "value": mode
            },
            {
                "type": "number",
                "name": "threshold",
                "label": "Threshold",
                "value": threshold,
                "min": 0,
                "max": 255
            },
            {
                "type": "boolean",
                "name": "invert",
                "label": "Invert silhouette",
                "value": invert
            },
            {
                "type": "number",
                "name": "colors",
                "label": "Max colors",
                "value": colors,
                "min": 2,
                "max": 16
            },
            {
                "type": "number",
                "name": "alphaCutoff",
                "label": "Alpha cutoff",
                "value": alpha_cutoff,
                "min": 1,
                "max": 255
            },
            {
                "type": "number",
                "name": "resolution",
                "label": "Resolution (max side)",
                "value": resolution,
                "min": 16,
                "max": 512
            },
            {
                "type": "list",
                "name": "output",
                "label": "Output",
                "options": ["vectors", "mask"],
                "value": output
            },
            {
                "type": "note",
                "label": "Flat artwork and silhouettes; reduced palette and pixel boundaries, no curve smoothing. Original stays unchanged."
            },
            {
                "type": "button",
                "name": "apply",
                "label": "Apply"
            },
            {
                "type": "button",
                "name": "close",
                "label": "Close"
            }
        ]
    }))?;
    Ok(Value::Null)
}

fn save_settings(v: &Value) -> Result<()> {
    for key in &["mode", "threshold", "invert", "colors", "alphaCutoff", "resolution", "output"] {
        if let Some(val) = v.get(*key) {
            set_setting(key, val)?;
        }
    }
    Ok(())
}

fn on_event(e: Event) -> Result<Value> {
    match e.control.as_str() {
        "apply" => {
            let current_sel = selection();
            let params = operation_params(&e.values, &current_sel)?;
            let report = call("image.trace", &params)?;
            save_settings(&e.values)?;
            let created = report.get("created").and_then(Value::as_array).map_or(0, |a| a.len());
            notify(&format!("Traced {created} layers"));
            close_ui();
            Ok(report)
        }
        "close" => {
            close_ui();
            Ok(Value::Null)
        }
        "mode" | "threshold" | "invert" | "colors" | "alphaCutoff" | "resolution" | "output" => {
            if let Some(val) = e.values.get(&e.control) {
                set_setting(&e.control, val)?;
            }
            Ok(Value::Null)
        }
        _ => Ok(Value::Null),
    }
}

fn run(input: Input) -> Result<Value> {
    if let Some(e) = input.event {
        return on_event(e);
    }
    if input.command == "panel" {
        let mode = setting("mode").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "monochrome".to_string());
        let threshold = setting("threshold").and_then(|v| v.as_u64()).map_or(128, |n| n as u8);
        let invert = setting("invert").and_then(|v| v.as_bool()).unwrap_or(false);
        let colors = setting("colors").and_then(|v| v.as_u64()).map_or(8, |n| n as u8);
        let alpha_cutoff = setting("alphaCutoff").and_then(|v| v.as_u64()).map_or(16, |n| n as u8);
        let resolution = setting("resolution").and_then(|v| v.as_u64()).map_or(256, |n| n as u32);
        let output = setting("output").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "vectors".to_string());

        return show_panel(&mode, threshold, invert, colors, alpha_cutoff, resolution, &output);
    }
    if input.command == "apply" {
        let ids = match input.params.get("ids").and_then(Value::as_array) {
            Some(arr) => arr.iter().filter_map(Value::as_u64).collect(),
            None => selection(),
        };
        let params = operation_params(&input.params, &ids)?;
        return call("image.trace", &params);
    }
    Err(format!("unknown command `{}`", input.command))
}

entry!(run);

#[cfg(test)]
mod tests {
    use super::operation_params;
    use serde_json::json;

    #[test]
    fn defaults_and_forwarding() {
        let values = json!({});
        let res = operation_params(&values, &[10]).unwrap();
        assert_eq!(res["ids"], json!([10]));
        assert_eq!(res["mode"], "monochrome");
        assert_eq!(res["threshold"], 128);
        assert_eq!(res["invert"], false);
        assert_eq!(res["colors"], 8);
        assert_eq!(res["alphaCutoff"], 16);
        assert_eq!(res["resolution"], 256);
        assert_eq!(res["output"], "vectors");
    }

    #[test]
    fn color_mode_and_mask_output() {
        let values = json!({
            "mode": "color",
            "colors": 12,
            "output": "mask",
            "resolution": 512,
            "alphaCutoff": 32,
            "invert": true
        });
        let res = operation_params(&values, &[10]).unwrap();
        assert_eq!(res["mode"], "color");
        assert_eq!(res["colors"], 12);
        assert_eq!(res["output"], "mask");
        assert_eq!(res["resolution"], 512);
        assert_eq!(res["alphaCutoff"], 32);
        assert_eq!(res["invert"], true);
    }

    #[test]
    fn out_of_range_rejection() {
        assert!(operation_params(&json!({"colors": 1}), &[10]).is_err());
        assert!(operation_params(&json!({"colors": 17}), &[10]).is_err());
        assert!(operation_params(&json!({"alphaCutoff": 0}), &[10]).is_err());
        assert!(operation_params(&json!({"resolution": 15}), &[10]).is_err());
        assert!(operation_params(&json!({"resolution": 513}), &[10]).is_err());
        assert!(operation_params(&json!({"mode": "invalid"}), &[10]).is_err());
        assert!(operation_params(&json!({"output": "invalid"}), &[10]).is_err());
    }

    #[test]
    fn selection_bounds() {
        assert!(operation_params(&json!({}), &[]).is_err());
        let seventeen_ids: Vec<u64> = (1..=17).collect();
        assert!(operation_params(&json!({}), &seventeen_ids).is_err());
        let sixteen_ids: Vec<u64> = (1..=16).collect();
        assert!(operation_params(&json!({}), &sixteen_ids).is_ok());
    }
}
