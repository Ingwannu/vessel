//! Actual runner boundaries, independent of a log's printable contents.

use std::time::Duration;

use vessel_proto::workspace::{ProjectCommand, SetupFailure};

/// A command's zero-based position in this exact invocation's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandIndex(pub(super) usize);

impl CommandIndex {
    /// The zero-based list position.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// What actually ended a command; cancellation is not an inferred exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The reaped process's code, absent for a signal or unknown exit.
    Exited(Option<i32>),
    /// The shell could not be started.
    SpawnFailed,
    /// The invocation's deadline expired and its process group was stopped.
    TimedOut,
    /// The invocation was explicitly stopped.
    Cancelled,
    /// The command or log could not be prepared or written.
    IoFailed,
}

impl CommandOutcome {
    pub(super) fn failure(self, index: CommandIndex) -> Option<SetupFailure> {
        let command = u32::try_from(index.get()).unwrap_or(u32::MAX);
        match self {
            Self::Exited(Some(0)) => None,
            Self::Exited(code) => Some(SetupFailure::Exit { command, code }),
            Self::Cancelled => Some(SetupFailure::Exit {
                command,
                code: None,
            }),
            Self::TimedOut => Some(SetupFailure::TimedOut),
            Self::SpawnFailed | Self::IoFailed => Some(SetupFailure::Spawn),
        }
    }
}

/// Borrowed events delivered inline. Consumers copy only bounded state;
/// they must not do disk or network I/O in this synchronous callback.
#[derive(Debug)]
pub enum CommandEvent<'a> {
    /// The runner is preparing this command; spawning can still fail.
    Started {
        /// Position within this invocation.
        index: CommandIndex,
        /// The exact already-approved command.
        command: &'a ProjectCommand,
    },
    /// Bytes read from this command's stdout/stderr, before global log caps.
    Output {
        /// Position within this invocation.
        index: CommandIndex,
        /// Raw output; it cannot create another command boundary.
        bytes: &'a [u8],
    },
    /// This command ended, including preparation or log I/O failures.
    Finished {
        /// Position within this invocation.
        index: CommandIndex,
        /// Monotonic elapsed execution/preparation time.
        elapsed: Duration,
        /// Typed terminal result.
        outcome: CommandOutcome,
    },
}
