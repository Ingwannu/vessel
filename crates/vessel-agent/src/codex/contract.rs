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
//!   fails on any difference.
//! - The second runs the real `codex`, which AGENTS.md section 8 keeps out
//!   of tests: it is the one exception, opt-in ([`OPT_IN`], set only by
//!   the gate's own step, AGENTS.md section 1), so a person who upgrades
//!   Codex does not fail `cargo test` for unrelated work. It generates a
//!   schema and nothing else: no session, no network, an empty
//!   environment but `PATH`, a temporary `HOME` and `CODEX_HOME`, output
//!   in a temporary directory. Without the variable it says it skipped, on
//!   the test's standard error, where a run shows it.
//! - Cost: a Codex release that changes a documentation string in one of
//!   these definitions fails the second test too. That is the point: the
//!   person upgrading reads what changed and regenerates the file.

use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// The definitions the driver follows (Codex 0.166.0).
const SCHEMA: &str = include_str!("schema/app-server.json");

/// The oldest Codex these definitions were checked against.
const CHECKED: (u64, u64, u64) = (0, 166, 0);

/// Set (to anything) to regenerate the definitions from the installed
/// Codex: the gate's own step does (AGENTS.md section 1).
const OPT_IN: &str = "VESSEL_CODEX_CONTRACT";

/// Each definition the driver reads or writes, and the properties of it
/// (or of one of its variants) it uses.
const FIELDS: &[(&str, &[&str])] = &[
    ("InitializeParams", &["clientInfo", "capabilities"]),
    ("ClientInfo", &["name", "title", "version"]),
    ("InitializeCapabilities", &["experimentalApi"]),
    ("InitializeResponse", &["userAgent"]),
    (
        "ThreadStartParams",
        &[
            "cwd",
            "model",
            "approvalPolicy",
            "approvalsReviewer",
            "sandbox",
        ],
    ),
    (
        "ThreadResumeParams",
        &[
            "threadId",
            "cwd",
            "model",
            "approvalPolicy",
            "approvalsReviewer",
            "sandbox",
            "excludeTurns",
        ],
    ),
    ("ThreadStartResponse", &["thread"]),
    ("ThreadResumeResponse", &["thread"]),
    ("Thread", &["id"]),
    (
        "TurnStartParams",
        &[
            "threadId",
            "input",
            "approvalPolicy",
            "approvalsReviewer",
            "sandboxPolicy",
        ],
    ),
    ("SandboxPolicy", &["type", "networkAccess"]),
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
    (
        "CommandExecutionRequestApprovalParams",
        &["itemId", "command", "cwd", "availableDecisions"],
    ),
    (
        "CommandExecutionApprovalDecision",
        &[
            "acceptWithExecpolicyAmendment",
            "applyNetworkPolicyAmendment",
        ],
    ),
    ("CommandExecutionRequestApprovalResponse", &["decision"]),
    ("FileChangeRequestApprovalParams", &["itemId", "reason"]),
    ("FileChangeRequestApprovalResponse", &["decision"]),
    (
        "PermissionsRequestApprovalParams",
        &["itemId", "permissions", "reason"],
    ),
    (
        "PermissionsRequestApprovalResponse",
        &["permissions", "scope"],
    ),
    // Granted as asked: the two profiles must keep one shape (checked
    // below as well).
    ("RequestPermissionProfile", &["fileSystem", "network"]),
    ("GrantedPermissionProfile", &["fileSystem", "network"]),
    ("ToolRequestUserInputParams", &["itemId", "questions"]),
    (
        "ToolRequestUserInputQuestion",
        &["id", "header", "question", "isOther", "isSecret", "options"],
    ),
    ("ToolRequestUserInputOption", &["label", "description"]),
    ("ToolRequestUserInputResponse", &["answers"]),
    ("ToolRequestUserInputAnswer", &["answers"]),
    (
        "McpServerElicitationRequestParams",
        &["mode", "message", "requestedSchema"],
    ),
    (
        "McpServerElicitationRequestResponse",
        &["action", "content"],
    ),
    ("ServerRequestResolvedNotification", &["requestId"]),
];

/// Each definition whose values the driver names, and those values.
const VALUES: &[(&str, &[&str])] = &[
    ("AskForApproval", &["on-request", "never"]),
    ("ApprovalsReviewer", &["user", "auto_review"]),
    (
        "SandboxMode",
        &["read-only", "workspace-write", "danger-full-access"],
    ),
    (
        "SandboxPolicy",
        &["readOnly", "workspaceWrite", "dangerFullAccess"],
    ),
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
    (
        "CommandExecutionApprovalDecision",
        &["accept", "acceptForSession", "decline", "cancel"],
    ),
    (
        "FileChangeApprovalDecision",
        &["accept", "acceptForSession", "decline", "cancel"],
    ),
    ("PermissionGrantScope", &["turn", "session"]),
    (
        "McpServerElicitationAction",
        &["accept", "decline", "cancel"],
    ),
    ("McpServerElicitationRequestParams", &["form", "url"]),
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

/// More permissions are granted by echoing what was asked: the requested
/// and the granted profile must have the same properties, of the same
/// types.
#[test]
fn a_permission_is_granted_in_the_shape_it_was_asked() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let definitions = definitions(&schema);
    let asked = &definitions["RequestPermissionProfile"]["properties"];
    assert!(asked.is_object());
    assert_eq!(
        asked,
        &definitions["GrantedPermissionProfile"]["properties"]
    );
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
    if std::env::var_os(OPT_IN).is_none() {
        let _shown = writeln!(
            std::io::stderr(),
            "codex contract: {OPT_IN} is unset, the schema is not regenerated from the \
             installed codex"
        );
        return;
    }
    // An empty environment but `PATH` (an npm-installed codex is a script
    // its `node` runs), and homes of the test's own.
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join("codex");
    std::fs::create_dir(&codex_home).unwrap();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let codex = || {
        let mut command = Command::new("codex");
        let _set = command
            .env_clear()
            .env("PATH", &path)
            .env("HOME", home.path())
            .env("CODEX_HOME", &codex_home);
        command
    };
    let output = codex()
        .arg("--version")
        .output()
        .expect("the gate's contract step needs codex on PATH");
    let line = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let installed = version(&line);
    assert!(
        installed.is_some_and(|installed| installed >= CHECKED),
        "the contract step needs a codex of at least {}.{}.{}: {line}",
        CHECKED.0,
        CHECKED.1,
        CHECKED.2
    );
    let out = tempfile::tempdir().unwrap();
    let generated = codex()
        .args([
            "app-server",
            "generate-json-schema",
            "--experimental",
            "--out",
        ])
        .arg(out.path())
        .status()
        .unwrap();
    assert!(generated.success(), "codex could not generate its schema");
    let fresh = read(&out.path().join("codex_app_server_protocol.schemas.json"));
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
