//! What a session is called, and what a person may rename it to
//! (protocol 9, ADR 0022; the page's `data/titles.ts`).
//!
//! The node keeps the one truth (`SessionInfo::title`) and parses a title
//! on the way in (`SessionTitle`). The title bar checks a typed title with
//! that same parse before it asks, so a refusal is said beside the field
//! as the person types; the node still decides.

use std::path::Path;

use vessel_proto::{InvalidTitle, SessionInfo, SessionTitle};

/// What a session is called when it has no title: its folder's last
/// segment; `None` when it has none (the home folder, no folder, the
/// root). Both separators: the node may be a Windows machine.
#[must_use]
pub fn folder_name(cwd: Option<&Path>) -> Option<String> {
    let text = cwd?.to_string_lossy();
    text.split(['/', '\\'])
        .rfind(|segment| !segment.is_empty())
        .map(str::to_owned)
}

/// What the session is called: the title its log last reported
/// (fresher than the listed one), else the node's, else its folder's
/// name; `None` when it has none of them (the screen then says
/// "Untitled session").
#[must_use]
pub fn session_name(session: &SessionInfo, renamed: Option<&SessionTitle>) -> Option<String> {
    renamed
        .or(session.title.as_ref())
        .map(ToString::to_string)
        .or_else(|| folder_name(session.spec.cwd.as_deref()))
}

/// Whether `session` has a title of its own (not its folder's name).
#[must_use]
pub const fn has_title(session: &SessionInfo) -> bool {
    session.title.is_some()
}

/// `text` as a title, or why it cannot be one (the node's own rules:
/// trimmed, one line, at most `SessionTitle::MAX_CHARS` characters).
///
/// # Errors
///
/// Why the node would refuse it.
pub fn check_title(text: &str) -> Result<SessionTitle, InvalidTitle> {
    text.parse()
}

/// Whether a title may be edited in place: not for an ended session.
#[must_use]
pub const fn can_rename(session: &SessionInfo) -> bool {
    session.status.is_live()
}

/// The title a prompt gives a session, as the node takes one from a
/// first prompt: its first line with text, cut at
/// `SessionTitle::FROM_PROMPT_CHARS` characters.
#[must_use]
pub fn title_from_prompt(text: &str) -> Option<SessionTitle> {
    SessionTitle::from_prompt(text)
}
