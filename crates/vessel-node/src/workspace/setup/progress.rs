//! Bounded structured state, updated only by the runner's actual events.
//!
//! The synchronous observer never performs persistence or network I/O.
//! A watch wakes readers even after the legacy combined log fills.

use chrono::Utc;
use vessel_proto::workspace::{
    MAX_PROJECT_COMMANDS, MAX_SETUP_COMMAND_OUTPUT_BYTES as MAX_COMMAND_OUTPUT, ProjectCommand,
    SetupCommandIndex, SetupCommandProgress, SetupCommandResult, SetupCommandState, SetupCursor,
    SetupProgress, SetupRunState,
};
use vessel_proto::{DurationMs, SetupRevision, SetupRunId, TerminalBytes, WorktreeId};
use vessel_workspace::commands::{CommandEvent, CommandOutcome};

const MIDDLE_MARKER: &[u8] = b"\n...<output truncated in the middle>...\n";
const CONTENT_BUDGET: usize = MAX_COMMAND_OUTPUT.saturating_sub(MIDDLE_MARKER.len());
const HEAD_BUDGET: usize = CONTENT_BUDGET.div_ceil(2);
const TAIL_BUDGET: usize = CONTENT_BUDGET.saturating_sub(HEAD_BUDGET);

#[derive(Debug)]
pub(crate) struct Observation {
    pub(super) snapshot: SetupProgress,
}

impl Observation {
    pub(super) fn new(worktree: WorktreeId, commands: &[ProjectCommand]) -> Option<Self> {
        if commands.len() > MAX_PROJECT_COMMANDS {
            return None;
        }
        let commands = commands
            .iter()
            .enumerate()
            .map(|(index, command)| {
                Some(SetupCommandProgress {
                    index: SetupCommandIndex::try_from(u32::try_from(index.checked_add(1)?).ok()?)
                        .ok()?,
                    command: command.clone(),
                    state: SetupCommandState::Waiting,
                    output: TerminalBytes::default(),
                    truncated: false,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            snapshot: SetupProgress {
                worktree,
                cursor: SetupCursor {
                    run: SetupRunId::random(),
                    revision: SetupRevision::ZERO,
                },
                state: SetupRunState::Running,
                started_at: Utc::now(),
                commands,
            },
        })
    }

    pub(super) fn event(&mut self, event: CommandEvent<'_>) {
        let index = match &event {
            CommandEvent::Started { index, command: _ }
            | CommandEvent::Output { index, bytes: _ }
            | CommandEvent::Finished {
                index,
                elapsed: _,
                outcome: _,
            } => index.get(),
        };
        let Some(row) = self.snapshot.commands.get_mut(index) else {
            self.snapshot.state = SetupRunState::Unknown;
            self.advance();
            return;
        };
        match event {
            CommandEvent::Started { index: _, command } => {
                row.state = if row.command == *command && row.state == SetupCommandState::Waiting {
                    SetupCommandState::Running
                } else {
                    SetupCommandState::Unknown
                };
            }
            CommandEvent::Output { index: _, bytes } => {
                if row.state == SetupCommandState::Running {
                    append_output(row, bytes);
                }
            }
            CommandEvent::Finished {
                index: _,
                elapsed,
                outcome,
            } => {
                row.state = if row.state == SetupCommandState::Running {
                    SetupCommandState::Finished {
                        elapsed: DurationMs::from_duration(elapsed),
                        result: result(outcome),
                    }
                } else {
                    SetupCommandState::Unknown
                };
            }
        }
        self.advance();
    }

    pub(super) fn finish(&mut self, succeeded: bool) {
        self.snapshot.state = if self.snapshot.state == SetupRunState::Unknown
            || self
                .snapshot
                .commands
                .iter()
                .any(|row| row.state == SetupCommandState::Unknown)
        {
            SetupRunState::Unknown
        } else if self.snapshot.commands.iter().any(|row| {
            matches!(
                row.state,
                SetupCommandState::Finished {
                    result: SetupCommandResult::Cancelled,
                    ..
                }
            )
        }) {
            SetupRunState::Cancelled
        } else if succeeded
            && self.snapshot.commands.iter().all(|row| {
                matches!(
                    row.state,
                    SetupCommandState::Finished {
                        result: SetupCommandResult::Exited { code: Some(0) },
                        ..
                    }
                )
            })
        {
            SetupRunState::Succeeded
        } else {
            SetupRunState::Failed
        };
        self.advance();
    }

    fn advance(&mut self) {
        if let Some(revision) = self.snapshot.cursor.revision.next() {
            self.snapshot.cursor.revision = revision;
        } else {
            // A new generation prevents wraparound and wakes every old cursor.
            self.snapshot.cursor.run = SetupRunId::random();
            self.snapshot.cursor.revision = SetupRevision::ZERO;
            self.snapshot.state = SetupRunState::Unknown;
        }
    }
}

fn result(outcome: CommandOutcome) -> SetupCommandResult {
    match outcome {
        CommandOutcome::Exited(code) => SetupCommandResult::Exited { code },
        CommandOutcome::SpawnFailed => SetupCommandResult::SpawnFailed,
        CommandOutcome::TimedOut => SetupCommandResult::TimedOut,
        CommandOutcome::Cancelled => SetupCommandResult::Cancelled,
        CommandOutcome::IoFailed => SetupCommandResult::IoFailed,
    }
}

fn append_output(row: &mut SetupCommandProgress, bytes: &[u8]) {
    let before = row.output.bytes();
    if !row.truncated && before.len().saturating_add(bytes.len()) <= MAX_COMMAND_OUTPUT {
        let mut output = before.to_vec();
        output.extend_from_slice(bytes);
        row.output = TerminalBytes::new(output.into());
        return;
    }
    let mut output = Vec::with_capacity(MAX_COMMAND_OUTPUT);
    output.extend_from_slice(
        before
            .get(..HEAD_BUDGET.min(before.len()))
            .unwrap_or_default(),
    );
    let need_head = HEAD_BUDGET.saturating_sub(output.len());
    output.extend_from_slice(bytes.get(..need_head.min(bytes.len())).unwrap_or_default());
    output.extend_from_slice(MIDDLE_MARKER);
    if bytes.len() < TAIL_BUDGET {
        let needed = TAIL_BUDGET.saturating_sub(bytes.len());
        output.extend_from_slice(
            before
                .get(before.len().saturating_sub(needed)..)
                .unwrap_or_default(),
        );
        output.extend_from_slice(bytes);
    } else {
        output.extend_from_slice(
            bytes
                .get(bytes.len().saturating_sub(TAIL_BUDGET)..)
                .unwrap_or_default(),
        );
    }
    row.output = TerminalBytes::new(output.into());
    row.truncated = true;
}

#[cfg(test)]
mod tests;
