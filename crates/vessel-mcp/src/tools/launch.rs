//! Launch-only metadata uses existing typed session calls; no provider guessing.
use super::{ToolError, text};
use rmcp::model::JsonObject;
use serde_json::{Value, json};
use vessel_proto::{OptionChoice, SessionMode, SessionSpec};

const MAX_OPTIONS: usize = 128;

pub(super) fn configure(
    mut spec: SessionSpec,
    args: &JsonObject,
) -> Result<SessionSpec, ToolError> {
    if matches!(spec.mode, SessionMode::Terminal { .. }) && text(args, "model")?.is_some() {
        return Err(ToolError::Arguments(
            "terminal sessions do not accept a structured model choice".into(),
        ));
    }
    if let Some(title) = text(args, "title")? {
        spec = spec.with_title(
            title
                .parse()
                .map_err(|_bad| ToolError::Arguments("invalid session title".into()))?,
        );
    }
    if let Some(labels) = args.get("labels").filter(|value| !value.is_null()) {
        spec = spec.with_labels(
            serde_json::from_value(labels.clone())
                .map_err(|_bad| ToolError::Arguments("invalid session label map".into()))?,
        );
    }
    if let Some(value) = args.get("options").filter(|value| !value.is_null()) {
        let Some(items) = value.as_array().filter(|items| items.len() <= MAX_OPTIONS) else {
            return Err(ToolError::Arguments(format!(
                "`options` must contain at most {MAX_OPTIONS} choices"
            )));
        };
        let choices: Vec<OptionChoice> = items
            .iter()
            .map(|item| {
                let allowed = match item.get("kind").and_then(Value::as_str) {
                    Some("select") => ["id", "kind", "value"],
                    Some("toggle") => ["id", "kind", "on"],
                    _ => return Err(ToolError::Arguments("unknown launch option kind".into())),
                };
                if !item.as_object().is_some_and(|fields| {
                    fields.len() == allowed.len()
                        && fields.keys().all(|key| allowed.contains(&key.as_str()))
                }) {
                    return Err(ToolError::Arguments(
                        "launch option contains missing or unexpected fields".into(),
                    ));
                }
                serde_json::from_value(item.clone())
                    .map_err(|_bad| ToolError::Arguments("invalid launch option choice".into()))
            })
            .collect::<Result<_, _>>()?;
        let mut ids = std::collections::HashSet::new();
        for choice in &choices {
            if choice.id.as_str().is_empty() || !ids.insert(choice.id.clone()) {
                return Err(ToolError::Arguments(
                    "launch option IDs must be nonempty and unique".into(),
                ));
            }
        }
        match &mut spec.mode {
            SessionMode::Structured { options, .. } => *options = choices,
            SessionMode::Terminal { .. } => {
                return Err(ToolError::Arguments(
                    "terminal sessions have no structured launch options".into(),
                ));
            }
        }
    }
    Ok(spec)
}

pub(super) fn options_schema() -> Value {
    json!({"type":"array","maxItems":MAX_OPTIONS,"description":"Exact choices discovered through harness_options; no inferred IDs.","items":{"oneOf":[
        {"type":"object","properties":{"id":{"type":"string","minLength":1},"kind":{"const":"select"},"value":{"type":"string"}},"required":["id","kind","value"],"additionalProperties":false},
        {"type":"object","properties":{"id":{"type":"string","minLength":1},"kind":{"const":"toggle"},"on":{"type":"boolean"}},"required":["id","kind","on"],"additionalProperties":false}
    ]}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_is_typed_and_duplicate_or_terminal_choices_refuse() {
        let spec = SessionSpec::new(
            vessel_proto::HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let args = json!({"title":"worker","labels":{"purpose":"audit"},"options":[{"id":"mode","kind":"select","value":"plan"}]});
        let selected = configure(spec.clone(), args.as_object().unwrap()).unwrap();
        assert_eq!(selected.title.unwrap().as_str(), "worker");
        assert!(!selected.labels.is_empty());
        for args in [
            json!({"labels":{"vessel.private":"bad"}}),
            json!({"options":[{"id":"mode","kind":"toggle","on":true,"value":"plan"}]}),
            json!({"options":[{"id":"mode","kind":"toggle","on":true},{"id":"mode","kind":"toggle","on":false}]}),
            json!({"options":[{"id":"","kind":"toggle","on":true}]}),
        ] {
            assert!(configure(spec.clone(), args.as_object().unwrap()).is_err());
        }
        let mut terminal = spec;
        terminal.mode = SessionMode::Terminal {
            args: Vec::new(),
            size: vessel_proto::TerminalSize::default(),
        };
        assert!(configure(terminal, args.as_object().unwrap()).is_err());
    }
}
