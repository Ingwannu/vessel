//! Ephemeral draft-fork summary producer (Paseo activity-curator.ts:308-375).
//! This owns no context store; W6's `ChatHistory` payload owns final draft intake.

use crate::timeline::{Entry, EntryKey, EntryKind};
use std::{borrow::Cow, path::Path};
use vessel_proto::{SessionRef, SessionTitle};

/// Paseo measures JavaScript string length: UTF-16 units, including the wrapper.
pub const MAX_FORK_UNITS: usize = 100_000;
const MAX_METADATA_UNITS: usize = 1_000;
const MAX_TOOL_UNITS: usize = 200;
const OPEN: &str = "<chat-history-summary>\nChat history from a previous Vessel agent.";
const CLOSE: &str = "\n</chat-history-summary>";
const OMITTED: &str = "[Earlier chat history omitted]\n";
const UNAVAILABLE: &str = "[Earlier chat history unavailable]\n";
const CUT: &str = "\n[Message truncated]\n";

/// A timeline row retains its full source identity when inherited rows are joined.
#[derive(Debug, Clone, Copy)]
pub struct SummaryEntry<'a> {
    /// Managed source of this row; sequence alone is not fleet identity.
    pub source: SessionRef,
    /// Already projected canonical timeline row.
    pub entry: &'a Entry,
}

/// A render-time checkpoint detects a changed streamed row before seeding a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBoundary {
    source: SessionRef,
    key: EntryKey,
    revision: u32,
    through: vessel_proto::EventSeq,
}
impl ForkBoundary {
    /// Capture the exact visible row, not a mutable array position.
    #[must_use]
    pub fn of(source: SessionRef, entry: &Entry) -> Self {
        Self {
            source,
            key: entry.key.clone(),
            revision: entry.rev,
            through: entry.through,
        }
    }
}

/// The caller must report whether inherited/retained history was actually loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryHistory {
    /// The complete selected conversation is present.
    Complete,
    /// Only retained/loaded history is available; disclose the missing prefix.
    RetainedOnly,
    /// No reliable source history was available.
    Unavailable,
}

/// Bounded metadata accompanies the snapshot; neither field is a lookup path.
#[derive(Debug, Clone, Copy)]
pub struct SummarySource<'a> {
    /// Original session title, when available.
    pub title: Option<&'a SessionTitle>,
    /// Original working directory, when reported.
    pub directory: Option<&'a Path>,
    /// Explicit source completeness.
    pub history: SummaryHistory,
}

/// Missing history and a stale checkpoint are refusal states, never empty success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SummaryProblem {
    /// Retry source loading before creating another draft.
    #[error("source history is unavailable")]
    Unavailable,
    /// The selected row disappeared or changed after rendering.
    #[error("selected history checkpoint changed")]
    StaleBoundary,
}

/// The generated snapshot is bounded and transient, not a second context store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkSummary {
    text: String,
    history: SummaryHistory,
    limited: bool,
}
impl ForkSummary {
    /// Pass once to W6's validated `ContextText` at the composer intake boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
    /// Adapt once into the canonical composer payload; this owns no attachment store.
    /// # Errors
    /// Refuses a snapshot outside the composer's validated text budget.
    pub fn into_chat_history(
        self,
        id: crate::composer::context::ContextId,
        title: crate::composer::context::ContextTitle,
    ) -> Result<crate::composer::context::ChatHistory, crate::composer::context::ContextProblem>
    {
        use crate::composer::context::{ChatHistory, ContextText, Payload};
        Ok(ChatHistory::new(Payload {
            id,
            title,
            text: ContextText::parse(&self.text)?,
            url: None,
        }))
    }
    /// Preserve missing source history separately from budget truncation.
    #[must_use]
    pub const fn history(&self) -> SummaryHistory {
        self.history
    }
    /// Whether earlier content or the middle of a large message was omitted.
    #[must_use]
    pub const fn limited(&self) -> bool {
        self.limited
    }
}

/// Curate only user/assistant/tool activity; never parse external raw tool input.
/// # Errors
/// Source history is unavailable or the selected row is stale/missing.
pub fn fork_summary(
    rows: &[SummaryEntry<'_>],
    boundary: Option<&ForkBoundary>,
    source: SummarySource<'_>,
) -> Result<ForkSummary, SummaryProblem> {
    if source.history == SummaryHistory::Unavailable {
        return Err(SummaryProblem::Unavailable);
    }
    let selected = match boundary {
        None => rows,
        Some(boundary) => {
            let index = rows
                .iter()
                .position(|row| row.source == boundary.source && row.entry.key == boundary.key)
                .ok_or(SummaryProblem::StaleBoundary)?;
            if rows
                .get(index)
                .is_none_or(|row| row.entry.rev != boundary.revision)
            {
                return Err(SummaryProblem::StaleBoundary);
            }
            let selected = rows.get(..=index).ok_or(SummaryProblem::StaleBoundary)?;
            // An earlier tool may have changed after the selected answer while
            // that answer's own revision stayed unchanged. Its current body is
            // not a historical snapshot through the captured source boundary.
            if selected
                .iter()
                .any(|row| row.source == boundary.source && row.entry.through > boundary.through)
            {
                return Err(SummaryProblem::StaleBoundary);
            }
            selected
        }
    };
    let header = header(source);
    let budget = MAX_FORK_UNITS
        .saturating_sub(units(header.chars(), MAX_FORK_UNITS))
        .saturating_sub(units(CLOSE.chars(), MAX_FORK_UNITS));
    let (body, limited) = recent(selected, budget, source.history);
    Ok(ForkSummary {
        text: format!("{header}{body}{CLOSE}"),
        history: source.history,
        limited,
    })
}

fn header(source: SummarySource<'_>) -> String {
    let mut header = OPEN.to_owned();
    if let Some(title) = source.title {
        header.push_str("\nSource agent: ");
        header.push_str(&take_units(
            title.as_str().trim().chars(),
            MAX_METADATA_UNITS,
        ));
    }
    if let Some(directory) = source.directory {
        header.push_str("\nSource directory: ");
        header.push_str(&take_units(
            directory.to_string_lossy().trim().chars(),
            MAX_METADATA_UNITS,
        ));
    }
    header.push_str("\n\n");
    header
}

struct Activity<'a> {
    prefix: &'static str,
    text: Cow<'a, str>,
}
impl Activity<'_> {
    fn units(&self, limit: usize) -> usize {
        units(self.prefix.chars().chain(self.text.chars()), limit)
    }
    fn full(&self) -> String {
        format!("{}{}", self.prefix, self.text)
    }
    fn cut(&self, budget: usize) -> String {
        let available = budget.saturating_sub(units(CUT.chars(), budget));
        let head = available.div_ceil(2);
        let front = take_units(self.prefix.chars().chain(self.text.chars()), head);
        let backwards = take_units(
            self.text.chars().rev().chain(self.prefix.chars().rev()),
            available.saturating_sub(head),
        );
        format!(
            "{front}{CUT}{}",
            backwards.chars().rev().collect::<String>()
        )
    }
}

fn activity(entry: &EntryKind) -> Option<Activity<'_>> {
    let (prefix, text) = match entry {
        EntryKind::Prompt(prompt) => ("[User] ", Cow::Borrowed(prompt.text.trim())),
        EntryKind::Steer { text, .. } => ("[User] ", Cow::Borrowed(text.trim())),
        EntryKind::Message { text } if !text.trim().is_empty() => {
            ("[Assistant] ", Cow::Borrowed(text.trim()))
        }
        EntryKind::Tool(tool) => ("[Tool] ", Cow::Owned(tool_summary(&tool.title))),
        EntryKind::Ready { .. }
        | EntryKind::Question { .. }
        | EntryKind::Message { .. }
        | EntryKind::Thought { .. }
        | EntryKind::Plan { .. }
        | EntryKind::Permission(_)
        | EntryKind::TurnEnded { .. }
        | EntryKind::TurnFailed { .. }
        | EntryKind::Log { .. }
        | EntryKind::Status { .. }
        | EntryKind::Parking(_)
        | EntryKind::Forked { .. }
        | EntryKind::Profile { .. }
        | EntryKind::Model(_) => return None,
    };
    Some(Activity { prefix, text })
}

fn tool_summary(text: &str) -> String {
    let mut output = String::new();
    let mut size = 0_usize;
    let mut space = false;
    for ch in text.trim().chars() {
        if ch.is_whitespace() {
            space = !output.is_empty();
            continue;
        }
        let extra = ch.len_utf16().saturating_add(usize::from(space));
        if size.saturating_add(extra) > MAX_TOOL_UNITS {
            break;
        }
        if space {
            output.push(' ');
        }
        output.push(ch);
        size = size.saturating_add(extra);
        space = false;
    }
    output
}

fn recent(rows: &[SummaryEntry<'_>], budget: usize, history: SummaryHistory) -> (String, bool) {
    let prefix = if history == SummaryHistory::RetainedOnly {
        UNAVAILABLE
    } else {
        ""
    };
    let mut remaining = budget.saturating_sub(units(prefix.chars(), budget));
    let mut selected = Vec::new();
    for entry in rows
        .iter()
        .rev()
        .filter_map(|row| activity(&row.entry.kind))
    {
        let separator = usize::from(!selected.is_empty());
        let size = entry.units(remaining).saturating_add(separator);
        if size > remaining {
            return omitted(selected, &entry, remaining, budget, prefix);
        }
        remaining = remaining.saturating_sub(size);
        selected.push(entry.full());
    }
    if selected.is_empty() {
        selected.push("No chat history to display.".into());
    }
    selected.reverse();
    (format!("{prefix}{}", selected.join("\n")), false)
}

fn omitted(
    mut selected: Vec<String>,
    entry: &Activity<'_>,
    mut remaining: usize,
    budget: usize,
    prefix: &str,
) -> (String, bool) {
    let marker = if prefix.is_empty() { OMITTED } else { prefix };
    let marker_size = units(marker.chars(), budget);
    if selected.is_empty() {
        return (
            format!("{marker}{}", entry.cut(budget.saturating_sub(marker_size))),
            true,
        );
    }
    if prefix.is_empty() {
        while remaining < marker_size {
            let Some(oldest) = selected.pop() else { break };
            remaining = remaining
                .saturating_add(units(oldest.chars(), budget))
                .saturating_add(1);
        }
    }
    selected.reverse();
    (format!("{marker}{}", selected.join("\n")), true)
}

fn units(chars: impl Iterator<Item = char>, limit: usize) -> usize {
    let mut size = 0_usize;
    for ch in chars {
        size = size.saturating_add(ch.len_utf16());
        if size > limit {
            break;
        }
    }
    size
}
fn take_units(chars: impl Iterator<Item = char>, limit: usize) -> String {
    let mut size = 0_usize;
    let mut text = String::new();
    for ch in chars {
        let next = size.saturating_add(ch.len_utf16());
        if next > limit {
            break;
        }
        text.push(ch);
        size = next;
    }
    text
}

#[cfg(test)]
#[path = "fork_summary_tests.rs"]
mod tests;
