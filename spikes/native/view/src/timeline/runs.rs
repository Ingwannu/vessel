//! "Collapse tool calls" (Settings › Appearance; Paseo's "Chat display"):
//! a run of consecutive tool calls and thoughts is one summary row,
//! "Edited 2 files and ran 3 commands", opened on a click (the page's
//! `data/tool-runs.ts`). A pure function of the entries, so any slice of
//! a windowed timeline can ask the same.
//!
//! Runs are ranges of entry indices here, where the page keys them by
//! entry key: an entry's index never changes (entries are only appended
//! or changed in place), so an index names an entry as well as its key
//! does, and costs no hashing.

use vessel_proto::ToolKind;

use super::entry::{Entry, EntryKind};

/// How a run's tool calls are counted, in the sentence's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunSummary {
    /// Edits, deletes and moves.
    pub edited: u32,
    /// Commands run.
    pub ran: u32,
    /// Files read.
    pub read: u32,
    /// Searches.
    pub searched: u32,
    /// Anything else (thinking, fetching, a mode switch).
    pub other: u32,
    /// Thoughts in the run.
    pub thoughts: u32,
}

impl RunSummary {
    /// The counts for `kinds` and `thoughts` thoughts.
    #[must_use]
    pub fn of(kinds: &[ToolKind], thoughts: u32) -> Self {
        let mut summary = Self {
            thoughts,
            ..Self::default()
        };
        for kind in kinds {
            summary.count(*kind);
        }
        summary
    }

    fn count(&mut self, kind: ToolKind) {
        let tally = match kind {
            ToolKind::Edit | ToolKind::Delete | ToolKind::Move => &mut self.edited,
            ToolKind::Execute => &mut self.ran,
            ToolKind::Read => &mut self.read,
            ToolKind::Search => &mut self.searched,
            ToolKind::Think | ToolKind::Fetch | ToolKind::SwitchMode | ToolKind::Other => {
                &mut self.other
            }
        };
        *tally = tally.saturating_add(1);
    }

    /// The tool counts that are not zero, in the sentence's order.
    #[must_use]
    pub fn parts(&self) -> Vec<(Tally, u32)> {
        [
            (Tally::Edited, self.edited),
            (Tally::Ran, self.ran),
            (Tally::Read, self.read),
            (Tally::Searched, self.searched),
            (Tally::Other, self.other),
        ]
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect()
    }
}

/// One kind of count in a run's sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tally {
    /// "edited n files".
    Edited,
    /// "ran n commands".
    Ran,
    /// "read n files".
    Read,
    /// "searched n times".
    Searched,
    /// "used n other tools".
    Other,
}

/// One run of consecutive tool calls and thoughts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRun {
    /// The first entry's index: where the summary row stands.
    pub lead: usize,
    /// One past the last entry's index.
    pub end: usize,
    /// What its sentence says.
    pub summary: RunSummary,
}

/// The runs of a timeline worth folding, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folds {
    runs: Vec<ToolRun>,
}

impl Folds {
    /// The runs of `entries` worth folding: every run with a tool call in
    /// it, a single one included (Paseo's overview rows summarise one
    /// call as they do ten); a lone thought stays as it is, a disclosure
    /// already.
    #[must_use]
    pub fn of(entries: &[Entry]) -> Self {
        let mut runs = Vec::new();
        let mut start: Option<usize> = None;
        for (index, entry) in entries.iter().enumerate() {
            match (entry.kind.folds(), start) {
                (true, None) => start = Some(index),
                (false, Some(from)) => {
                    close(&mut runs, entries, from, index);
                    start = None;
                }
                (true, Some(_)) | (false, None) => {}
            }
        }
        if let Some(from) = start {
            close(&mut runs, entries, from, entries.len());
        }
        Self { runs }
    }

    /// The runs, in order.
    #[must_use]
    pub fn runs(&self) -> &[ToolRun] {
        &self.runs
    }

    /// The run entry `index` is in, if it is folded.
    #[must_use]
    pub fn run_of(&self, index: usize) -> Option<&ToolRun> {
        let after = self.runs.partition_point(|run| run.lead <= index);
        let run = self.runs.get(after.checked_sub(1)?)?;
        (index < run.end).then_some(run)
    }
}

fn close(runs: &mut Vec<ToolRun>, entries: &[Entry], from: usize, to: usize) {
    let Some(members) = entries.get(from..to) else {
        return;
    };
    let mut kinds = Vec::new();
    let mut thoughts = 0_u32;
    for entry in members {
        match &entry.kind {
            EntryKind::Tool(tool) => kinds.push(tool.kind),
            EntryKind::Thought { .. } => thoughts = thoughts.saturating_add(1),
            EntryKind::Ready { .. }
            | EntryKind::Prompt(_)
            | EntryKind::Steer { .. }
            | EntryKind::Question { .. }
            | EntryKind::Message { .. }
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
    if !kinds.is_empty() {
        runs.push(ToolRun {
            lead: from,
            end: to,
            summary: RunSummary::of(&kinds, thoughts),
        });
    }
}
