//! A failed turn's words as one sentence (the coordinator's finding on
//! 0.0.0.0.15: a usage limit drew "Turn failed · Internal error: You've
//! hit your 5-hour limit · resets 6:40am (UTC)" followed by the agent's
//! data as JSON, under the agent's own reply saying the same).
//!
//! [Decision Log]
//! - Purpose: the person reads why the turn stopped, once, in a sentence.
//! - Constraint: the node writes the agent's error with its causes joined
//!   by ": " (`vessel-agent` `driver::describe`): the JSON-RPC error's
//!   generic message ("Internal error"), the agent's own sentence, and its
//!   data as JSON (Claude's `errorKind`).
//! - The reference: Paseo writes a failed turn once, as its error text
//!   (`agent-manager.ts` `formatTurnFailedMessage`, appended as a system
//!   error message), and its Claude provider holds back the reply Claude
//!   writes when a turn stops on an API error, since the failure says the
//!   same (`providers/claude/agent.ts` `withheldErrorReply`).
//! - Choice: the agent's sentence alone, without the JSON-RPC wrapper or
//!   the JSON; a usage limit (`errorKind` `rate_limit`, or the limit's own
//!   sentence) is said with nothing before it, any other failure after
//!   "Turn failed". A reply that repeats the sentence right before the
//!   failure is not drawn ([`repeats_failure`]).
//! - Cost: an agent whose real words start with a JSON-RPC message's name
//!   loses those words; the full text stays in the session's log (`vessel
//!   log`).

use serde_json::Value;

use super::entry::{Entry, EntryKind};

/// The JSON-RPC errors' generic messages (the specification's), which
/// say nothing the agent's own words after them do not.
const GENERIC: [&str; 5] = [
    "Internal error",
    "Invalid params",
    "Invalid request",
    "Method not found",
    "Parse error",
];

/// The `errorKind`s that mean a usage limit.
const LIMIT_KINDS: [&str; 2] = ["rate_limit", "usage_limit"];

/// Why a turn failed, as one sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFailure {
    /// The agent's sentence, without the wrapper and the data.
    pub sentence: String,
    /// What kind of failure it is.
    pub kind: FailureKind,
}

/// What kind of failure a turn's is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// A usage limit: the sentence says it all (when it resets).
    Limit,
    /// Anything else.
    Other,
}

/// The failure the node's `message` describes.
#[must_use]
pub fn turn_failure(message: &str) -> TurnFailure {
    let (text, error_kind) = split_data(message.trim());
    let sentence = strip_generic(text).trim();
    let sentence = if sentence.is_empty() {
        message.trim()
    } else {
        sentence
    };
    // An agent whose error's data is a plain string (the node writes it
    // as JSON, quoted: `Internal error: "the turn failed"`): its words.
    let sentence = serde_json::from_str::<String>(sentence).unwrap_or_else(|_| sentence.to_owned());
    let limit = error_kind
        .as_deref()
        .is_some_and(|kind| LIMIT_KINDS.contains(&kind))
        || says_limit(&sentence);
    TurnFailure {
        sentence,
        kind: if limit {
            FailureKind::Limit
        } else {
            FailureKind::Other
        },
    }
}

/// Whether the reply at `index` only repeats the failure that ends its
/// turn (Claude writes its limit as a reply, then fails the turn with the
/// same words): Paseo holds such a reply back.
#[must_use]
pub fn repeats_failure(entries: &[Entry], index: usize) -> bool {
    let Some(Entry {
        kind: EntryKind::Message { text },
        ..
    }) = entries.get(index)
    else {
        return false;
    };
    let reply = text.trim();
    entries
        .get(index.saturating_add(1)..)
        .unwrap_or_default()
        .iter()
        .find(|entry| ends_or_speaks(&entry.kind))
        .is_some_and(|entry| {
            if let EntryKind::TurnFailed { message } = &entry.kind {
                turn_failure(message).sentence == reply
            } else {
                false
            }
        })
}

/// Whether an entry ends a turn or is someone's words: the first such
/// after a reply says whether the reply was the turn's last word.
const fn ends_or_speaks(kind: &EntryKind) -> bool {
    matches!(
        kind,
        EntryKind::Message { .. }
            | EntryKind::Prompt(_)
            | EntryKind::Steer { .. }
            | EntryKind::TurnEnded { .. }
            | EntryKind::TurnFailed { .. }
    )
}

/// `text` without the JSON data the node joined after it (": {...}" or
/// ": [...]", the first that parses to the end), and the data's
/// `errorKind` when it has one.
fn split_data(text: &str) -> (&str, Option<String>) {
    let starts = text
        .match_indices(": {")
        .chain(text.match_indices(": ["))
        .map(|(at, _)| at);
    let mut starts: Vec<usize> = starts.collect();
    starts.sort_unstable();
    for at in starts {
        let Some(data) = text.get(at.saturating_add(2)..) else {
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(data) {
            let kind = value
                .get("errorKind")
                .and_then(Value::as_str)
                .map(str::to_owned);
            return (text.get(..at).unwrap_or(text), kind);
        }
    }
    (text, None)
}

/// `text` without a JSON-RPC generic message before the agent's words.
fn strip_generic(text: &str) -> &str {
    GENERIC
        .iter()
        .find_map(|generic| {
            text.strip_prefix(generic)
                .and_then(|rest| rest.strip_prefix(": "))
                .filter(|rest| !rest.trim().is_empty())
        })
        .unwrap_or(text)
}

/// Whether the sentence is a usage limit's, when the data does not say
/// (Claude Code's "You've hit your 5-hour limit", "Claude AI usage limit
/// reached").
fn says_limit(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    (lower.contains("hit your") && lower.contains("limit")) || lower.contains("usage limit")
}
