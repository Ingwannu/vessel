//! The Codex driver's contract with the app-server (ADR 0057): every field
//! and value `protocol` reads or writes is in `schema/app-server.json`,
//! and that file is what an installed Codex generates.
//!
//! [Decision Log]
//! - Purpose: Codex's protocol moves between releases (0.166.0 dropped
//!   `thread/rollback`). A field renamed under the driver must fail here,
//!   not read as absent in a session.
//! - Choice: the definitions the driver uses, as Codex 0.166.0's
//!   `generate-json-schema --experimental` wrote them, are checked in. One
//!   test checks the driver's fields against that file (always run); one
//!   regenerates the file's definitions from the `codex` on `PATH` and
//!   fails on any difference (run when that Codex is at least the oldest
//!   version the driver was checked against, said and skipped otherwise).
//! - Cost: a Codex release that changes a documentation string in one of
//!   these definitions fails the second test too. That is the point: the
//!   person upgrading reads what changed and regenerates the file.

use std::path::Path;

use serde_json::Value;

/// The definitions the driver follows (Codex 0.166.0).
const SCHEMA: &str = include_str!("schema/app-server.json");

/// The oldest Codex these definitions were checked against.
const CHECKED: (u64, u64, u64) = (0, 166, 0);

/// Each definition the driver reads or writes, and the properties of it
/// (or of one of its variants) it uses.
const FIELDS: &[(&str, &[&str])] = &[
    ("InitializeParams", &["clientInfo", "capabilities"]),
    ("ClientInfo", &["name", "title", "version"]),
    ("InitializeCapabilities", &["experimentalApi"]),
    ("InitializeResponse", &["userAgent"]),
    (
        "ThreadStartParams",
        &["cwd", "model", "approvalPolicy", "sandbox"],
    ),
    (
        "ThreadResumeParams",
        &[
            "threadId",
            "cwd",
            "model",
            "approvalPolicy",
            "sandbox",
            "excludeTurns",
        ],
    ),
    ("ThreadStartResponse", &["thread"]),
    ("ThreadResumeResponse", &["thread"]),
    ("Thread", &["id"]),
    ("TurnStartParams", &["threadId", "input"]),
    ("TurnStartResponse", &["turn"]),
    ("TurnInterruptParams", &["threadId", "turnId"]),
    ("TurnSteerParams", &["threadId", "expectedTurnId", "input"]),
    ("Turn", &["id", "status", "error"]),
    ("TurnError", &["message"]),
    ("UserInput", &["type", "text", "name", "path"]),
    ("TurnStartedNotification", &["threadId", "turn"]),
    ("TurnCompletedNotification", &["threadId", "turn"]),
    ("ItemStartedNotification", &["threadId", "item"]),
    ("ItemCompletedNotification", &["threadId", "item"]),
    (
        "AgentMessageDeltaNotification",
        &["threadId", "itemId", "delta"],
    ),
    (
        "ReasoningSummaryTextDeltaNotification",
        &["threadId", "itemId", "delta"],
    ),
    ("TurnPlanUpdatedNotification", &["threadId", "plan"]),
    ("TurnPlanStep", &["step", "status"]),
    (
        "ThreadTokenUsageUpdatedNotification",
        &["threadId", "tokenUsage"],
    ),
    ("ThreadTokenUsage", &["total", "last", "modelContextWindow"]),
    (
        "TokenUsageBreakdown",
        &[
            "inputTokens",
            "cachedInputTokens",
            "outputTokens",
            "reasoningOutputTokens",
            "totalTokens",
        ],
    ),
    ("ErrorNotification", &["threadId", "error", "willRetry"]),
    (
        "ThreadItem",
        &[
            "type",
            "id",
            "text",
            "summary",
            "command",
            "cwd",
            "status",
            "aggregatedOutput",
            "exitCode",
            "changes",
            "server",
            "tool",
            "arguments",
            "query",
        ],
    ),
    ("FileUpdateChange", &["path", "diff"]),
    ("GrantedPermissionProfile", &[]),
];

/// Each definition whose values the driver names, and those values.
const VALUES: &[(&str, &[&str])] = &[
    ("AskForApproval", &["on-request"]),
    ("SandboxMode", &["workspace-write"]),
    (
        "TurnStatus",
        &["completed", "interrupted", "failed", "inProgress"],
    ),
    (
        "TurnPlanStepStatus",
        &["pending", "inProgress", "completed"],
    ),
    (
        "CommandExecutionStatus",
        &["inProgress", "completed", "failed", "declined"],
    ),
    (
        "PatchApplyStatus",
        &["inProgress", "completed", "failed", "declined"],
    ),
    ("McpToolCallStatus", &["inProgress", "completed", "failed"]),
    ("CommandExecutionApprovalDecision", &["decline"]),
    ("FileChangeApprovalDecision", &["decline"]),
    ("McpServerElicitationAction", &["decline"]),
    (
        "ThreadItem",
        &[
            "agentMessage",
            "reasoning",
            "commandExecution",
            "fileChange",
            "mcpToolCall",
            "webSearch",
        ],
    ),
    ("UserInput", &["text", "mention"]),
];

fn definitions(schema: &Value) -> &serde_json::Map<String, Value> {
    schema["definitions"]
        .as_object()
        .expect("a schema has definitions")
}

/// Whether `definition`, or one of its variants, has property `name`.
fn has_property(definition: &Value, name: &str) -> bool {
    if definition["properties"].get(name).is_some() {
        return true;
    }
    ["oneOf", "anyOf", "allOf"].iter().any(|key| {
        definition[*key]
            .as_array()
            .is_some_and(|variants| variants.iter().any(|variant| has_property(variant, name)))
    })
}

/// Every string an `enum` anywhere in `value` allows.
fn allowed(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(Value::Array(values)) = fields.get("enum") {
                into.extend(values.iter().filter_map(Value::as_str).map(str::to_owned));
            }
            fields.values().for_each(|field| allowed(field, into));
        }
        Value::Array(items) => items.iter().for_each(|item| allowed(item, into)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[test]
fn every_field_and_value_the_driver_uses_is_in_the_checked_in_schema() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let definitions = definitions(&schema);
    for (name, properties) in FIELDS {
        let definition = definitions
            .get(*name)
            .unwrap_or_else(|| panic!("{name} is not in the checked-in schema"));
        for property in *properties {
            assert!(
                has_property(definition, property),
                "{name} has no `{property}`"
            );
        }
    }
    for (name, values) in VALUES {
        let mut found = Vec::new();
        allowed(&definitions[*name], &mut found);
        for value in *values {
            assert!(
                found.iter().any(|one| one == value),
                "{name} has no `{value}`"
            );
        }
    }
}

/// The `x.y.z` of `codex --version`'s line.
fn version(line: &str) -> Option<(u64, u64, u64)> {
    let word = line.split_whitespace().last()?;
    let mut parts = word.split('.').map(|part| {
        part.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<u64>()
    });
    Some((
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

#[test]
fn the_checked_in_schema_is_what_an_installed_codex_generates() {
    let Ok(output) = std::process::Command::new("codex")
        .arg("--version")
        .output()
    else {
        tracing::info!("no codex on PATH: the schema is not regenerated");
        return;
    };
    let line = String::from_utf8_lossy(&output.stdout);
    let Some(installed) = version(line.trim()).filter(|installed| *installed >= CHECKED) else {
        tracing::info!(
            installed = %line.trim(),
            "codex is older than the driver's schema: not regenerated"
        );
        return;
    };
    let out = std::env::temp_dir().join(format!(
        "vessel-codex-schema-{}-{}",
        std::process::id(),
        installed.2
    ));
    let generated = std::process::Command::new("codex")
        .args([
            "app-server",
            "generate-json-schema",
            "--experimental",
            "--out",
        ])
        .arg(&out)
        .status()
        .unwrap();
    assert!(generated.success(), "codex could not generate its schema");
    let fresh = read(&out.join("codex_app_server_protocol.schemas.json"));
    drop(std::fs::remove_dir_all(&out));
    let checked: Value = serde_json::from_str(SCHEMA).unwrap();
    let all = definitions(&fresh);
    let v2 = all.get("v2").and_then(Value::as_object);
    for (name, definition) in definitions(&checked) {
        let now = all
            .get(name)
            .or_else(|| v2.and_then(|v2| v2.get(name)))
            .unwrap_or_else(|| panic!("codex {line} no longer has {name}"));
        assert_eq!(
            now, definition,
            "codex {line} changed {name}: read what changed, adjust the driver, regenerate \
             schema/app-server.json"
        );
    }
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_codex_version_is_its_lines_last_word() {
    assert_eq!(version("codex-cli 0.166.0"), Some((0, 166, 0)));
    assert_eq!(version("codex-cli 0.150.0-alpha.11"), Some((0, 150, 0)));
    assert_eq!(version("codex-cli"), None);
}
