//! Codex approval decisions decoded at their provider edge, without session state.

use serde_json::Value;
use vessel_proto::{PermissionOption, PermissionOptionId, PermissionOptionKind};

pub(super) fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
    PermissionOption {
        id: PermissionOptionId::new(id),
        name: name.to_owned(),
        kind,
    }
}

pub(super) fn command_choice(
    decision: &Value,
) -> Option<(&str, &'static str, PermissionOptionKind)> {
    let id = match decision {
        Value::String(id) => id.as_str(),
        // A decision carrying what it applies: its one key names it.
        Value::Object(fields) if fields.len() == 1 => fields.keys().next()?.as_str(),
        Value::Object(_) | Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
            return None;
        }
    };
    let (name, kind) = match id {
        "accept" => ("Run", PermissionOptionKind::AllowOnce),
        "acceptForSession" => (
            "Run, and do not ask again this session",
            PermissionOptionKind::AllowAlways,
        ),
        "acceptWithExecpolicyAmendment" => (
            "Run, and allow commands like it from now on",
            PermissionOptionKind::AllowAlways,
        ),
        "applyNetworkPolicyAmendment" => (
            "Apply the proposed network rule from now on",
            PermissionOptionKind::AllowAlways,
        ),
        "decline" => ("Do not run", PermissionOptionKind::RejectOnce),
        "cancel" => (
            "Do not run, and stop the turn",
            PermissionOptionKind::RejectOnce,
        ),
        _ => return None,
    };
    Some((id, name, kind))
}
