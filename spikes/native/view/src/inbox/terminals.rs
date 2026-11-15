//! Semantic terminal facts share the Inbox snapshot. No parser or terminal
//! resident lives here; plain shells remain Unknown until a trusted producer.
use super::{Snapshot, buckets::Bucket};
use vessel_proto::notifications::{TerminalActivity, TerminalAttention, TerminalAttentionReason};
use vessel_proto::{NodeId, SessionTitle, TerminalId};

/// One exact terminal destination, independent of sessions and local labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    /// Authenticated reporting node.
    pub node: NodeId,
    /// Node-owned terminal identity.
    pub terminal: TerminalId,
}
/// Metadata-only projection; the existing terminal owner retains its renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Exact source on this node.
    pub terminal: TerminalId,
    /// User-visible local list label, never an OS payload.
    pub title: Option<SessionTitle>,
    /// Explicit node state; absence remains unknown.
    pub attention: Option<TerminalAttention>,
    /// Process failure, distinct from whether its attention was read.
    pub failed: bool,
}
impl Row {
    /// The same priority buckets used by sessions and workspace rollups.
    #[must_use]
    pub fn bucket(&self) -> Bucket {
        let activity = self.attention.map(|state| state.activity);
        if matches!(
            activity,
            Some(TerminalActivity::Attention {
                reason: TerminalAttentionReason::NeedsInput
            })
        ) {
            Bucket::NeedsInput
        } else if self.failed {
            Bucket::Failed
        } else if activity == Some(TerminalActivity::Working) {
            Bucket::Working
        } else if matches!(
            activity,
            Some(TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished
            })
        ) {
            Bucket::Review
        } else {
            Bucket::Done
        }
    }
}
/// Only explicitly supported reasons authorize a generic OS claim.
#[must_use]
pub fn announces(state: TerminalAttention) -> bool {
    matches!(
        state.activity,
        TerminalActivity::Attention {
            reason: TerminalAttentionReason::Finished | TerminalAttentionReason::NeedsInput
        }
    )
}
/// First hydration and reconnect stay silent; a changed exact revision may
/// announce once. Re-reading or clearing the same revision never replays it.
#[must_use]
pub fn fresh(previous: &Snapshot, next: &Snapshot) -> Vec<TerminalAttention> {
    let Some(previous) = previous.terminals.as_ref() else {
        return Vec::new();
    };
    next.terminals
        .iter()
        .flatten()
        .filter_map(|row| {
            let state = row.attention?;
            (announces(state)
                && !row.failed
                && previous
                    .iter()
                    .find(|known| known.terminal == row.terminal)
                    .and_then(|known| known.attention)
                    .is_none_or(|known| known.revision < state.revision))
            .then_some(state)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_shell_is_unknown_and_pending_wins_over_failure() {
        let terminal = TerminalId::random();
        let mut row = Row {
            terminal,
            title: None,
            attention: None,
            failed: false,
        };
        assert_eq!(row.bucket(), Bucket::Done);
        row.failed = true;
        assert_eq!(row.bucket(), Bucket::Failed);
        let mut state = TerminalAttention {
            terminal,
            revision: vessel_proto::notifications::TerminalAttentionRevision::new(1),
            activity: TerminalActivity::Working,
            changed_at: chrono::DateTime::UNIX_EPOCH,
        };
        row.attention = Some(state);
        assert_eq!(row.bucket(), Bucket::Failed);
        state.activity = TerminalActivity::Attention {
            reason: TerminalAttentionReason::NeedsInput,
        };
        row.attention = Some(state);
        assert_eq!(row.bucket(), Bucket::NeedsInput);
        assert!(announces(state));
        state.activity = TerminalActivity::Attention {
            reason: TerminalAttentionReason::Unknown,
        };
        assert!(!announces(state));
        state.activity = TerminalActivity::Unknown;
        assert!(!announces(state));
    }
}
