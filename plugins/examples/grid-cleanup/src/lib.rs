use figcraft_plugin_sdk::{Event, Input, Result, Value, call, close_ui, entry, json, notify, selection, set_setting, setting, show_ui};

pub fn parse_step(val: Option<&Value>) -> Result<f64> {
    match val {
        Some(Value::String(s)) => match s.trim() {
            "4" => Ok(4.0),
            "8" => Ok(8.0),
            other => Err(format!("`step` must be 4 or 8 (got `{other}`)")),
        },
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_f64() {
                if (v - 4.0).abs() < 1e-6 {
                    Ok(4.0)
                } else if (v - 8.0).abs() < 1e-6 {
                    Ok(8.0)
                } else {
                    Err(format!("`step` must be 4 or 8 (got {v})"))
                }
            } else {
                Err("`step` must be a valid number".to_string())
            }
        }
        Some(Value::Null) | None => Ok(4.0),
        Some(_) => Err("`step` must be a string or number".to_string()),
    }
}

pub fn operation_params(values: &Value, ids: &[u64]) -> Result<Value> {
    let step = parse_step(values.get("step"))?;
    let descendants = values.get("descendants").and_then(Value::as_bool).unwrap_or(false);
    let positions = values.get("positions").and_then(Value::as_bool).unwrap_or(false);
    let dimensions = values.get("dimensions").and_then(Value::as_bool).unwrap_or(false);
    let padding = values.get("padding").and_then(Value::as_bool).unwrap_or(true);
    let gaps = values.get("gaps").and_then(Value::as_bool).unwrap_or(true);
    let radii = values.get("radii").and_then(Value::as_bool).unwrap_or(true);
    let bind_variables = values.get("bindVariables").and_then(Value::as_bool).unwrap_or(false);
    let replace_bindings = values.get("replaceBindings").and_then(Value::as_bool).unwrap_or(false);

    Ok(json!({
        "ids": ids,
        "step": step,
        "descendants": descendants,
        "properties": {
            "positions": positions,
            "dimensions": dimensions,
            "padding": padding,
            "gaps": gaps,
            "radii": radii,
        },
        "bindVariables": bind_variables,
        "replaceBindings": replace_bindings,
    }))
}

#[derive(Clone, Debug)]
struct PanelState {
    step: String,
    descendants: bool,
    positions: bool,
    dimensions: bool,
    padding: bool,
    gaps: bool,
    radii: bool,
    bind_variables: bool,
    replace_bindings: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            step: "4".to_string(),
            descendants: false,
            positions: false,
            dimensions: false,
            padding: true,
            gaps: true,
            radii: true,
            bind_variables: false,
            replace_bindings: false,
        }
    }
}

fn load_state() -> PanelState {
    let step = setting("step").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "4".into());
    let descendants = setting("descendants").and_then(|v| v.as_bool()).unwrap_or(false);
    let positions = setting("positions").and_then(|v| v.as_bool()).unwrap_or(false);
    let dimensions = setting("dimensions").and_then(|v| v.as_bool()).unwrap_or(false);
    let padding = setting("padding").and_then(|v| v.as_bool()).unwrap_or(true);
    let gaps = setting("gaps").and_then(|v| v.as_bool()).unwrap_or(true);
    let radii = setting("radii").and_then(|v| v.as_bool()).unwrap_or(true);
    let bind_variables = setting("bindVariables").and_then(|v| v.as_bool()).unwrap_or(false);
    let replace_bindings = setting("replaceBindings").and_then(|v| v.as_bool()).unwrap_or(false);

    PanelState { step, descendants, positions, dimensions, padding, gaps, radii, bind_variables, replace_bindings }
}

fn save_state(s: &PanelState) -> Result<()> {
    set_setting("step", &json!(s.step))?;
    set_setting("descendants", &json!(s.descendants))?;
    set_setting("positions", &json!(s.positions))?;
    set_setting("dimensions", &json!(s.dimensions))?;
    set_setting("padding", &json!(s.padding))?;
    set_setting("gaps", &json!(s.gaps))?;
    set_setting("radii", &json!(s.radii))?;
    set_setting("bindVariables", &json!(s.bind_variables))?;
    set_setting("replaceBindings", &json!(s.replace_bindings))?;
    Ok(())
}

fn show_panel(st: &PanelState) -> Result<Value> {
    show_ui(&json!({
        "title": "Grid Cleanup",
        "controls": [
            {
                "type": "list",
                "name": "step",
                "label": "Grid step",
                "options": ["4", "8"],
                "value": st.step
            },
            {
                "type": "boolean",
                "name": "descendants",
                "label": "Include descendants",
                "value": st.descendants
            },
            {
                "type": "boolean",
                "name": "positions",
                "label": "Positions (flow children protected)",
                "value": st.positions
            },
            {
                "type": "boolean",
                "name": "dimensions",
                "label": "Dimensions (Hug/Fill protected)",
                "value": st.dimensions
            },
            {
                "type": "boolean",
                "name": "padding",
                "label": "Padding",
                "value": st.padding
            },
            {
                "type": "boolean",
                "name": "gaps",
                "label": "Gaps (auto gaps protected)",
                "value": st.gaps
            },
            {
                "type": "boolean",
                "name": "radii",
                "label": "Corner radii",
                "value": st.radii
            },
            {
                "type": "boolean",
                "name": "bindVariables",
                "label": "Bind to owned variables",
                "value": st.bind_variables
            },
            {
                "type": "boolean",
                "name": "replaceBindings",
                "label": "Replace existing bindings",
                "value": st.replace_bindings
            },
            {
                "type": "note",
                "label": "Rounds values to nearest grid step. Selection scope only. Non-destructive auto layout protection."
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
    let mut state = load_state();

    if let Some(s) = e.values.get("step").and_then(Value::as_str) {
        state.step = s.to_string();
    }
    if let Some(b) = e.values.get("descendants").and_then(Value::as_bool) {
        state.descendants = b;
    }
    if let Some(b) = e.values.get("positions").and_then(Value::as_bool) {
        state.positions = b;
    }
    if let Some(b) = e.values.get("dimensions").and_then(Value::as_bool) {
        state.dimensions = b;
    }
    if let Some(b) = e.values.get("padding").and_then(Value::as_bool) {
        state.padding = b;
    }
    if let Some(b) = e.values.get("gaps").and_then(Value::as_bool) {
        state.gaps = b;
    }
    if let Some(b) = e.values.get("radii").and_then(Value::as_bool) {
        state.radii = b;
    }
    if let Some(b) = e.values.get("bindVariables").and_then(Value::as_bool) {
        state.bind_variables = b;
    }
    if let Some(b) = e.values.get("replaceBindings").and_then(Value::as_bool) {
        state.replace_bindings = b;
    }

    match e.control.as_str() {
        "apply" => {
            let current_sel = selection();
            let params = operation_params(&e.values, &current_sel)?;
            let report = call("arrange.snapToGrid", &params)?;
            save_state(&state)?;
            let changed = report.get("changed").and_then(Value::as_u64).unwrap_or(0);
            let skipped = report.get("skipped").and_then(Value::as_u64).unwrap_or(0);
            let protected = report.get("protectedFields").and_then(Value::as_u64).unwrap_or(0);
            notify(&format!("Changed {changed} layers (skipped {skipped}, {protected} protected fields)"));
            Ok(report)
        }
        "close" => {
            close_ui();
            Ok(Value::Null)
        }
        "step" | "descendants" | "positions" | "dimensions" | "padding" | "gaps" | "radii" | "bindVariables" | "replaceBindings" => {
            save_state(&state)?;
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
        let st = load_state();
        return show_panel(&st);
    }
    if input.command == "apply" {
        let current_sel = selection();
        let params = operation_params(&input.params, &current_sel)?;
        return call("arrange.snapToGrid", &params);
    }
    Err(format!("unknown command `{}`", input.command))
}

entry!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_contract() {
        let p = operation_params(&json!({}), &[10, 20]).unwrap();
        assert_eq!(p["ids"], json!([10, 20]));
        assert_eq!(p["step"], 4.0);
        assert_eq!(p["descendants"], false);
        assert_eq!(p["bindVariables"], false);
        assert_eq!(p["replaceBindings"], false);
        assert_eq!(p["properties"]["positions"], false);
        assert_eq!(p["properties"]["dimensions"], false);
        assert_eq!(p["properties"]["padding"], true);
        assert_eq!(p["properties"]["gaps"], true);
        assert_eq!(p["properties"]["radii"], true);
    }

    #[test]
    fn step_parsing() {
        assert_eq!(parse_step(Some(&json!("4"))).unwrap(), 4.0);
        assert_eq!(parse_step(Some(&json!("8"))).unwrap(), 8.0);
        assert_eq!(parse_step(Some(&json!(4))).unwrap(), 4.0);
        assert_eq!(parse_step(Some(&json!(8))).unwrap(), 8.0);
        assert_eq!(parse_step(None).unwrap(), 4.0);

        assert!(parse_step(Some(&json!("3"))).is_err());
        assert!(parse_step(Some(&json!(3))).is_err());
        assert!(parse_step(Some(&json!("unknown"))).is_err());
    }

    #[test]
    fn full_options_preserved() {
        let values = json!({
            "step": "8",
            "descendants": true,
            "positions": true,
            "dimensions": true,
            "padding": false,
            "gaps": false,
            "radii": false,
            "bindVariables": true,
            "replaceBindings": true,
        });
        let p = operation_params(&values, &[42]).unwrap();
        assert_eq!(p["step"], 8.0);
        assert_eq!(p["descendants"], true);
        assert_eq!(p["bindVariables"], true);
        assert_eq!(p["replaceBindings"], true);
        assert_eq!(p["properties"]["positions"], true);
        assert_eq!(p["properties"]["dimensions"], true);
        assert_eq!(p["properties"]["padding"], false);
        assert_eq!(p["properties"]["gaps"], false);
        assert_eq!(p["properties"]["radii"], false);
    }
}
