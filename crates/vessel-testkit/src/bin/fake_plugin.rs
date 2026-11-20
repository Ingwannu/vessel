//! `fake-plugin`: a Vessel plugin helper (helper protocol 1) that keeps one
//! settings document for the node's settings tests (ADR 0115, "Settings").
//! It speaks the length-prefixed JSON frames of `vessel-plugin`'s pipe in
//! plain JSON, as the other fakes keep no wire dependency.
//!
//! - `ready` declares the document `display` at version 2 and no methods.
//! - `parse`: the schema is `{color: string = "blue", size: whole number =
//!   1}`. Another key or another type is refused with words. A document
//!   holding `"boom"` is refused with words that hold control characters and
//!   run past the node's bound, so tests see the node cut them.
//! - `migrate` from version 1 renames `colour` to `color`, then parses.
//! - `changed`: each notice is appended as one JSON line to
//!   `notices.jsonl` in its working directory (the plugin's folder), so a
//!   test can read what the helper was told.
//! - Any other call is refused; `shutdown` or the end of input ends it.

use std::io::{self, Read, Write};

use serde_json::{Map, Value, json};

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    while let Some(message) = read_frame(&mut input)? {
        match message.get("type").and_then(Value::as_str) {
            Some("initialize") => write_frame(&mut output, &ready())?,
            Some("request") => {
                let id = message.get("request").cloned().unwrap_or(Value::Null);
                let reply = match message.get("call").and_then(settings_step) {
                    Some(step) => json!({"type": "result", "request": id, "output": answer(step)?}),
                    None => json!({
                        "type": "error",
                        "request": id,
                        "kind": "refused",
                        "diagnostic": "fake-plugin answers settings only",
                    }),
                };
                write_frame(&mut output, &reply)?;
            }
            Some("shutdown") => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

fn ready() -> Value {
    json!({
        "type": "ready",
        "protocol": 1,
        "client": {"methods": [], "contributions": [], "themes": []},
        "hooks": [],
        "settings": [{"id": "display", "version": 2}],
    })
}

/// The step of a `display` settings call.
fn settings_step(call: &Value) -> Option<&Value> {
    (call.get("type").and_then(Value::as_str) == Some("settings")
        && call.get("settings").and_then(Value::as_str) == Some("display"))
    .then(|| call.get("step"))
    .flatten()
}

fn answer(step: &Value) -> io::Result<Value> {
    let values = step.get("values").cloned().unwrap_or_else(|| json!({}));
    match step.get("step").and_then(Value::as_str) {
        Some("parse") => Ok(parse(&values)),
        Some("migrate") => Ok(match step.get("from").and_then(Value::as_u64) {
            Some(1) => parse(&rename_colour(values)),
            _ => invalid("no migration from that version"),
        }),
        Some("changed") => {
            let mut notices = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("notices.jsonl")?;
            let state = step.get("state").cloned().unwrap_or(Value::Null);
            writeln!(notices, "{state}")?;
            Ok(json!({}))
        }
        _ => Ok(invalid("unknown step")),
    }
}

/// Version 1 called it `colour`.
fn rename_colour(values: Value) -> Value {
    let Value::Object(mut object) = values else {
        return values;
    };
    if let Some(colour) = object.remove("colour") {
        let _replaced = object.insert("color".to_owned(), colour);
    }
    Value::Object(object)
}

fn parse(values: &Value) -> Value {
    let Some(object) = values.as_object() else {
        return invalid("settings must be an object");
    };
    if object.contains_key("boom") {
        let words = format!("boom\u{7}\n{}", "x".repeat(2000));
        return invalid(&words);
    }
    let mut parsed = Map::new();
    for (key, value) in object {
        let fits = match key.as_str() {
            "color" => value.is_string(),
            "size" => value.is_u64(),
            _ => return invalid(&format!("unknown setting: {key}")),
        };
        if !fits {
            return invalid(&format!("{key} has the wrong type"));
        }
        let _none = parsed.insert(key.clone(), value.clone());
    }
    let _color = parsed.entry("color").or_insert_with(|| json!("blue"));
    let _size = parsed.entry("size").or_insert_with(|| json!(1));
    json!({"verdict": "valid", "values": Value::Object(parsed)})
}

fn invalid(message: &str) -> Value {
    json!({"verdict": "invalid", "message": message})
}

/// One frame: a big-endian u32 length, then that many bytes of JSON.
fn read_frame(input: &mut impl Read) -> io::Result<Option<Value>> {
    let mut length = [0_u8; 4];
    match input.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = usize::try_from(u32::from_be_bytes(length)).map_err(io::Error::other)?;
    let mut body = vec![0_u8; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(io::Error::other)
}

fn write_frame(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(io::Error::other)?;
    let length = u32::try_from(body.len()).map_err(io::Error::other)?;
    output.write_all(&length.to_be_bytes())?;
    output.write_all(&body)?;
    output.flush()
}
