//! Discovery-only responses and an opt-in audit: a catalogue test can prove
//! no thread, turn, setter or sign-in request was sent.

use serde_json::{Value, json};
use std::io::Write as _;

pub(super) fn record(method: &str) -> std::io::Result<()> {
    if let Some(path) = std::env::var_os("FAKE_CODEX_CATALOGUE_AUDIT") {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{method}")?;
    }
    Ok(())
}

pub(super) fn response(method: &str, params: &Value) -> Value {
    match (method, std::env::var("FAKE_CODEX_CATALOGUE").as_deref()) {
        ("model/list", Ok("malformed")) => json!({"data": [{"displayName": "missing id"}]}),
        ("model/list", _) => {
            if params.get("cursor").and_then(Value::as_str) == Some("second") {
                json!({"data": [{"id": "second", "displayName": "Second", "defaultReasoningEffort": "low", "supportedReasoningEfforts": [{"reasoningEffort": "low"}]}]})
            } else {
                json!({"data": [{"id": "first", "displayName": "First", "isDefault": true, "defaultReasoningEffort": "high", "supportedReasoningEfforts": [{"reasoningEffort": "high"}]}], "nextCursor": "second"})
            }
        }
        ("getUserSavedConfig", _) => json!({"config": {"model": "second"}}),
        ("config/read", _) => json!({"config": {}}),
        _ => Value::Null,
    }
}
