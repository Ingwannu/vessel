//! What the existing typed git outcomes mean for the menu: a new status,
//! a success said once, or a refusal kept beside the action. No outcome
//! is inferred from git's output text.

use vessel_proto::workspace::{
    BranchName, CommitId, CommitInfo, CommitOutcome, FetchOutcome, GitRefusal, GitStatus,
    PullOutcome, PushOutcome, RemoteName, RenameOutcome, StashList, StashOutcome, SwitchOutcome,
};

use super::super::problem::Problem;
use super::WriteIntent;
use super::state::RequestKey;

/// How a successful push changed the upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamChange {
    /// The upstream was already in place.
    Kept,
    /// This push set the upstream.
    Set,
}

/// A successful write's facts, for localized success text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Success {
    /// A new commit.
    Committed(CommitInfo),
    /// A fetch completed.
    Fetched,
    /// A push completed.
    Pushed {
        /// Its remote.
        remote: RemoteName,
        /// Its branch.
        branch: BranchName,
        /// Whether it established the upstream.
        upstream: UpstreamChange,
    },
    /// A fast-forward completed.
    Pulled {
        /// Before.
        from: CommitId,
        /// After.
        to: CommitId,
    },
    /// A pull had nothing to change.
    UpToDate,
    /// Checked out another branch.
    Switched(BranchName),
    /// Changed the branch's name.
    Renamed(BranchName),
    /// A stash action completed, with its new list.
    Stashed(StashList),
}

/// A write's answer, before any words or UI toolkit are involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// Stage, unstage or discard; use the returned status.
    Status(GitStatus),
    /// Another successful operation; read status again afterwards.
    Said(Success),
    /// A git refusal; keep it visible until dismissed or the next write.
    Refused(GitRefusal),
}

impl From<GitStatus> for Written {
    fn from(status: GitStatus) -> Self {
        Self::Status(status)
    }
}

impl From<CommitOutcome> for Written {
    fn from(outcome: CommitOutcome) -> Self {
        match outcome {
            CommitOutcome::Committed { commit, .. } => Self::Said(Success::Committed(commit)),
            CommitOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<FetchOutcome> for Written {
    fn from(outcome: FetchOutcome) -> Self {
        match outcome {
            FetchOutcome::Fetched => Self::Said(Success::Fetched),
            FetchOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<PushOutcome> for Written {
    fn from(outcome: PushOutcome) -> Self {
        match outcome {
            PushOutcome::Pushed {
                remote,
                branch,
                upstream_set,
                ..
            } => Self::Said(Success::Pushed {
                remote,
                branch,
                upstream: if upstream_set {
                    UpstreamChange::Set
                } else {
                    UpstreamChange::Kept
                },
            }),
            PushOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<PullOutcome> for Written {
    fn from(outcome: PullOutcome) -> Self {
        match outcome {
            PullOutcome::Pulled { from, to, .. } => Self::Said(Success::Pulled { from, to }),
            PullOutcome::UpToDate => Self::Said(Success::UpToDate),
            PullOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<SwitchOutcome> for Written {
    fn from(outcome: SwitchOutcome) -> Self {
        match outcome {
            SwitchOutcome::Switched { branch, .. } => Self::Said(Success::Switched(branch)),
            SwitchOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<RenameOutcome> for Written {
    fn from(outcome: RenameOutcome) -> Self {
        match outcome {
            RenameOutcome::Renamed { branch, .. } => Self::Said(Success::Renamed(branch)),
            RenameOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

impl From<StashOutcome> for Written {
    fn from(outcome: StashOutcome) -> Self {
        match outcome {
            StashOutcome::Done { stashes, .. } => Self::Said(Success::Stashed(stashes)),
            StashOutcome::Refused { refusal, .. } => Self::Refused(refusal),
        }
    }
}

/// Paseo's actions-store keeps a successful row visible for one second.
pub const SUCCESS_DISPLAY: std::time::Duration = std::time::Duration::from_millis(1000);

/// A successful keyed action's exact outcome and original intention. Only
/// the write tracker constructs it; a returned stash list cannot identify the
/// operation that produced it. Bounded facts are retained for one display phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessReceipt {
    key: RequestKey,
    intent: WriteIntent,
    outcome: Success,
}

impl SuccessReceipt {
    /// The completion that owns the display/reset phase.
    #[must_use]
    pub const fn key(&self) -> RequestKey {
        self.key
    }
    /// The original request, including the pinned saved entry when applicable.
    #[must_use]
    pub const fn intent(&self) -> &WriteIntent {
        &self.intent
    }
    /// All of the successful outcome's facts, never inferred from text.
    #[must_use]
    pub const fn outcome(&self) -> &Success {
        &self.outcome
    }
}

/// One outstanding write. Keys remain unique even if this value is
/// recreated, because the area's separate issuer survives it.
#[derive(Debug, Default)]
pub struct Writing {
    pending: Option<Pending>,
    success: Option<SuccessReceipt>,
}

#[derive(Debug)]
struct Pending {
    key: RequestKey,
    intent: WriteIntent,
}

impl Writing {
    /// Reserve the one write slot. A double click cannot send two writes.
    pub fn begin(&mut self, key: RequestKey) -> bool {
        self.begin_action(key, WriteIntent::Unspecified)
    }

    /// Reserve the slot with the intention whose pending label is displayed.
    pub fn begin_action(&mut self, key: RequestKey, intent: WriteIntent) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.success = None;
        self.pending = Some(Pending { key, intent });
        true
    }

    /// Whether a write is in flight.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// The pending intention is inseparable from its request key.
    #[must_use]
    pub const fn intent(&self) -> Option<&WriteIntent> {
        match &self.pending {
            Some(pending) => Some(&pending.intent),
            None => None,
        }
    }

    /// The last successful keyed action until reset or another write begins.
    #[must_use]
    pub const fn success(&self) -> Option<&SuccessReceipt> {
        self.success.as_ref()
    }

    /// A previous action's timer cannot clear a later success or pending write,
    /// including after the area follows another repository and comes back.
    pub fn expire_success(&mut self, key: RequestKey) -> bool {
        if self
            .success
            .as_ref()
            .is_some_and(|receipt| receipt.key == key)
        {
            self.success = None;
            true
        } else {
            false
        }
    }

    /// Consume exactly the pending reply, never a late/duplicate reply.
    pub fn finish(
        &mut self,
        key: RequestKey,
        result: Result<Written, Problem>,
    ) -> Option<Result<Written, Problem>> {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.key != key)
        {
            return None;
        }
        let pending = self.pending.take()?;
        // The caller still needs the outcome for repository/list refresh. Keep
        // its bounded facts alongside the keyed intention for the brief success
        // phase, rather than guessing an action from a refreshed stash list.
        self.success = match &result {
            Ok(Written::Said(outcome)) => Some(SuccessReceipt {
                key: pending.key,
                intent: pending.intent,
                outcome: outcome.clone(),
            }),
            Err(_) | Ok(Written::Status(_) | Written::Refused(_)) => None,
        };
        Some(result)
    }
}

impl Success {
    /// A completed write has the notice vocabulary's success meaning;
    /// no kit type or colour enters the action model.
    #[must_use]
    pub const fn alert_variant(&self) -> crate::alert::Variant {
        match self {
            Self::Committed(_)
            | Self::Fetched
            | Self::Pushed { .. }
            | Self::Pulled { .. }
            | Self::UpToDate
            | Self::Switched(_)
            | Self::Renamed(_)
            | Self::Stashed(_) => crate::alert::Variant::Success,
        }
    }
}
