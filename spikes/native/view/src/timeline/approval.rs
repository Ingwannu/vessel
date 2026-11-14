//! A permission request as it is answered and as the timeline keeps it
//! (the page's `data/approval.ts`): GOAT's approval row (Decline and
//! Approve, the rest under "More") and one quiet line per request.
//!
//! Approve is the first "once" allow, else the first allow; Decline the
//! first "once" reject, else the first reject. Every other option, in the
//! agent's order, is under "More".
//!
//! [Decision Log] What a button says ([`answer_words`]).
//! - Purpose: on a Korean screen the permission card said
//!   "에이전트가 실행 허락을 구합니다" over buttons the agent named in
//!   English ("Yes", "No"; the coordinator, 2026-11-13). The words of a
//!   kind the app knows are the app's, in the person's language, the same
//!   on every agent.
//! - Constraint: the agents' real options. Claude Code's adapter offers
//!   one of each kind for a command ("Yes", "Yes, and don't ask again for
//!   `cargo test` commands", "No"), but four allows of one kind to leave
//!   plan mode ("Yes, and use auto mode", "Yes, and bypass permissions",
//!   ...). Codex's offers two "always" allows ("for this command in this
//!   session", "for commands that start with `git status`") and two
//!   "once" rejects ("continue without running it", "tell Codex what to
//!   do differently"). Paseo shows the agent's names on ACP and its own
//!   words (Accept, Deny) only where an agent names nothing.
//! - Choice (the coordinator's): a "once" allow or reject the request
//!   offers once says the app's words (Paseo's Accept and Deny), when its
//!   direction (allow, or reject) has at most two options. An "always"
//!   option always keeps the agent's name: it is a grant, and its name is
//!   the scope agreed to ("this session", "commands that start with
//!   `git status`", "`cargo test` commands"), which the app cannot
//!   restate. A "once" kind offered several times keeps the agent's names
//!   too (they differ in meaning, and only the agent says how). Two
//!   buttons never say the same: should the rule give that, every button
//!   of the request keeps the agent's name.
//! - Refined (the coordinator on build 0.0.0.0.8's captures): among
//!   Claude Code's five named ways to leave plan mode a bare "수락" did not
//!   say what it does (approve edits by hand). With three or more options
//!   of one direction, the agent's names are the choice itself, so every
//!   option of that direction keeps its name, as Paseo shows them. That
//!   also gives Codex's "Yes, proceed" its own name beside its two
//!   "always" allows; its two rejects already kept theirs.

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

/// What an answer's button says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerWords<'a> {
    /// The app's words for a "once" option's kind.
    Kind(PermissionOptionKind),
    /// The agent's own name: an "always" option (its scope), a "once"
    /// kind offered more than once, any option of a direction (allow, or
    /// reject) offered three times or more, or every option of a request
    /// whose buttons would otherwise say the same.
    Agent(&'a str),
}

/// What each of `options`' buttons says, in the agent's order. `say`
/// puts a kind in the person's words, to find buttons that would say the
/// same.
#[must_use]
pub fn answer_words<'a>(
    options: &'a [PermissionOption],
    say: impl Fn(PermissionOptionKind) -> &'static str,
) -> Vec<AnswerWords<'a>> {
    let words: Vec<AnswerWords<'a>> = options
        .iter()
        .map(|option| {
            let once = matches!(
                option.kind,
                PermissionOptionKind::AllowOnce | PermissionOptionKind::RejectOnce
            );
            let alike = options
                .iter()
                .filter(|other| other.kind == option.kind)
                .take(2)
                .count();
            let direction = options
                .iter()
                .filter(|other| allows(other.kind) == allows(option.kind))
                .take(3)
                .count();
            if once && alike == 1 && direction <= 2 {
                AnswerWords::Kind(option.kind)
            } else {
                AnswerWords::Agent(&option.name)
            }
        })
        .collect();
    let text = |words: &AnswerWords<'a>| -> &'a str {
        match *words {
            AnswerWords::Kind(kind) => say(kind),
            AnswerWords::Agent(name) => name,
        }
    };
    let repeated = words.iter().enumerate().any(|(at, one)| {
        words
            .iter()
            .skip(at.saturating_add(1))
            .any(|other| text(one) == text(other))
    });
    if repeated {
        options
            .iter()
            .map(|option| AnswerWords::Agent(&option.name))
            .collect()
    } else {
        words
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
        Some(PermissionOutcome::Selected { option, .. }) => {
            match options.iter().find(|offered| offered.id == *option) {
                None => Settled::Allowed(option.to_string()),
                Some(chosen) if allows(chosen.kind) => Settled::Allowed(chosen.name.clone()),
                Some(chosen) => Settled::Declined(chosen.name.clone()),
            }
        }
    }
}
