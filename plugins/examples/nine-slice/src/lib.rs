use figcraft_plugin_sdk::{Event, Input, Result, Value, call, close_ui, entry, json, notify, selection, set_setting, setting, show_ui};

pub fn operation_params(values: &Value, ids: &[u64]) -> Result<Value> {
    if ids.is_empty() {
        return Err("No layer selected: please select an image rectangle".into());
    }
    if ids.len() != 1 {
        return Err("Exactly one image rectangle must be selected".into());
    }

    let mut out = serde_json::Map::new();
    out.insert("ids".into(), json!([ids[0]]));

    for &key in &["left", "right", "top", "bottom"] {
        if let Some(v) = values.get(key) {
            if v.is_null() {
                continue;
            }
            let num = match v.as_f64() {
                Some(n) => n,
                None => {
                    if let Some(s) = v.as_str() {
                        s.trim().trim_end_matches("px").parse::<f64>().map_err(|_| format!("`{key}` must be a number (got `{s}`)"))?
                    } else {
                        return Err(format!("`{key}` must be a number"));
                    }
                }
            };
            if num < 0.0 {
                return Err(format!("`{key}` must not be negative"));
            }
            if num > 0.0 {
                out.insert(key.into(), json!(num));
            }
        }
    }

    Ok(Value::Object(out))
}

fn show_panel(left: f64, right: f64, top: f64, bottom: f64) -> Result<Value> {
    show_ui(&json!({
        "title": "Nine Slice",
        "controls": [
            {
                "type": "number",
                "name": "left",
                "label": "Left (0 = automatic)",
                "value": left,
                "min": 0
            },
            {
                "type": "number",
                "name": "right",
                "label": "Right (0 = automatic)",
                "value": right,
                "min": 0
            },
            {
                "type": "number",
                "name": "top",
                "label": "Top (0 = automatic)",
                "value": top,
                "min": 0
            },
            {
                "type": "number",
                "name": "bottom",
                "label": "Bottom (0 = automatic)",
                "value": bottom,
                "min": 0
            },
            {
                "type": "note",
                "label": "Requires single image rectangle. Source preserved; fixed borders cannot resize below minimum."
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

fn on_event(e: Event) -> Result<Value> {
    let get_num = |k: &str| -> f64 { e.values.get(k).and_then(Value::as_f64).unwrap_or(0.0).max(0.0) };
    let left = get_num("left");
    let right = get_num("right");
    let top = get_num("top");
    let bottom = get_num("bottom");

    match e.control.as_str() {
        "apply" => {
            let current_sel = selection();
            let params = operation_params(&e.values, &current_sel)?;
            let report = call("image.nineSlice", &params)?;
            set_setting("left", &json!(left))?;
            set_setting("right", &json!(right))?;
            set_setting("top", &json!(top))?;
            set_setting("bottom", &json!(bottom))?;
            let created = report.get("created").and_then(Value::as_array).map_or(0, |a| a.len());
            notify(&format!("Created {created} nine-slice panel"));
            close_ui();
            Ok(report)
        }
        "close" => {
            close_ui();
            Ok(Value::Null)
        }
        "left" | "right" | "top" | "bottom" => {
            set_setting("left", &json!(left))?;
            set_setting("right", &json!(right))?;
            set_setting("top", &json!(top))?;
            set_setting("bottom", &json!(bottom))?;
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
        let read_val = |k: &str| -> f64 { setting(k).and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0) };
        return show_panel(read_val("left"), read_val("right"), read_val("top"), read_val("bottom"));
    }
    if input.command == "apply" {
        let ids = match input.params.get("ids").and_then(Value::as_array) {
            Some(arr) => arr.iter().filter_map(Value::as_u64).collect(),
            None => selection(),
        };
        let params = operation_params(&input.params, &ids)?;
        return call("image.nineSlice", &params);
    }
    Err(format!("unknown command `{}`", input.command))
}

entry!(run);

#[cfg(test)]
mod tests {
    use super::operation_params;
    use serde_json::json;

    #[test]
    fn zero_auto_omission() {
        let values = json!({
            "left": 0,
            "right": 0.0,
            "top": 0,
            "bottom": 0
        });
        let res = operation_params(&values, &[42]).unwrap();
        assert_eq!(res["ids"], json!([42]));
        assert!(res.get("left").is_none());
        assert!(res.get("right").is_none());
        assert!(res.get("top").is_none());
        assert!(res.get("bottom").is_none());
    }

    #[test]
    fn positive_inset_forwarding() {
        let values = json!({
            "left": 10.5,
            "right": 20.0,
            "top": 8.0,
            "bottom": 12.0
        });
        let res = operation_params(&values, &[42]).unwrap();
        assert_eq!(res["ids"], json!([42]));
        assert_eq!(res["left"], 10.5);
        assert_eq!(res["right"], 20.0);
        assert_eq!(res["top"], 8.0);
        assert_eq!(res["bottom"], 12.0);
    }

    #[test]
    fn invalid_negative_rejection() {
        let values = json!({ "left": -5.0 });
        assert!(operation_params(&values, &[42]).is_err());
    }

    #[test]
    fn exactly_one_selection_validation() {
        let values = json!({});
        assert!(operation_params(&values, &[]).is_err());
        assert!(operation_params(&values, &[1, 2]).is_err());
    }
}
