//! A tool request's choices and the answer each becomes: allow once, each
//! of the agent's own suggestions it can be allowed with, reject; and a
//! plan's (Paseo's reject and implement).
//!
//! [Decision Log]
//! - Purpose: the person chooses among what Claude Code itself offers.
//!   Its suggestions (`permission_suggestions`, the SDK's `PermissionUpdate`
//!   objects) go back exactly as given, with the allowing answer
//!   (`updatedPermissions`), so a rule or a mode change the agent proposed
//!   is never re-spelled here.
//! - What is offered of them: a suggestion read here as one an allowing
//!   answer can carry (rules added or replaced with behavior `allow`, a
//!   mode change, directories added) is an option, labelled from what it
//!   says. Any other (a `deny` or `ask` rule, a removal, a shape this build
//!   does not know) is left out, never guessed: an allowing answer cannot
//!   carry a rule that denies, and an unknown update must not be applied
//!   unseen.
//! - A plan (`ExitPlanMode`): Paseo's two choices, reject and implement.
//!   Implementing switches the session to `acceptEdits`, as Paseo does,
//!   but inside the answer (`updatedPermissions: [setMode]`, an update the
//!   SDK declares for that), where Paseo sends `set_permission_mode`
//!   first: one answer, nothing to fail between two. Paseo's third choice
//!   (implement and go back to the mode before planning) needs that mode
//!   known live, which the options slice brings.

use serde::Deserialize;
use serde_json::{Value, json};
use vessel_proto::{PermissionOption, PermissionOptionId, PermissionOptionKind};

use super::super::wire::Permission;

/// What the agent is told when the person rejects a request.
pub(super) const REJECTED: &str = "The person using Vessel rejected this.";
/// What the agent is told when a request is withdrawn unanswered.
pub(super) const WITHDRAWN: &str =
    "The request was withdrawn before anyone answered (the turn was cancelled or ended).";

/// A choice offered, and the answer it becomes.
pub(super) type Choice = (PermissionOption, Permission);

fn option(id: &str, name: String, kind: PermissionOptionKind) -> PermissionOption {
    PermissionOption {
        id: PermissionOptionId::new(id),
        name,
        kind,
    }
}

fn allow(input: &Value, updates: Vec<Value>, tool_use_id: &str) -> Permission {
    Permission::Allow {
        updated_input: input.clone(),
        updated_permissions: updates,
        tool_use_id: tool_use_id.to_owned(),
    }
}

/// The denial for `tool_use_id`, saying `message`.
pub(in crate::claude) fn deny(message: &str, tool_use_id: &str) -> Permission {
    Permission::Deny {
        message: message.to_owned(),
        tool_use_id: tool_use_id.to_owned(),
    }
}

/// A tool request's choices: allow once, each usable suggestion, reject.
pub(super) fn tool(input: &Value, suggestions: &[Value], tool_use_id: &str) -> Vec<Choice> {
    let mut choices = vec![(
        option("allow", "Allow".to_owned(), PermissionOptionKind::AllowOnce),
        allow(input, Vec::new(), tool_use_id),
    )];
    for (index, suggestion) in suggestions.iter().enumerate() {
        let Some(label) = label(suggestion) else {
            tracing::debug!(index, "a permission suggestion this driver does not offer");
            continue;
        };
        choices.push((
            option(
                &format!("suggestion-{index}"),
                label,
                PermissionOptionKind::AllowAlways,
            ),
            allow(input, vec![suggestion.clone()], tool_use_id),
        ));
    }
    choices.push((
        option(
            "reject",
            "Reject".to_owned(),
            PermissionOptionKind::RejectOnce,
        ),
        deny(REJECTED, tool_use_id),
    ));
    choices
}

/// A plan's choices, as Paseo offers them: reject, or implement (the
/// session goes on in `acceptEdits`).
pub(super) fn plan(input: &Value, tool_use_id: &str) -> Vec<Choice> {
    let accept_edits = json!({
        "type": "setMode",
        "mode": "acceptEdits",
        "destination": "session",
    });
    vec![
        (
            option(
                "reject",
                "Reject".to_owned(),
                PermissionOptionKind::RejectOnce,
            ),
            deny(REJECTED, tool_use_id),
        ),
        (
            option(
                "implement",
                "Implement".to_owned(),
                PermissionOptionKind::AllowOnce,
            ),
            allow(input, vec![accept_edits], tool_use_id),
        ),
    ]
}

/// The SDK's `PermissionUpdate`, as far as an option's label needs it.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Update {
    AddRules {
        rules: Vec<Rule>,
        behavior: String,
        destination: String,
    },
    ReplaceRules {
        rules: Vec<Rule>,
        behavior: String,
        destination: String,
    },
    SetMode {
        mode: String,
        destination: String,
    },
    AddDirectories {
        directories: Vec<String>,
        destination: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rule {
    tool_name: String,
    #[serde(default)]
    rule_content: Option<String>,
}

/// A suggestion's label, when it is one an allowing answer can carry.
fn label(suggestion: &Value) -> Option<String> {
    let update = Update::deserialize(suggestion).ok()?;
    let text = match update {
        Update::AddRules {
            rules,
            behavior,
            destination,
        }
        | Update::ReplaceRules {
            rules,
            behavior,
            destination,
        } if behavior == "allow" && !rules.is_empty() => {
            format!(
                "Always allow {} ({})",
                rules_text(&rules),
                place(&destination)?
            )
        }
        // Never into bypass: that mode is chosen at launch, the one launch
        // that carries the skip flag (ADR 0057's floor).
        Update::SetMode { mode, destination } if mode != "bypassPermissions" => {
            format!("Switch to {mode} ({})", place(&destination)?)
        }
        Update::AddDirectories {
            directories,
            destination,
        } if !directories.is_empty() => format!(
            "Allow access to {} ({})",
            directories.join(", "),
            place(&destination)?
        ),
        Update::AddRules { .. }
        | Update::ReplaceRules { .. }
        | Update::SetMode { .. }
        | Update::AddDirectories { .. } => return None,
    };
    Some(text)
}

fn rules_text(rules: &[Rule]) -> String {
    rules
        .iter()
        .map(|rule| match &rule.rule_content {
            Some(content) => format!("{}({content})", rule.tool_name),
            None => rule.tool_name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Where an update is kept, in words; `None` for a place this build does
/// not know, whose suggestion is then not offered.
fn place(destination: &str) -> Option<&'static str> {
    Some(match destination {
        "session" => "this session",
        "localSettings" => "this project, for you",
        "projectSettings" => "this project",
        "userSettings" => "all your projects",
        "cliArg" => "this launch",
        _ => return None,
    })
}
