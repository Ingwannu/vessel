//! What a project's commands to approve are, and what changed since the
//! set last approved (ADR 0027: a repository's commands run only when a
//! person approved that exact set; any change is asked again; ADR 0031:
//! the set's scripts and services are in it). The page's
//! `data/workspace/command-changes.ts`, ported with its cases.
//!
//! [Decision Log]
//! - Purpose: show exactly what will run, what changed since the last
//!   approval, and send that digest to `project.approve`. A set with
//!   scripts is approved for its scripts too: they are shown, every one.
//! - Constraint: a node of protocol 11 carries, beside a set that is not
//!   approved, the set last approved with its time (`last_approved`), when
//!   it kept what that approval covered. A node of protocol 10 carries
//!   neither that nor scripts; what can be known there is the project's
//!   folder's set and whether that one is approved.
//! - Alternatives: compare only with the folder (wrong when the set
//!   approved last was another branch's); keep what was shown before (lost
//!   with the window, and another client may have approved since); a text
//!   diff of the whole file (the digest covers commands, not comments).
//! - Choice: compare with `last_approved` when the node gives it; else with
//!   the folder's set when that one is approved and differs; else say that
//!   nothing is known to compare with. Setup and teardown are compared
//!   line by line (a longest common subsequence over at most 32 commands,
//!   the node's bound), a removed line followed by an added one read as
//!   one changed command; scripts are compared by name.
//! - Cost: a command moved within its list reads as removed and added,
//!   which it is to the digest.

use chrono::{DateTime, Utc};
use vessel_proto::scripts::ScriptEntry;
use vessel_proto::workspace::{
    ApprovedCommands, CommandsDigest, FileTerminal, ProjectCommand, ProjectCommands,
};

/// A set of commands as the node lists it, borrowed from its answer.
#[derive(Debug, Clone, Copy)]
pub struct Listed<'a> {
    /// Run when a worktree is created.
    pub setup: &'a [ProjectCommand],
    /// Run before a worktree is removed.
    pub teardown: &'a [ProjectCommand],
    /// What approving this set approves.
    pub digest: &'a CommandsDigest,
    /// Whether a person approved it.
    pub approved: bool,
    /// Its scripts and services, in name order.
    pub scripts: &'a [ScriptEntry],
    /// Terminal commands covered by the digest, not yet drawn by this card.
    pub terminals: &'a [FileTerminal],
    /// The set approved last, when the node kept it.
    pub last_approved: Option<&'a ApprovedCommands>,
}

impl<'a> Listed<'a> {
    /// Refuse a digest whose terminal commands this card cannot display.
    ///
    /// # Errors
    /// Returns the unsupported approved field instead of permitting unseen work.
    pub fn approval(&self) -> Result<(), ApprovalRefusal> {
        if self.terminals.is_empty()
            && self
                .last_approved
                .is_none_or(|last| last.commands.terminals.is_empty())
        {
            Ok(())
        } else {
            Err(ApprovalRefusal::Terminals)
        }
    }

    /// The listed set of `commands`, when it lists one.
    #[must_use]
    pub fn of(commands: &'a ProjectCommands) -> Option<Self> {
        match commands {
            // Every field named on purpose, no `..` (unlike the other wire
            // patterns): a person approves this set by its digest, so a
            // field the node adds (terminals, say) must break the build
            // here until the screen shows it or refuses it by type.
            // Ignoring it would approve commands the screen never showed.
            ProjectCommands::Listed {
                setup,
                teardown,
                digest,
                approved,
                scripts,
                terminals,
                last_approved,
            } => Some(Self {
                setup,
                teardown,
                digest,
                approved: *approved,
                scripts,
                terminals,
                last_approved: last_approved.as_deref(),
            }),
            ProjectCommands::None | ProjectCommands::Invalid { .. } => None,
        }
    }
}

/// Why this card must not approve a command digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ApprovalRefusal {
    /// The digest includes terminal commands the card cannot draw yet.
    #[error("terminal commands cannot be shown by this approval card")]
    Terminals,
}

/// How a line of one list stands against the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// In both.
    Same,
    /// Only in the new.
    Added,
    /// Only in the old.
    Removed,
}

/// One line of a list compared with another, as a diff reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListLine {
    /// How it stands.
    pub change: Change,
    /// The command.
    pub command: ProjectCommand,
}

/// One command of a list compared with the approved one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLine {
    /// Kept, added or removed.
    Line(ListLine),
    /// One command edited, in the place of `was`.
    Changed {
        /// Now.
        command: ProjectCommand,
        /// Before.
        was: ProjectCommand,
    },
}

/// One script or service compared with the approved set, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptLine {
    /// Kept, added or removed.
    Line {
        /// How it stands.
        change: Change,
        /// The entry.
        script: ScriptEntry,
    },
    /// Edited.
    Changed {
        /// Now.
        script: ScriptEntry,
        /// Before.
        was: ScriptEntry,
    },
}

impl ScriptLine {
    /// The entry as it is now (or was, for a removed one).
    #[must_use]
    pub const fn script(&self) -> &ScriptEntry {
        match self {
            Self::Line { script, .. } | Self::Changed { script, .. } => script,
        }
    }
}

/// `after` compared with `before`, in order, removed lines before the
/// added ones that took their place.
#[must_use]
pub fn compare_lists(before: &[ProjectCommand], after: &[ProjectCommand]) -> Vec<ListLine> {
    // Lengths of the longest common subsequence of every pair of suffixes.
    let columns = after.len().saturating_add(1);
    let mut table = vec![0_u32; before.len().saturating_add(1).saturating_mul(columns)];
    let at = |table: &[u32], i: usize, j: usize| -> u32 {
        i.checked_mul(columns)
            .and_then(|row| row.checked_add(j))
            .and_then(|cell| table.get(cell))
            .copied()
            .unwrap_or(0)
    };
    for (i, old) in before.iter().enumerate().rev() {
        for (j, next) in after.iter().enumerate().rev() {
            let value = if old == next {
                at(&table, i.saturating_add(1), j.saturating_add(1)).saturating_add(1)
            } else {
                at(&table, i.saturating_add(1), j).max(at(&table, i, j.saturating_add(1)))
            };
            if let Some(cell) = i
                .checked_mul(columns)
                .and_then(|row| row.checked_add(j))
                .and_then(|cell| table.get_mut(cell))
            {
                *cell = value;
            }
        }
    }
    let mut lines = Vec::with_capacity(before.len().saturating_add(after.len()));
    let (mut i, mut j) = (0_usize, 0_usize);
    loop {
        let (old, next) = (before.get(i), after.get(j));
        match (old, next) {
            (Some(old), Some(next)) if old == next => {
                lines.push(ListLine {
                    change: Change::Same,
                    command: old.clone(),
                });
                i = i.saturating_add(1);
                j = j.saturating_add(1);
            }
            (Some(old), next)
                if next.is_none()
                    || at(&table, i.saturating_add(1), j) >= at(&table, i, j.saturating_add(1)) =>
            {
                lines.push(ListLine {
                    change: Change::Removed,
                    command: old.clone(),
                });
                i = i.saturating_add(1);
            }
            (_, Some(next)) => {
                lines.push(ListLine {
                    change: Change::Added,
                    command: next.clone(),
                });
                j = j.saturating_add(1);
            }
            (None | Some(_), None) => break,
        }
    }
    lines
}

/// `after` against `before`, a run of removed commands followed by a run
/// of added ones paired, in order, into changed commands.
#[must_use]
pub fn compare_commands(before: &[ProjectCommand], after: &[ProjectCommand]) -> Vec<CommandLine> {
    let lines = compare_lists(before, after);
    let mut paired = Vec::with_capacity(lines.len());
    let mut rest = lines.as_slice();
    // The commands of a run of one kind at the start of `rest`.
    let run = |rest: &mut &[ListLine], kind: Change| -> Vec<ProjectCommand> {
        let length = rest.iter().take_while(|line| line.change == kind).count();
        let (taken, after) = rest.split_at(length);
        *rest = after;
        taken.iter().map(|line| line.command.clone()).collect()
    };
    while !rest.is_empty() {
        let same = run(&mut rest, Change::Same);
        let removed = run(&mut rest, Change::Removed);
        let added = run(&mut rest, Change::Added);
        paired.extend(same.into_iter().map(|command| {
            CommandLine::Line(ListLine {
                change: Change::Same,
                command,
            })
        }));
        let mut added = added.into_iter();
        for was in removed {
            paired.push(match added.next() {
                Some(command) => CommandLine::Changed { command, was },
                None => CommandLine::Line(ListLine {
                    change: Change::Removed,
                    command: was,
                }),
            });
        }
        paired.extend(added.map(|command| {
            CommandLine::Line(ListLine {
                change: Change::Added,
                command,
            })
        }));
    }
    paired
}

/// Whether two entries of one name run the same thing in the same way.
fn same_script(one: &ScriptEntry, two: &ScriptEntry) -> bool {
    one.view() == two.view()
}

/// `after`'s scripts against `before`'s, by name, in name order.
#[must_use]
pub fn compare_scripts(before: &[ScriptEntry], after: &[ScriptEntry]) -> Vec<ScriptLine> {
    let mut lines: Vec<ScriptLine> = after
        .iter()
        .map(
            |script| match before.iter().find(|was| was.name == script.name) {
                None => ScriptLine::Line {
                    change: Change::Added,
                    script: script.clone(),
                },
                Some(was) if same_script(was, script) => ScriptLine::Line {
                    change: Change::Same,
                    script: script.clone(),
                },
                Some(was) => ScriptLine::Changed {
                    script: script.clone(),
                    was: was.clone(),
                },
            },
        )
        .collect();
    lines.extend(
        before
            .iter()
            .filter(|was| !after.iter().any(|script| script.name == was.name))
            .map(|was| ScriptLine::Line {
                change: Change::Removed,
                script: was.clone(),
            }),
    );
    lines.sort_by(|one, two| one.script().name.as_str().cmp(two.script().name.as_str()));
    lines
}

/// What the set asked for is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparedWith {
    /// The set the node says was approved last, and when.
    LastApproved {
        /// When.
        at: DateTime<Utc>,
    },
    /// The project folder's set, approved (a node that does not say).
    Folder,
}

/// What the approval dialog shows beside the set asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compared {
    /// Compared with an approved set.
    Changed {
        /// Which.
        against: ComparedWith,
        /// The setup commands.
        setup: Vec<CommandLine>,
        /// The teardown commands.
        teardown: Vec<CommandLine>,
        /// The scripts and services.
        scripts: Vec<ScriptLine>,
    },
    /// No approved set is known to compare with.
    None,
}

fn against(
    asked: &Listed<'_>,
    setup: &[ProjectCommand],
    teardown: &[ProjectCommand],
    scripts: &[ScriptEntry],
    basis: ComparedWith,
) -> Compared {
    Compared::Changed {
        against: basis,
        setup: compare_commands(setup, asked.setup),
        teardown: compare_commands(teardown, asked.teardown),
        scripts: compare_scripts(scripts, asked.scripts),
    }
}

/// The set `asked` compared with the set last approved, or, from a node
/// that does not say, with the folder's (`folder`, `None` when it was not
/// read or could not be).
#[must_use]
pub fn compare_approved(asked: &Listed<'_>, folder: Option<&ProjectCommands>) -> Compared {
    if let Some(last) = asked.last_approved {
        return against(
            asked,
            &last.commands.setup,
            &last.commands.teardown,
            &last.commands.scripts,
            ComparedWith::LastApproved {
                at: last.approved_at,
            },
        );
    }
    match folder.and_then(Listed::of) {
        Some(folder) if folder.approved && folder.digest != asked.digest => against(
            asked,
            folder.setup,
            folder.teardown,
            folder.scripts,
            ComparedWith::Folder,
        ),
        Some(_) | None => Compared::None,
    }
}

/// Whether the dialog must read the folder's set to compare: only when the
/// node did not say which set was approved last.
#[must_use]
pub const fn needs_folder(asked: &Listed<'_>) -> bool {
    asked.last_approved.is_none()
}
