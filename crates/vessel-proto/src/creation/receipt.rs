//! One receipt owns reserved identities and serializes side-effect boundaries.

use serde::{Deserialize, Serialize};

use super::{CreationError, Fingerprint, Revision};
use crate::{ErrorCode, SessionId, TerminalId, WorkspaceId, WorkspaceOperationId};

/// Different create operations can use the same opaque client key independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationKind"))]
pub enum Kind {
    /// Provision a fresh independent workspace, optionally with its first resource.
    Workspace,
    /// Add a chat to an existing workspace.
    Agent,
    /// Add a shell/profile terminal to an existing workspace.
    Terminal,
}

/// A single reserved resource, never an ambiguous pair of optional ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationResource"))]
pub enum Resource {
    /// Reserved session identity, which is not proof of registration.
    Agent {
        /// Session id reserved before starting its process.
        session: SessionId,
    },
    /// Reserved terminal identity, which is not proof of launch.
    Terminal {
        /// Terminal id reserved before opening its process.
        terminal: TerminalId,
    },
}

/// Existing workspace membership must not claim the whole workspace exclusively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationWorkspaceClaim"))]
pub enum WorkspaceClaim {
    /// This operation alone provisions the reserved workspace identity.
    Create,
    /// Several new chats/terminals can share this already-created workspace.
    Existing,
}

/// Message presence is retained without putting its text into the receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationFirstInput"))]
pub enum FirstInput {
    /// Empty workspace or a launch with no first message/typed shell command.
    Absent,
    /// Unkeyed historical/terminal delivery. New chats use Identified.
    Present,
    /// A new chat retains the caller's canonical input identity before spawn.
    Identified {
        /// The same id is used for every first-prompt retry on this child.
        message: crate::timeline::ClientMessageId,
    },
}

impl FirstInput {
    /// Receipt observation retains unkeyed historical rows without fabricating ids.
    #[must_use]
    pub const fn message(&self) -> Option<&crate::timeline::ClientMessageId> {
        match self {
            Self::Identified { message } => Some(message),
            Self::Absent | Self::Present => None,
        }
    }
    const fn required(&self) -> bool {
        match self {
            Self::Absent => false,
            Self::Present | Self::Identified { .. } => true,
        }
    }
}

/// Resource identities reserved durably before any provision or spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CreationReservation"))]
pub struct Reservation {
    /// Shared operation identity from ADR 0082, not an alternate workspace id.
    pub operation: WorkspaceOperationId,
    /// Exact workspace record, independent of its cwd/backing.
    pub workspace: WorkspaceId,
    /// Whether this receipt claims new workspace provisioning.
    pub workspace_claim: WorkspaceClaim,
    /// None creates an empty workspace; otherwise registration is a later stage.
    pub resource: Option<Resource>,
    /// A required first delivery cannot silently be skipped by completion.
    pub first_input: FirstInput,
}

/// Write a stage marker before crossing any side-effect boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationStage"))]
pub enum Stage {
    /// Workspace/backing provision.
    Workspace,
    /// Agent or terminal registration.
    Resource,
    /// First chat message or typed shell command.
    Prompt,
}

/// A known refusal can be retried; uncertain effects require observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationCertainty"))]
pub enum Certainty {
    /// Side effect was explicitly refused before completion.
    Known,
    /// A side effect may already have happened; do not replay.
    Unknown,
}

/// Reference creation lifecycle, with a resource-neutral registration phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "phase", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "CreationPhase"))]
pub enum Phase {
    /// Key/intent and ids committed before provisioning.
    Accepted,
    /// Workspace provision committed, independently of agent/terminal startup.
    WorkspaceReady,
    /// Registration committed; prompt failure belongs to this resource.
    ResourceReady,
    /// First-message delivery accepted by the resource.
    PromptStarted,
    /// All requested stages completed.
    Completed,
    /// Failure at a named side-effect boundary.
    Failed {
        /// The last attempted side effect.
        stage: Stage,
        /// Whether retry could duplicate that effect.
        certainty: Certainty,
        /// Safe failure code, never prompt/template contents or raw diagnostics.
        code: ErrorCode,
    },
}

/// Observation contains only identities and safe lifecycle metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CreationSnapshot"))]
pub struct Snapshot {
    /// Kind namespaces the caller key.
    pub kind: Kind,
    /// Identities were reserved, not necessarily created yet; inspect phase.
    pub reserved: Reservation,
    /// Committed publication ordering.
    pub revision: Revision,
    /// Last committed completed stage or failure.
    pub phase: Phase,
}

/// Store-owned record, including the private intent fingerprint/in-flight marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ReceiptWire", into = "ReceiptWire")]
pub struct Receipt {
    snapshot: Snapshot,
    fingerprint: Fingerprint,
    in_flight: Option<Stage>,
}

#[derive(Serialize, Deserialize)]
struct ReceiptWire {
    snapshot: Snapshot,
    fingerprint: Fingerprint,
    in_flight: Option<Stage>,
}

impl TryFrom<ReceiptWire> for Receipt {
    type Error = CreationError;

    fn try_from(wire: ReceiptWire) -> Result<Self, Self::Error> {
        let mut receipt = Self::admitted(
            wire.snapshot.kind,
            wire.snapshot.reserved.clone(),
            wire.fingerprint,
        )?;
        let reserved = &wire.snapshot.reserved;
        let valid_phase = match wire.snapshot.phase {
            Phase::Accepted | Phase::Completed => true,
            Phase::WorkspaceReady => reserved.workspace_claim == WorkspaceClaim::Create,
            Phase::ResourceReady => reserved.resource.is_some(),
            Phase::PromptStarted => reserved.resource.is_some() && reserved.first_input.required(),
            Phase::Failed { stage, .. } => match stage {
                Stage::Workspace => reserved.workspace_claim == WorkspaceClaim::Create,
                Stage::Resource => reserved.resource.is_some(),
                Stage::Prompt => reserved.resource.is_some() && reserved.first_input.required(),
            },
        };
        if !valid_phase {
            return Err(CreationError::InvalidTransition);
        }
        receipt.snapshot = wire.snapshot;
        if let Some(stage) = wire.in_flight {
            receipt.begin(stage)?;
        }
        Ok(receipt)
    }
}

impl From<Receipt> for ReceiptWire {
    fn from(receipt: Receipt) -> Self {
        Self {
            snapshot: receipt.snapshot,
            fingerprint: receipt.fingerprint,
            in_flight: receipt.in_flight,
        }
    }
}

/// A driver advances only after its current stage result is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Commit a marker before calling the stage's effect.
    Begin(Stage),
    /// Commit the effect's result before beginning another stage.
    Finished(Stage),
    /// All requested effects have completed.
    Complete,
    /// Explicit refusal or uncertain side effect, never implicit rollback.
    Fail {
        /// Safe error information.
        code: ErrorCode,
        /// Whether this particular side effect may have committed.
        certainty: Certainty,
    },
    /// A restart found an unfinished marker; mark it unknown instead of replaying.
    Recover,
}

impl Receipt {
    /// Bind validated intent and reserved ids before any side effect starts.
    /// # Errors
    /// Resource/claim/input combinations incompatible with the operation kind.
    pub fn admitted(
        kind: Kind,
        reserved: Reservation,
        fingerprint: Fingerprint,
    ) -> Result<Self, CreationError> {
        let valid = match (kind, reserved.workspace_claim, reserved.resource) {
            (Kind::Workspace, WorkspaceClaim::Create, None) => {
                reserved.first_input == FirstInput::Absent
            }
            (Kind::Workspace, WorkspaceClaim::Create, Some(_))
            | (Kind::Agent, WorkspaceClaim::Existing, Some(Resource::Agent { .. }))
            | (Kind::Terminal, WorkspaceClaim::Existing, Some(Resource::Terminal { .. })) => true,
            (Kind::Workspace, WorkspaceClaim::Existing, _)
            | (Kind::Agent | Kind::Terminal, WorkspaceClaim::Create, _)
            | (Kind::Agent | Kind::Terminal, WorkspaceClaim::Existing, None)
            | (Kind::Agent, WorkspaceClaim::Existing, Some(Resource::Terminal { .. }))
            | (Kind::Terminal, WorkspaceClaim::Existing, Some(Resource::Agent { .. })) => false,
        };
        if !valid
            || (reserved.first_input.message().is_some()
                && !matches!(reserved.resource, Some(Resource::Agent { .. })))
        {
            return Err(CreationError::InvalidTransition);
        }
        Ok(Self {
            snapshot: Snapshot {
                kind,
                reserved,
                revision: Revision::INITIAL,
                phase: Phase::Accepted,
            },
            fingerprint,
            in_flight: None,
        })
    }

    /// Only safe projection may leave the storage/service boundary.
    #[must_use]
    pub const fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Compare normalized intent without retaining its prompts or environment.
    #[must_use]
    pub const fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    /// Recovery/admission distinguishes active work from an absent receipt.
    #[must_use]
    pub const fn in_flight(&self) -> Option<Stage> {
        self.in_flight
    }

    /// Validate transition against the current receipt; invalid calls change nothing.
    /// # Errors
    /// A stale revision, exhausted counter or incompatible stage/outcome.
    pub fn advance(&mut self, expected: Revision, update: Advance) -> Result<(), CreationError> {
        if expected != self.snapshot.revision {
            return Err(CreationError::StaleRevision);
        }
        let revision = expected.next()?;
        let mut next = self.clone();
        next.apply(update)?;
        next.snapshot.revision = revision;
        *self = next;
        Ok(())
    }

    fn apply(&mut self, update: Advance) -> Result<(), CreationError> {
        match update {
            Advance::Begin(stage) => self.begin(stage),
            Advance::Finished(stage) => {
                if self.in_flight != Some(stage) {
                    return Err(CreationError::InvalidTransition);
                }
                self.snapshot.phase = match stage {
                    Stage::Workspace => Phase::WorkspaceReady,
                    Stage::Resource => Phase::ResourceReady,
                    Stage::Prompt => Phase::PromptStarted,
                };
                self.in_flight = None;
                Ok(())
            }
            Advance::Complete => {
                if self.in_flight.is_some() || !self.completable() {
                    return Err(CreationError::InvalidTransition);
                }
                self.snapshot.phase = Phase::Completed;
                Ok(())
            }
            Advance::Fail { code, certainty } => {
                let stage = self.in_flight.ok_or(CreationError::InvalidTransition)?;
                self.snapshot.phase = Phase::Failed {
                    stage,
                    certainty,
                    code,
                };
                self.in_flight = None;
                Ok(())
            }
            Advance::Recover => {
                let stage = self.in_flight.ok_or(CreationError::InvalidTransition)?;
                self.snapshot.phase = Phase::Failed {
                    stage,
                    certainty: Certainty::Unknown,
                    code: ErrorCode::Unavailable,
                };
                self.in_flight = None;
                Ok(())
            }
        }
    }

    fn begin(&mut self, stage: Stage) -> Result<(), CreationError> {
        if self.in_flight.is_some() {
            return Err(CreationError::InvalidTransition);
        }
        let allowed = match self.snapshot.phase {
            Phase::Accepted => match self.snapshot.reserved.workspace_claim {
                WorkspaceClaim::Create => stage == Stage::Workspace,
                WorkspaceClaim::Existing => stage == Stage::Resource,
            },
            Phase::WorkspaceReady => {
                stage == Stage::Resource && self.snapshot.reserved.resource.is_some()
            }
            Phase::ResourceReady => {
                stage == Stage::Prompt && self.snapshot.reserved.first_input.required()
            }
            Phase::Failed {
                stage: failed,
                certainty: Certainty::Known,
                ..
            } => stage == failed,
            Phase::PromptStarted
            | Phase::Completed
            | Phase::Failed {
                certainty: Certainty::Unknown,
                ..
            } => false,
        };
        if !allowed {
            return Err(CreationError::InvalidTransition);
        }
        self.in_flight = Some(stage);
        Ok(())
    }

    fn completable(&self) -> bool {
        match self.snapshot.phase {
            Phase::WorkspaceReady => self.snapshot.reserved.resource.is_none(),
            Phase::ResourceReady => self.snapshot.reserved.first_input == FirstInput::Absent,
            Phase::PromptStarted => true,
            Phase::Accepted | Phase::Completed | Phase::Failed { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests;
