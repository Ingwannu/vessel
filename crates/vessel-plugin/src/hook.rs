//! Paseo's fourteen hook names and which of them wait.
//!
//! The wait bit is `pluginHookRegistry` in
//! `packages/plugin/src/server/lifecycle.ts` lines 174 to 189. A waiting
//! hook can refuse the operation. The others only notify.

use std::fmt;
use std::time::Duration;

/// How long a waiting hook may run.
///
/// Paseo's `REQUEST_TIMEOUT_MS` is 30 seconds (`runtime.ts` line 41). The
/// node counts that around the call. This crate does not sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingHook;

impl WaitingHook {
    /// The deadline the node applies to `agent.create.before`.
    pub const LIMIT: Self = Self;

    /// The duration the node passes to its clock.
    #[must_use]
    pub const fn duration(self) -> Duration {
        Duration::from_secs(30)
    }
}

/// Whether the operation waits for the hook.
///
/// A boolean would not say which way a failure goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// A failure or a timeout fails the operation, and later handlers of
    /// this hook do not run.
    Blocks,
    /// A failure is logged. Later handlers of this hook still run.
    Continues,
}

/// One of Paseo's hook names.
///
/// Closed so a match cannot use `_` and a new name does not compile until
/// this enum grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    /// `workspace.create.before`
    WorkspaceCreateBefore,
    /// `workspace.create.after`
    WorkspaceCreateAfter,
    /// `workspace.archive.after`
    WorkspaceArchiveAfter,
    /// `worktree.create.before`
    WorktreeCreateBefore,
    /// `worktree.create.after`
    WorktreeCreateAfter,
    /// `agent.create.before`
    AgentCreateBefore,
    /// `agent.create.after`
    AgentCreateAfter,
    /// `agent.open.before`
    AgentOpenBefore,
    /// `agent.close.after`
    AgentCloseAfter,
    /// `agent.archive.after`
    AgentArchiveAfter,
    /// `agent.turn.before`
    AgentTurnBefore,
    /// `agent.turn.after`
    AgentTurnAfter,
    /// `agent.permission.before`
    AgentPermissionBefore,
    /// `agent.permission.after`
    AgentPermissionAfter,
}

impl Hook {
    /// Paseo's wait bit for this name.
    #[must_use]
    pub const fn wait(self) -> Wait {
        match self {
            Self::WorkspaceCreateBefore
            | Self::WorktreeCreateBefore
            | Self::WorktreeCreateAfter
            | Self::AgentCreateBefore
            | Self::AgentOpenBefore => Wait::Blocks,
            Self::WorkspaceCreateAfter
            | Self::WorkspaceArchiveAfter
            | Self::AgentCreateAfter
            | Self::AgentCloseAfter
            | Self::AgentArchiveAfter
            | Self::AgentTurnBefore
            | Self::AgentTurnAfter
            | Self::AgentPermissionBefore
            | Self::AgentPermissionAfter => Wait::Continues,
        }
    }

    /// The name a person sees. Not a string the code matches on.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WorkspaceCreateBefore => "workspace.create.before",
            Self::WorkspaceCreateAfter => "workspace.create.after",
            Self::WorkspaceArchiveAfter => "workspace.archive.after",
            Self::WorktreeCreateBefore => "worktree.create.before",
            Self::WorktreeCreateAfter => "worktree.create.after",
            Self::AgentCreateBefore => "agent.create.before",
            Self::AgentCreateAfter => "agent.create.after",
            Self::AgentOpenBefore => "agent.open.before",
            Self::AgentCloseAfter => "agent.close.after",
            Self::AgentArchiveAfter => "agent.archive.after",
            Self::AgentTurnBefore => "agent.turn.before",
            Self::AgentTurnAfter => "agent.turn.after",
            Self::AgentPermissionBefore => "agent.permission.before",
            Self::AgentPermissionAfter => "agent.permission.after",
        }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::{Hook, Wait};

    #[test]
    fn every_name_has_paseos_wait_bit() {
        let bits = [
            (Hook::WorkspaceCreateBefore, Wait::Blocks),
            (Hook::WorkspaceCreateAfter, Wait::Continues),
            (Hook::WorkspaceArchiveAfter, Wait::Continues),
            (Hook::WorktreeCreateBefore, Wait::Blocks),
            (Hook::WorktreeCreateAfter, Wait::Blocks),
            (Hook::AgentCreateBefore, Wait::Blocks),
            (Hook::AgentCreateAfter, Wait::Continues),
            (Hook::AgentOpenBefore, Wait::Blocks),
            (Hook::AgentCloseAfter, Wait::Continues),
            (Hook::AgentArchiveAfter, Wait::Continues),
            (Hook::AgentTurnBefore, Wait::Continues),
            (Hook::AgentTurnAfter, Wait::Continues),
            (Hook::AgentPermissionBefore, Wait::Continues),
            (Hook::AgentPermissionAfter, Wait::Continues),
        ];
        for (hook, wait) in bits {
            assert_eq!(hook.wait(), wait, "{}", hook.name());
        }
        assert_eq!(Hook::AgentCreateBefore.name(), "agent.create.before");
        assert_eq!(Hook::AgentCreateAfter.name(), "agent.create.after");
    }
}
