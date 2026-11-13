//! A permission request as it is answered and as the timeline keeps it
//! (the page's `data/approval.ts`): GOAT's approval row (Decline and
//! Approve, the rest under "More") and one quiet line per request.
//!
//! Approve is the first "once" allow, else the first allow; Decline the
//! first "once" reject, else the first reject. Every other option, in the
//! agent's order, is under "More". The buttons keep the agent's names.

use vessel_proto::{PermissionOption, PermissionOptionKind, PermissionOutcome};

/// The answers of one request, as the approval row shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answers<'a> {
    /// The Approve button's option.
    pub approve: Option<&'a PermissionOption>,
    /// The Decline button's option.
    pub decline: Option<&'a PermissionOption>,
    /// The rest, under "More".
    pub more: Vec<&'a PermissionOption>,
}

/// Whether an option of `kind` allows.
#[must_use]
pub const fn allows(kind: PermissionOptionKind) -> bool {
    match kind {
        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways => true,
        PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways => false,
    }
}

/// `options` split into Approve, Decline and the rest.
#[must_use]
pub fn answers_of(options: &[PermissionOption]) -> Answers<'_> {
    let approve = options
        .iter()
        .find(|option| option.kind == PermissionOptionKind::AllowOnce)
        .or_else(|| options.iter().find(|option| allows(option.kind)));
    let decline = options
        .iter()
        .find(|option| option.kind == PermissionOptionKind::RejectOnce)
        .or_else(|| options.iter().find(|option| !allows(option.kind)));
    let chosen = |option: &PermissionOption, button: Option<&PermissionOption>| {
        button.is_some_and(|button| std::ptr::eq(button, option))
    };
    Answers {
        approve,
        decline,
        more: options
            .iter()
            .filter(|option| !chosen(option, approve) && !chosen(option, decline))
            .collect(),
    }
}

/// Where a request stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// Waiting for an answer.
    Pending,
    /// Allowed, with the option's name.
    Allowed(String),
    /// Declined, with the option's name.
    Declined(String),
    /// Withdrawn (the turn or the session ended).
    Cancelled,
}

/// `outcome` of a request offering `options`.
#[must_use]
pub fn settled(options: &[PermissionOption], outcome: Option<&PermissionOutcome>) -> Settled {
    match outcome {
        None => Settled::Pending,
        Some(PermissionOutcome::Cancelled) => Settled::Cancelled,
        Some(PermissionOutcome::Selected { option }) => {
            match options.iter().find(|offered| offered.id == *option) {
                None => Settled::Allowed(option.to_string()),
                Some(chosen) if allows(chosen.kind) => Settled::Allowed(chosen.name.clone()),
                Some(chosen) => Settled::Declined(chosen.name.clone()),
            }
        }
    }
}
