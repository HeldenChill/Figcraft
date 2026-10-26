use figcraft_plugin_sdk::{Event, Input, Result, Value, call, close_ui, entry, json, notify, selection, set_setting, setting, show_ui};

pub fn operation_params(values: &Value, ids: &[u64]) -> Result<Value> {
    let mode = match values.get("mode").and_then(Value::as_str) {
        Some(m) => {
            let lower = m.to_ascii_lowercase();
            if lower != "fit" && lower != "fill" {
                return Err(format!("`mode` must be \"fit\" or \"fill\" (got `{m}`)"));
            }
            lower
        }
        None => "fit".to_string(),
    };
    let descendants = values.get("descendants").and_then(Value::as_bool).unwrap_or(false);
    Ok(json!({
        "mode": mode,
        "descendants": descendants,
        "ids": ids,
    }))
}

fn show_panel(mode: &str, descendants: bool) -> Result<Value> {
    show_ui(&json!({
        "title": "Fit Images",
        "controls": [
            {
                "type": "list",
                "name": "mode",
                "label": "Mode",
                "options": ["fit", "fill"],
                "value": mode
            },
            {
                "type": "boolean",
                "name": "descendants",
                "label": "Include descendants",
                "value": descendants
            },
            {
                "type": "note",
                "label": "Changes image fills; layer sizes stay unchanged."
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
    let mode = e.values.get("mode").and_then(Value::as_str).unwrap_or("fit");
    let descendants = e.values.get("descendants").and_then(Value::as_bool).unwrap_or(false);

    match e.control.as_str() {
        "apply" => {
            let current_sel = selection();
            let params = operation_params(&e.values, &current_sel)?;
            let report = call("image.fit", &params)?;
            set_setting("mode", &json!(mode))?;
            set_setting("descendants", &json!(descendants))?;
            let changed = report.get("changed").and_then(Value::as_u64).unwrap_or(0);
            let skipped = report.get("skipped").and_then(Value::as_u64).unwrap_or(0);
            notify(&format!("Changed {changed} layers (skipped {skipped})"));
            Ok(report)
        }
        "close" => {
            close_ui();
            Ok(Value::Null)
        }
        "mode" | "descendants" => {
            set_setting("mode", &json!(mode))?;
            set_setting("descendants", &json!(descendants))?;
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
        let saved_mode = setting("mode").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "fit".into());
        let saved_descendants = setting("descendants").and_then(|v| v.as_bool()).unwrap_or(false);
        return show_panel(&saved_mode, saved_descendants);
    }
    if input.command == "apply" {
        let current_sel = selection();
        let params = operation_params(&input.params, &current_sel)?;
        return call("image.fit", &params);
    }
    Err(format!("unknown command `{}`", input.command))
}

entry!(run);

#[cfg(test)]
mod tests {
    use super::operation_params;
    use serde_json::json;

    #[test]
    fn params_default_to_fit() {
        let p = operation_params(&json!({}), &[12]).unwrap();
        assert_eq!(p["mode"], "fit");
        assert_eq!(p["descendants"], false);
        assert_eq!(p["ids"], json!([12]));
    }

    #[test]
    fn params_keep_current_selection() {
        let p = operation_params(&json!({"mode": "fill", "descendants": true}), &[42, 99]).unwrap();
        assert_eq!(p["mode"], "fill");
        assert_eq!(p["descendants"], true);
        assert_eq!(p["ids"], json!([42, 99]));
    }

    #[test]
    fn params_reject_invalid_mode() {
        let err = operation_params(&json!({"mode": "stretch"}), &[12]).unwrap_err();
        assert!(err.contains("mode"), "error should mention mode: {err}");
    }
}
