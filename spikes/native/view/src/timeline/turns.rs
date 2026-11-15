//! Closed turns fold only the work before their final reply. The prompt,
//! final answer and footer remain ordinary rows with stable entry indices.

use std::ops::Range;

use super::{Entry, EntryKind, RunSummary};

/// The latest loaded turn cannot fold while it is still producing output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnActivity {
    /// Its stream is still open.
    Open,
    /// Its stream is settled, even if history omitted a closing record.
    Closed,
}

/// A prompt owns the disclosure for work before its last assistant message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFold {
    /// The trigger remains visible and carries the summary.
    pub host: usize,
    /// These entries fold, except explicitly retained markers such as compaction.
    pub hidden: Range<usize>,
    /// Tool activity is the summary; reasoning alone does not create a turn fold.
    pub summary: RunSummary,
}

/// Disjoint turn ranges make scrolling, find and prompt jumps share one owner map.
#[derive(Debug, Default)]
pub struct TurnFolds {
    turns: Vec<TurnFold>,
}

impl TurnFolds {
    /// Only completed turns with actual work or earlier assistant prose fold.
    #[must_use]
    pub fn of(entries: &[Entry], latest: TurnActivity) -> Self {
        let mut turns = Vec::new();
        let mut host = None;
        for (index, entry) in entries.iter().enumerate() {
            if matches!(entry.kind, EntryKind::Prompt(_)) {
                if let Some(previous) = host {
                    finish(&mut turns, entries, previous, index);
                }
                host = Some(index);
            } else if matches!(
                entry.kind,
                EntryKind::TurnEnded { .. } | EntryKind::TurnFailed { .. }
            ) && let Some(previous) = host.take()
            {
                finish(&mut turns, entries, previous, index);
            }
        }
        if latest == TurnActivity::Closed
            && let Some(host) = host
        {
            finish(&mut turns, entries, host, entries.len());
        }
        Self { turns }
    }

    /// The summary hosted by this trigger, when it has foldable work.
    #[must_use]
    pub fn at_host(&self, host: usize) -> Option<&TurnFold> {
        self.turns
            .binary_search_by_key(&host, |turn| turn.host)
            .ok()
            .and_then(|index| self.turns.get(index))
    }

    /// An entry hidden by a turn is revealed by expanding this same host.
    #[must_use]
    pub fn containing(&self, entry: usize) -> Option<&TurnFold> {
        let after = self
            .turns
            .partition_point(|turn| turn.hidden.start <= entry);
        self.turns
            .get(after.checked_sub(1)?)
            .filter(|turn| turn.hidden.contains(&entry))
    }
}

fn finish(turns: &mut Vec<TurnFold>, entries: &[Entry], host: usize, end: usize) {
    let start = host.saturating_add(1);
    let Some(run) = entries.get(start..end) else {
        return;
    };
    let Some(last_reply) = run
        .iter()
        .rposition(|entry| matches!(entry.kind, EntryKind::Message { .. }))
    else {
        return;
    };
    let Some(folded) = run.get(..last_reply) else {
        return;
    };
    let mut kinds = Vec::new();
    let mut thoughts = 0_u32;
    for entry in folded {
        if let Some(tool) = entry.kind.as_tool() {
            kinds.push(tool.kind);
        }
        if matches!(entry.kind, EntryKind::Thought { .. }) {
            thoughts = thoughts.saturating_add(1);
        }
    }
    if kinds.is_empty()
        && !folded
            .iter()
            .any(|entry| matches!(entry.kind, EntryKind::Message { .. }))
    {
        return;
    }
    turns.push(TurnFold {
        host,
        hidden: start..start.saturating_add(last_reply),
        summary: RunSummary::of(&kinds, thoughts),
    });
}

#[cfg(test)]
mod tests;
