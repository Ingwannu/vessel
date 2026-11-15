//! Modern lifecycle names and wait policy shared with the helper.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Paseo server/lifecycle.ts:174–192. Aliases from older plugin releases are
/// deliberately absent from the native author contract (ADR 0079).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HookName {
    /// May rewrite a workspace creation request before core validation.
    #[serde(rename = "workspace.create.before")]
    WorkspaceCreateBefore,
    /// Observes successful workspace creation without holding the caller.
    #[serde(rename = "workspace.create.after")]
    WorkspaceCreateAfter,
    /// Observes successful workspace archive.
    #[serde(rename = "workspace.archive.after")]
    WorkspaceArchiveAfter,
    /// May rewrite a worktree creation request before core validation.
    #[serde(rename = "worktree.create.before")]
    WorktreeCreateBefore,
    /// Waited setup after the worktree exists.
    #[serde(rename = "worktree.create.after")]
    WorktreeCreateAfter,
    /// Runs on shared agent creation after draining refusal.
    #[serde(rename = "agent.create.before")]
    AgentCreateBefore,
    /// Observes successful shared agent creation.
    #[serde(rename = "agent.create.after")]
    AgentCreateAfter,
    /// May change launch-only values before core launch validation.
    #[serde(rename = "agent.open.before")]
    AgentOpenBefore,
    /// Observes an agent closing.
    #[serde(rename = "agent.close.after")]
    AgentCloseAfter,
    /// Observes an agent archive.
    #[serde(rename = "agent.archive.after")]
    AgentArchiveAfter,
    /// Observes a turn starting.
    #[serde(rename = "agent.turn.before")]
    AgentTurnBefore,
    /// Observes a turn ending.
    #[serde(rename = "agent.turn.after")]
    AgentTurnAfter,
    /// Observes a permission request.
    #[serde(rename = "agent.permission.before")]
    AgentPermissionBefore,
    /// Observes a permission decision.
    #[serde(rename = "agent.permission.after")]
    AgentPermissionAfter,
}

impl HookName {
    /// Every supported hook, also used to verify the complete wait table.
    pub const ALL: [Self; 14] = [
        Self::WorkspaceCreateBefore,
        Self::WorkspaceCreateAfter,
        Self::WorkspaceArchiveAfter,
        Self::WorktreeCreateBefore,
        Self::WorktreeCreateAfter,
        Self::AgentCreateBefore,
        Self::AgentCreateAfter,
        Self::AgentOpenBefore,
        Self::AgentCloseAfter,
        Self::AgentArchiveAfter,
        Self::AgentTurnBefore,
        Self::AgentTurnAfter,
        Self::AgentPermissionBefore,
        Self::AgentPermissionAfter,
    ];

    /// Core callers wait only where the reference contract waits. A transport
    /// deadline still bounds delivery of a notification to a blocked helper.
    #[must_use]
    pub const fn wait(self) -> HookWait {
        match self {
            Self::WorkspaceCreateBefore
            | Self::WorktreeCreateBefore
            | Self::WorktreeCreateAfter
            | Self::AgentCreateBefore
            | Self::AgentOpenBefore => HookWait::Blocks,
            Self::WorkspaceCreateAfter
            | Self::WorkspaceArchiveAfter
            | Self::AgentCreateAfter
            | Self::AgentCloseAfter
            | Self::AgentArchiveAfter
            | Self::AgentTurnBefore
            | Self::AgentTurnAfter
            | Self::AgentPermissionBefore
            | Self::AgentPermissionAfter => HookWait::Notifies,
        }
    }
}

/// The core operation's ordering, distinct from helper delivery deadlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookWait {
    /// Refusal or timeout prevents the core operation from proceeding.
    Blocks,
    /// Completion does not hold the successful core operation.
    Notifies,
}

/// A complete replacement of one installation's subscriptions. A set cannot
/// accidentally run a hook twice because the helper registered it twice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<HookName>", into = "Vec<HookName>")]
pub struct HookSubscriptions(BTreeSet<HookName>);

/// A duplicated lifecycle subscription is a malformed declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("plugin lifecycle subscriptions contain a duplicate hook")]
pub struct DuplicateHook;

impl TryFrom<Vec<HookName>> for HookSubscriptions {
    type Error = DuplicateHook;
    fn try_from(values: Vec<HookName>) -> Result<Self, Self::Error> {
        let mut hooks = BTreeSet::new();
        for hook in values {
            if !hooks.insert(hook) {
                return Err(DuplicateHook);
            }
        }
        Ok(Self(hooks))
    }
}

impl From<HookSubscriptions> for Vec<HookName> {
    fn from(hooks: HookSubscriptions) -> Self {
        hooks.0.into_iter().collect()
    }
}

impl HookSubscriptions {
    /// Whether this generation currently subscribes to the named event.
    #[must_use]
    pub fn contains(&self, hook: HookName) -> bool {
        self.0.contains(&hook)
    }
}
