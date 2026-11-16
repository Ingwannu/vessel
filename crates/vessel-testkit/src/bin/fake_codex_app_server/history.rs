//! Fake native history pages and a method-only audit, never provider data.
use serde_json::{Value, json};
use std::io::Write as _;
pub(super) fn audit(method: Option<&str>) {
    let Ok(path) = std::env::var("FAKE_CODEX_HISTORY_AUDIT") else {
        return;
    };
    if let (Some(method), Ok(mut file)) = (
        method,
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path),
    ) {
        let _written = writeln!(file, "{method}");
    }
}
pub(super) fn page(id: &Value, params: &Value) -> Value {
    let mode = std::env::var("FAKE_CODEX_HISTORY").unwrap_or_default();
    if mode == "unsupported" {
        return json!({"id":id,"error":{"code":-32601,"message":"not found"}});
    }
    if mode == "invalid" {
        return json!({"id":id,"result":{"data":"invalid"}});
    }
    if params.get("cursor").and_then(Value::as_str).is_none() {
        return json!({"id":id,"result":{"data":[],"nextCursor":"native-second"}});
    }
    let directory = params
        .get("cwd")
        .cloned()
        .unwrap_or_else(|| json!(std::env::temp_dir()));
    json!({"id":id,"result":{"data":[
        {"id":"native-actual","cwd":directory,"name":"Native title","updatedAt":123},
        {"id":"unknown-date","cwd":directory,"preview":"Preview"},
        {"id":"unknown-directory"}
    ],"nextCursor":null}})
}
