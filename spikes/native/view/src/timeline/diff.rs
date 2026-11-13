//! A tool call's diff and its input and output as text, as the page's
//! tool card shows them (`features/session/tool-card.tsx`): the old text's
//! lines removed (an empty one left out), then the new text's added, each
//! numbered on its own side; JSON indented by two.

use vessel_proto::RawJson;

/// Which side a diff's line is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Removed: a line of the old text.
    Removed,
    /// Added: a line of the new text.
    Added,
}

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine<'a> {
    /// Its side.
    pub side: Side,
    /// Its number on that side, from 1.
    pub number: usize,
    /// Its words.
    pub text: &'a str,
}

/// The most lines a diff shows: a tool that rewrote a generated file
/// would otherwise hand the screen a hundred thousand rows in one card.
pub const MAX_LINES: usize = 2000;

/// The lines of a diff from `old` (absent: a new file) to `new`, at most
/// [`MAX_LINES`] of them; and whether it was cut.
#[must_use]
pub fn diff_lines<'a>(old: Option<&'a str>, new: &'a str) -> (Vec<DiffLine<'a>>, bool) {
    let removed = old
        .unwrap_or_default()
        .split('\n')
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(at, text)| DiffLine {
            side: Side::Removed,
            number: at.saturating_add(1),
            text,
        });
    let added = new.split('\n').enumerate().map(|(at, text)| DiffLine {
        side: Side::Added,
        number: at.saturating_add(1),
        text,
    });
    let mut lines: Vec<DiffLine<'a>> = removed
        .chain(added)
        .take(MAX_LINES.saturating_add(1))
        .collect();
    let cut = lines.len() > MAX_LINES;
    lines.truncate(MAX_LINES);
    (lines, cut)
}

/// JSON as the card shows it: a string as itself, anything else indented
/// by two; as received when it cannot be read.
#[must_use]
pub fn json_text(raw: &RawJson) -> String {
    match serde_json::from_str::<serde_json::Value>(raw.as_text()) {
        Ok(serde_json::Value::String(text)) => text,
        Ok(value) => {
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.as_text().to_owned())
        }
        Err(_) => raw.as_text().to_owned(),
    }
}
