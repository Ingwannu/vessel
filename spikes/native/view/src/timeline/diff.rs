//! Bounded tool edit scripts preserve unchanged context and empty lines.
//! Unified patches retain their hunk offsets instead of being plain output.

use similar::{Algorithm, ChangeTag, capture_diff_slices};
use vessel_proto::RawJson;

mod patch;
pub use patch::unified_lines;
mod words;
pub use words::{WordHighlights, word_highlights};

/// Which side a diff's line is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Removed: a line of the old text.
    Removed,
    /// Added: a line of the new text.
    Added,
    /// The line exists on both sides.
    Context,
    /// A unified patch's hunk header or metadata line has no source number.
    Header,
}

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine<'a> {
    /// Its side.
    pub side: Side,
    /// Its number on that side, from 1; absent on metadata or unknown offsets.
    pub number: Option<usize>,
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
    let old = split_lines(old.unwrap_or_default());
    let new = split_lines(new);
    let ops = capture_diff_slices(Algorithm::Myers, &old, &new);
    let mut old_number = 1_usize;
    let mut new_number = 1_usize;
    let mut lines: Vec<DiffLine<'a>> = ops
        .iter()
        .flat_map(|op| op.iter_changes(&old, &new))
        .take(MAX_LINES.saturating_add(1))
        .map(|change| {
            let (side, number) = match change.tag() {
                ChangeTag::Equal => (Side::Context, new_number),
                ChangeTag::Delete => (Side::Removed, old_number),
                ChangeTag::Insert => (Side::Added, new_number),
            };
            // Saturating counts keep even a hostile oversized input non-panicking.
            if side != Side::Added {
                old_number = old_number.saturating_add(1);
            }
            if side != Side::Removed {
                new_number = new_number.saturating_add(1);
            }
            DiffLine {
                side,
                number: Some(number),
                text: change.value(),
            }
        })
        .collect();
    let cut = old.len() > MAX_LINES || new.len() > MAX_LINES || lines.len() > MAX_LINES;
    lines.truncate(MAX_LINES);
    (lines, cut)
}

fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n')
        .take(MAX_LINES.saturating_add(1))
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
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

#[cfg(test)]
mod tests;
