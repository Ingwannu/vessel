//! Navigation-safe creation transitions, without node or toolkit dependencies.
//!
//! [Decision Log]
//! - Intent: keep duplicate submits and late completions away from other drafts.
//! - Constraint: a disconnect does not prove that a side effect did not happen.
//! - Choice: unknown outcomes lock submission until reconciled; leaving only
//!   relinquishes navigation and never cancels a provision already accepted.
//! - Cost: recovery needs the node's receipt; a blind retry is not offered.

use vessel_proto::ErrorCode;

use super::Ticket;

/// Side-effect boundaries shared by chat and terminal creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Provision the independent workspace and any required backing.
    Workspace,
    /// Register the chat agent or shell terminal.
    Resource,
    /// Deliver the first chat message or blank-shell input.
    InitialInput,
}

/// Whether completion still owns the route where submission began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    /// The person is still waiting in this submission's destination.
    Foreground,
    /// The person left; completion may update data but must not navigate.
    Background,
}

/// Safe presentation reasons; raw diagnostics remain at the call boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No command reached the link's runtime.
    QueueFull,
    /// A required capability is absent.
    Unsupported,
    /// No node was connected, before any side effect was sent.
    Offline,
    /// The node explicitly refused the operation.
    Refused(ErrorCode),
    /// The connection or deadline ended after a side effect could have begun.
    OutcomeUnknown,
}

impl Failure {
    /// The legacy placement walk may already have made a worktree or run setup
    /// before a later RPC fails. A node refusal alone cannot prove no effect.
    #[must_use]
    pub const fn placement(problem: &crate::workspace::problem::Problem) -> Self {
        use crate::workspace::problem::Problem;
        match problem {
            Problem::Older => Self::Unsupported,
            Problem::Unsent => Self::QueueFull,
            Problem::Node { .. } | Problem::Gone(_) => Self::OutcomeUnknown,
        }
    }
}

/// One active draft submission, retained independently of its rendered route.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Flow {
    /// The draft can be edited and submitted.
    #[default]
    Editing,
    /// A command has been accepted locally and awaits its next stage.
    Creating {
        /// Correlates every callback and the attachment payload.
        ticket: Ticket,
        /// The last known side-effect boundary.
        stage: Stage,
        /// Whether completion may take the route.
        navigation: Navigation,
    },
    /// The creation completed and consumed its submitted payload.
    Complete {
        /// Which payload was consumed.
        ticket: Ticket,
    },
    /// A known refusal permits retry; unknown outcomes require reconciliation.
    Failed {
        /// Which request must be reconciled or retried.
        ticket: Ticket,
        /// The side effect that did not finish.
        stage: Stage,
        /// Safe failure reason.
        failure: Failure,
        /// Route ownership retained across failure/reconciliation.
        navigation: Navigation,
    },
}

/// Whether a callback belonged to the active submission and advanced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Applied to the active operation.
    Applied,
    /// Stale, regressive or incompatible with the current state.
    Ignored,
}

/// Completion may navigate only while its original destination remains active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Open the created resource.
    Open,
    /// Retain it in the registry without changing the current route.
    Background,
    /// An unrelated callback; consume no draft payload.
    Ignored,
}

impl Flow {
    /// Which payload a consumer may mutate; completed callbacks cannot recur.
    #[must_use]
    pub const fn ticket(&self) -> Option<Ticket> {
        match self {
            Self::Creating { ticket, .. } | Self::Failed { ticket, .. } => Some(*ticket),
            Self::Editing | Self::Complete { .. } => None,
        }
    }

    /// Only a known refusal may be resubmitted without observing a receipt.
    #[must_use]
    pub const fn may_submit(&self) -> bool {
        match self {
            Self::Editing | Self::Complete { .. } => true,
            Self::Creating { .. } => false,
            Self::Failed { failure, .. } => match failure {
                Failure::OutcomeUnknown => false,
                Failure::QueueFull
                | Failure::Unsupported
                | Failure::Offline
                | Failure::Refused(_) => true,
            },
        }
    }

    /// Lock the payload before dispatch, preventing duplicate keyboard/click sends.
    #[must_use]
    pub fn begin(&mut self, ticket: Ticket, stage: Stage) -> Transition {
        if !self.may_submit() {
            return Transition::Ignored;
        }
        *self = Self::Creating {
            ticket,
            stage,
            navigation: Navigation::Foreground,
        };
        Transition::Applied
    }

    /// A person's explicit retry reuses the committed resource, never a new create.
    #[must_use]
    pub fn retry_input(&mut self, incoming: Ticket) -> Transition {
        let Self::Failed {
            ticket,
            failure,
            stage: Stage::InitialInput,
            ..
        } = self
        else {
            return Transition::Ignored;
        };
        if *ticket != incoming || *failure == Failure::OutcomeUnknown {
            return Transition::Ignored;
        }
        *self = Self::Creating {
            ticket: incoming,
            stage: Stage::InitialInput,
            navigation: Navigation::Foreground,
        };
        Transition::Applied
    }

    /// A later stage cannot be overwritten by an older progress update.
    #[must_use]
    pub fn progress(&mut self, incoming: Ticket, next: Stage) -> Transition {
        match self {
            Self::Creating { ticket, stage, .. } if *ticket == incoming && next >= *stage => {
                *stage = next;
                Transition::Applied
            }
            Self::Editing | Self::Complete { .. } | Self::Failed { .. } | Self::Creating { .. } => {
                Transition::Ignored
            }
        }
    }

    /// Leaving is sticky even if the person later visits a different new-chat route.
    pub fn leave(&mut self) {
        match self {
            Self::Creating { navigation, .. } | Self::Failed { navigation, .. } => {
                *navigation = Navigation::Background;
            }
            Self::Editing | Self::Complete { .. } => {}
        }
    }

    /// Settle only the active payload, including an observed uncertain receipt.
    #[must_use]
    pub fn complete(&mut self, incoming: Ticket) -> Completion {
        let navigation = match self {
            Self::Creating {
                ticket, navigation, ..
            }
            | Self::Failed {
                ticket,
                failure: Failure::OutcomeUnknown,
                navigation,
                ..
            } if *ticket == incoming => *navigation,
            Self::Editing | Self::Complete { .. } | Self::Failed { .. } | Self::Creating { .. } => {
                return Completion::Ignored;
            }
        };
        *self = Self::Complete { ticket: incoming };
        match navigation {
            Navigation::Foreground => Completion::Open,
            Navigation::Background => Completion::Background,
        }
    }

    /// Retain stage and route ownership when a correlated call fails.
    #[must_use]
    pub fn fail(&mut self, incoming: Ticket, failure: Failure) -> Transition {
        let Self::Creating {
            ticket,
            stage,
            navigation,
        } = *self
        else {
            return Transition::Ignored;
        };
        if incoming != ticket {
            return Transition::Ignored;
        }
        *self = Self::Failed {
            ticket,
            stage,
            failure,
            navigation,
        };
        Transition::Applied
    }
}

#[cfg(test)]
mod tests;
