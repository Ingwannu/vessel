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

use std::collections::HashSet;
use vessel_proto::ToolKind;

use super::entry::{Entry, EntryKind, ToolView};

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

    /// File summaries count distinct reported paths; unknown legacy paths retain
    /// their per-call count rather than inventing a shared file identity.
    #[must_use]
    pub fn of_tools<'a>(tools: impl IntoIterator<Item = &'a ToolView>, thoughts: u32) -> Self {
        let mut summary = Self {
            thoughts,
            ..Self::default()
        };
        let mut reads = HashSet::new();
        let mut edits = HashSet::new();
        for tool in tools {
            let paths = match tool.kind {
                ToolKind::Read => Some(&mut reads),
                ToolKind::Edit | ToolKind::Delete | ToolKind::Move => Some(&mut edits),
                ToolKind::Execute
                | ToolKind::Search
                | ToolKind::Think
                | ToolKind::Fetch
                | ToolKind::SwitchMode
                | ToolKind::Other => None,
            };
            if let Some(paths) = paths
                && !tool.locations.is_empty()
            {
                for location in &tool.locations {
                    let _new = paths.insert(&location.path);
                }
            } else {
                summary.count(tool.kind);
            }
        }
        summary.read = summary
            .read
            .saturating_add(u32::try_from(reads.len()).unwrap_or(u32::MAX));
        summary.edited = summary
            .edited
            .saturating_add(u32::try_from(edits.len()).unwrap_or(u32::MAX));
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
    /// Consecutive groups with at least two members and a tool call. Paseo
    /// keeps a single tool or thought as its original disclosure row.
    #[must_use]
    pub fn of(entries: &[Entry]) -> Self {
        Self::with_reasoning(entries, false)
    }

    /// Expanded reasoning must not disappear inside a closed tool group.
    #[must_use]
    pub fn with_reasoning(entries: &[Entry], expand_reasoning: bool) -> Self {
        let mut runs = Vec::new();
        let mut start: Option<usize> = None;
        for (index, entry) in entries.iter().enumerate() {
            let folds = entry.kind.folds()
                && !(expand_reasoning && matches!(entry.kind, EntryKind::Thought { .. }));
            match (folds, start) {
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
    if members.len() < 2 {
        return;
    }
    let mut tools = members
        .iter()
        .filter_map(|entry| entry.kind.as_tool())
        .peekable();
    let thoughts = members
        .iter()
        .filter(|entry| matches!(entry.kind, EntryKind::Thought { .. }))
        .fold(0_u32, |count, _| count.saturating_add(1));
    if tools.peek().is_some() {
        runs.push(ToolRun {
            lead: from,
            end: to,
            summary: RunSummary::of_tools(tools, thoughts),
        });
    }
}
