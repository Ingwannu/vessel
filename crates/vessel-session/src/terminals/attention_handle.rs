//! Semantic activity changes share the existing info watch's mutation lock.
//! The once-only terminal claim lives as long as its non-reused TerminalId.
use super::{AttentionError, TerminalHandle, attention};
use std::sync::PoisonError;
use vessel_proto::notifications::{
    NotificationClaim, TerminalActivity, TerminalActivitySignal, TerminalAttention,
    TerminalAttentionRevision,
};

impl TerminalHandle {
    /// Apply a trusted semantic signal against the exact observed revision.
    /// # Errors
    /// A stale revision or revision exhaustion leaves current state unchanged.
    pub fn report_activity(
        &self,
        revision: TerminalAttentionRevision,
        signal: TerminalActivitySignal,
    ) -> Result<TerminalAttention, AttentionError> {
        self.change_attention(revision, true, |current| {
            attention::signal(current, signal, chrono::Utc::now())
        })
    }
    /// Clear only the attention the person actually saw; never future work.
    /// # Errors
    /// A stale revision or revision exhaustion leaves current state unchanged.
    pub fn clear_attention(
        &self,
        revision: TerminalAttentionRevision,
    ) -> Result<TerminalAttention, AttentionError> {
        self.change_attention(revision, false, |current| {
            attention::clear(current, revision, chrono::Utc::now())
        })
    }
    fn change_attention(
        &self,
        revision: TerminalAttentionRevision,
        require_live: bool,
        change: impl FnOnce(TerminalAttention) -> Result<TerminalAttention, AttentionError>,
    ) -> Result<TerminalAttention, AttentionError> {
        let mut result = Err(AttentionError::Stale);
        let mut change = Some(change);
        self.0.info.send_if_modified(|info| {
            if require_live && !info.is_live() {
                result = Err(AttentionError::Ended);
                return false;
            }
            let Some(current) = info
                .attention
                .filter(|current| current.revision == revision)
            else {
                return false;
            };
            let Some(change) = change.take() else {
                return false;
            };
            match change(current) {
                Ok(next) if next == current => {
                    result = Ok(current);
                    false
                }
                Ok(next) => {
                    let Some(revision) = info.revision.checked_add(1) else {
                        result = Err(AttentionError::Exhausted);
                        return false;
                    };
                    info.attention = Some(next);
                    info.revision = revision;
                    result = Ok(next);
                    true
                }
                Err(error) => {
                    result = Err(error);
                    false
                }
            }
        });
        result
    }
    /// Recheck a terminal source after asynchronous recipient preparation.
    #[must_use]
    pub fn attention_current(&self, revision: TerminalAttentionRevision) -> bool {
        self.0.info.borrow().attention.is_some_and(|current| {
            current.revision == revision
                && matches!(current.activity, TerminalActivity::Attention { .. })
        })
    }
    /// Consume one current attention revision at most once. The node must
    /// elect and authorize the connection before calling this local mutation.
    #[must_use]
    pub fn claim_attention(&self, revision: TerminalAttentionRevision) -> NotificationClaim {
        let mut claimed = self
            .0
            .attention_claim
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !self.attention_current(revision) {
            return NotificationClaim::NoAttention;
        }
        if *claimed == Some(revision) {
            return NotificationClaim::AlreadyClaimed;
        }
        *claimed = Some(revision);
        if self.attention_current(revision) {
            NotificationClaim::Admitted
        } else {
            NotificationClaim::NoAttention
        }
    }
}
