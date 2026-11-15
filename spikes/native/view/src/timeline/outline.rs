//! The outline of a session (GOAT's minimap, Paseo's "Chat outline"): one
//! mark per prompt, with what the agent last said in reply, to jump
//! between turns of a long conversation (the page's `data/outline.ts`).

use super::entry::{Entry, EntryKind};
use super::find::{is_space, readable_text};

/// One prompt in the outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    /// The prompt's entry index, to reveal it.
    pub entry: usize,
    /// Its text on one line, cut at `PREVIEW` characters.
    pub prompt: String,
    /// The agent's last message of that turn, cut at `REPLY`; `None`
    /// before it has answered.
    pub reply: Option<String>,
}

/// The longest prompt preview, ellipsis included (Paseo's 120).
pub const PREVIEW: usize = 120;

/// The longest reply preview (GOAT shows three lines of it).
pub const REPLY: usize = 240;

/// The fewest prompts an outline is shown for (GOAT's and Paseo's).
pub const OUTLINE_MIN: usize = 2;

/// The narrowest window that shows the outline rail, in points.
pub const OUTLINE_WIDTH: u32 = 1100;

/// `text` on one line, cut to `max` characters with an ellipsis.
#[must_use]
pub fn preview(text: &str, max: usize) -> String {
    let mut line = String::with_capacity(text.len());
    for word in text.split(is_space).filter(|word| !word.is_empty()) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if line.chars().count() <= max {
        return line;
    }
    let mut cut: String = line.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// The outline of `entries`, in order.
#[must_use]
pub fn outline(entries: &[Entry]) -> Vec<OutlineItem> {
    let mut items: Vec<OutlineItem> = Vec::new();
    let mut last: Option<&str> = None;
    let close = |items: &mut Vec<OutlineItem>, last: Option<&str>| {
        if let (Some(item), Some(reply)) = (items.last_mut(), last) {
            item.reply = Some(preview(&readable_text(reply), REPLY));
        }
    };
    for (index, entry) in entries.iter().enumerate() {
        match &entry.kind {
            EntryKind::Prompt(prompt) => {
                close(&mut items, last.take());
                items.push(OutlineItem {
                    entry: index,
                    prompt: preview(&prompt.text, PREVIEW),
                    reply: None,
                });
            }
            EntryKind::Message { text } if !items.is_empty() => last = Some(text),
            EntryKind::Ready { .. }
            | EntryKind::Steer { .. }
            | EntryKind::Question { .. }
            | EntryKind::Message { .. }
            | EntryKind::Thought { .. }
            | EntryKind::Tool(_)
            | EntryKind::Plan { .. }
            | EntryKind::Permission(_)
            | EntryKind::TurnEnded { .. }
            | EntryKind::TurnFailed { .. }
            | EntryKind::Log { .. }
            | EntryKind::Status { .. }
            | EntryKind::Parking(_)
            | EntryKind::Forked { .. }
            | EntryKind::Profile { .. }
            | EntryKind::Model(_) => {}
        }
    }
    close(&mut items, last);
    items
}
