//! What Settings › Plugins says of a status, a change, a failure and a
//! source, in the person's language (`locales/*/plugins.ftl`).
//!
//! [Decision Log]
//! - Purpose: the words the page puts together from data (an id in a
//!   note, a code in a refusal) are chosen here, from typed values, so the
//!   app draws a string and both languages are tested without a window.
//! - Choice: a value message that cannot be formatted falls back to the
//!   value itself (the id, the path, the code), never to an error's text.
//! - A node's own words (`Problem::Refused` with a message) are shown as
//!   they came and alone, never inside a translated sentence.

use vessel_proto::plugins::{PluginListItem, PluginStatus};

use super::{Action, Change, Problem};
use crate::localization::Plugins;

/// A status's badge word.
#[must_use]
pub fn status_word(words: Plugins<'_>, status: PluginStatus) -> &str {
    match status {
        PluginStatus::Running => words.status_running(),
        PluginStatus::Disabled => words.status_disabled(),
        PluginStatus::Failed => words.status_failed(),
    }
}

/// The note after `change`.
#[must_use]
pub fn change_words(words: Plugins<'_>, change: &Change) -> String {
    let (said, id) = match change {
        Change::Installed(id) => (words.installed(id.as_str()), id),
        Change::Acted(Action::Reload, id) => (words.reloaded(id.as_str()), id),
        Change::Acted(Action::Enable, id) => (words.enabled(id.as_str()), id),
        Change::Acted(Action::Disable, id) => (words.disabled(id.as_str()), id),
        Change::Acted(Action::Remove, id) => (words.removed(id.as_str()), id),
        Change::Kept(id) => (words.kept(id.as_str()), id),
        Change::Switched(true) => return words.global_enabled().to_owned(),
        Change::Switched(false) => return words.global_disabled().to_owned(),
    };
    said.unwrap_or_else(|_unformatted| id.as_str().to_owned())
}

/// Why a call failed: the node's own words when it gave some, otherwise
/// a sentence for the failure.
#[must_use]
pub fn problem_words(words: Plugins<'_>, problem: &Problem) -> String {
    match problem {
        Problem::Offline => words.problem_offline().to_owned(),
        Problem::Dropped => words.problem_dropped().to_owned(),
        Problem::TimedOut => words.problem_timed_out().to_owned(),
        Problem::Unsupported => words.problem_unsupported().to_owned(),
        Problem::Refused {
            message: Some(message),
            ..
        } => message.clone(),
        Problem::Refused {
            code,
            message: None,
        } => {
            let code = vessel_proto::diagnostics::text::words(code);
            words.problem_refused(&code).unwrap_or(code)
        }
    }
}

/// A row's source line ("Directory: {path}"), when the node said where it
/// is (only to a device that may administer it).
#[must_use]
pub fn source_line(words: Plugins<'_>, plugin: &PluginListItem) -> Option<String> {
    let path = plugin.path.as_deref()?;
    Some(
        words
            .directory(path)
            .unwrap_or_else(|_unformatted| path.to_owned()),
    )
}
