//! The Claude Code driver's contract with the Agent SDK's declared types
//! (ADR 0057): every frame type, field and literal value `wire` reads or
//! writes is declared by the SDK version the driver follows.
//!
//! [Decision Log]
//! - Purpose: the control frames are the SDK's internal wire, not
//!   documented; a field renamed under the driver must fail here, not read
//!   as absent in a session. The documented frames are checked the same
//!   way, since one test costs nothing more.
//! - Choice: the SDK's `sdk.d.ts` is Anthropic's, all rights reserved, so
//!   nothing of it is copied into this public repository. The lists below
//!   are our own (the names the driver uses), and the test reads the
//!   installed SDK's file, named by `VESSEL_CLAUDE_SDK_TYPES`, checking
//!   each name inside its type's declaration, and the SDK's
//!   `claudeCodeVersion` against the version `vessel_harness::direct`
//!   says the driver was checked against.
//! - Ignored by default (it needs the SDK installed, which a CI runner has
//!   not), so a run without it says so instead of passing silently:
//!   `VESSEL_CLAUDE_SDK_TYPES=<…/@anthropic-ai/claude-agent-sdk/sdk.d.ts>
//!   cargo test -p vessel-agent claude::contract -- --ignored`. Run it
//!   before changing `wire` or the version followed.

use std::path::PathBuf;

/// Each declared type the driver reads or writes, and the fields of it
/// (or of one of its variants) it uses.
const FIELDS: &[(&str, &[&str])] = &[
    ("SDKControlRequest", &["type", "request_id", "request"]),
    ("SDKControlInitializeRequest", &["subtype"]),
    ("SDKControlInterruptRequest", &["subtype"]),
    (
        "SDKControlPermissionRequest",
        &[
            "subtype",
            "tool_name",
            "input",
            "permission_suggestions",
            "tool_use_id",
            "title",
        ],
    ),
    ("SDKControlResponse", &["type", "response"]),
    ("ControlResponse", &["subtype", "request_id", "response"]),
    ("ControlErrorResponse", &["subtype", "request_id", "error"]),
    ("SDKControlCancelRequest", &["type", "request_id"]),
    ("SDKKeepAliveMessage", &["type"]),
    (
        "PermissionResult",
        &["behavior", "updatedInput", "toolUseID", "message"],
    ),
    (
        "SDKUserMessage",
        &[
            "type",
            "message",
            "parent_tool_use_id",
            "session_id",
            "isSynthetic",
        ],
    ),
    (
        "SDKAssistantMessage",
        &["type", "message", "parent_tool_use_id"],
    ),
    (
        "SDKPartialAssistantMessage",
        &["type", "event", "parent_tool_use_id"],
    ),
    (
        "SDKResultSuccess",
        &[
            "type",
            "subtype",
            "is_error",
            "result",
            "stop_reason",
            "usage",
            "session_id",
        ],
    ),
    (
        "SDKResultError",
        &["type", "subtype", "is_error", "errors", "usage"],
    ),
    (
        "SDKSystemMessage",
        &[
            "type",
            "subtype",
            "session_id",
            "claude_code_version",
            "model",
            "permissionMode",
            "slash_commands",
            "capabilities",
        ],
    ),
];

/// Each declared type whose literal values the driver names, and those
/// values.
const LITERALS: &[(&str, &[&str])] = &[
    ("SDKControlRequest", &["control_request"]),
    ("SDKControlInitializeRequest", &["initialize"]),
    ("SDKControlInterruptRequest", &["interrupt"]),
    ("SDKControlPermissionRequest", &["can_use_tool"]),
    ("SDKControlResponse", &["control_response"]),
    ("ControlResponse", &["success"]),
    ("ControlErrorResponse", &["error"]),
    ("SDKControlCancelRequest", &["control_cancel_request"]),
    ("SDKKeepAliveMessage", &["keep_alive"]),
    ("PermissionResult", &["allow", "deny"]),
    ("SDKPartialAssistantMessage", &["stream_event"]),
    ("SDKResultSuccess", &["result", "success"]),
    ("SDKSystemMessage", &["system", "init"]),
];

/// `name`'s declaration in `types`: from `type <name> =` to the `};` that
/// closes it at the start of a line, without its comments.
fn declaration(types: &str, name: &str) -> Option<String> {
    let start = types.find(&format!("type {name} = "))?;
    let rest = types.get(start..)?;
    let end = rest
        .find("\n};")
        .map_or(rest.len(), |end| end.saturating_add(3));
    let block = rest.get(..end)?;
    Some(
        block
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                !(line.starts_with('*') || line.starts_with("/*") || line.starts_with("//"))
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// Whether `block` declares field `field` (optional or not).
fn declares(block: &str, field: &str) -> bool {
    block.lines().any(|line| {
        let line = line.trim_start();
        line.strip_prefix(field)
            .is_some_and(|rest| rest.starts_with(':') || rest.starts_with("?:"))
    })
}

/// The installed SDK's declarations, named by `VESSEL_CLAUDE_SDK_TYPES`.
fn installed() -> (PathBuf, String) {
    let path = std::env::var_os("VESSEL_CLAUDE_SDK_TYPES").map(PathBuf::from).expect(
        "VESSEL_CLAUDE_SDK_TYPES names the installed Agent SDK's sdk.d.ts (see the module docs)",
    );
    let types = std::fs::read_to_string(&path).unwrap();
    (path, types)
}

#[test]
#[ignore = "needs the Agent SDK installed: VESSEL_CLAUDE_SDK_TYPES=<its sdk.d.ts>, then --ignored"]
fn every_frame_field_and_value_the_driver_uses_is_declared_by_the_sdk() {
    let (_, types) = installed();
    for (name, fields) in FIELDS {
        let block = declaration(&types, name).unwrap_or_else(|| panic!("no type {name}"));
        for field in *fields {
            assert!(declares(&block, field), "{name} has no `{field}`");
        }
    }
    for (name, values) in LITERALS {
        let block = declaration(&types, name).unwrap_or_else(|| panic!("no type {name}"));
        for value in *values {
            assert!(
                block.contains(&format!("'{value}'")),
                "{name} has no value `{value}`"
            );
        }
    }
}

#[test]
#[ignore = "needs the Agent SDK installed: VESSEL_CLAUDE_SDK_TYPES=<its sdk.d.ts>, then --ignored"]
fn the_sdk_speaks_to_the_claude_code_the_driver_was_checked_against() {
    let (path, _) = installed();
    let manifest = path.parent().map(|dir| dir.join("package.json")).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    let follows = manifest["claudeCodeVersion"].as_str().unwrap();
    let direct = vessel_harness::direct(vessel_proto::HarnessKind::ClaudeCode).unwrap();
    assert_eq!(
        follows,
        direct.newest_checked.to_string(),
        "the installed SDK follows another Claude Code: read what changed in its types, adjust \
         `wire`, then the version in vessel_harness::direct"
    );
}

#[test]
fn a_declaration_is_read_without_its_comments() {
    let types = "declare type A = {\n    /**\n     * x: not a field\n     */\n    \
                 subtype: 'a';\n    b?: string;\n};\ndeclare type B = {\n    c: 1;\n};\n";
    let block = declaration(types, "A").unwrap();
    assert!(declares(&block, "subtype") && declares(&block, "b"));
    assert!(!declares(&block, "x") && !declares(&block, "c"));
    assert!(block.contains("'a'"));
    assert!(declaration(types, "C").is_none());
}
