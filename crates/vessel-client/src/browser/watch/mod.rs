//! Staged ephemeral watch validation, never a durable/resumable event log.
//!
//! The `browser: wire qualified browser watch transport` commit removes the
//! outer cfg(test) only with the actual node stream, router and client consumer.
//! This checker binds metadata; it authorizes no principal, origin or engine,
//! decodes no pixels and never turns a no-engine result into a frame.
#[cfg(test)]
mod tests;

use vessel_proto::browser::{
    BrowserChange, BrowserId, BrowserProfile, BrowserRevision, BrowserUpdate, BrowserUpdateSeq,
    BrowserUrl, BrowserViewport, BrowserWatched,
};
use vessel_proto::{NodeId, SubscriptionId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum WatchError {
    #[error("browser watch is closed")]
    Closed,
    #[error("browser watch update has a different immutable scope")]
    Scope,
    #[error("browser watch update order is discontinuous")]
    Sequence,
    #[error("browser watch update has a stale or unannounced context revision")]
    Revision,
    #[error("browser watch frame disagrees with its announced CSS viewport")]
    Viewport,
    #[error("browser watch sequence is exhausted")]
    Exhausted,
}
#[derive(Debug, Clone, Copy)]
enum Phase {
    Open(BrowserUpdateSeq),
    Closed,
}
/// Owned by exactly one actual connection/subscription. Never retarget or persist it.
#[derive(Debug)]
pub(super) struct Cursor {
    node: NodeId,
    subscription: SubscriptionId,
    browser: BrowserId,
    workspace: WorkspaceId,
    profile: BrowserProfile,
    revision: BrowserRevision,
    viewport: BrowserViewport,
    url: BrowserUrl,
    phase: Phase,
}
impl Cursor {
    pub(super) fn new(node: NodeId, admitted: &BrowserWatched) -> Self {
        Self {
            node,
            subscription: admitted.subscription,
            browser: admitted.tab.browser_id.clone(),
            workspace: admitted.tab.workspace,
            profile: admitted.tab.profile,
            revision: admitted.revision,
            viewport: admitted.tab.viewport,
            url: admitted.tab.url.clone(),
            phase: Phase::Open(BrowserUpdateSeq::FIRST),
        }
    }
    /// A connection loss/owner cancellation is terminal. A new socket gets a new cursor.
    pub(super) fn close(&mut self) {
        self.phase = Phase::Closed;
    }
    /// Validate before handing content to a decoder/owner. An invalid update poisons
    /// this watch rather than quietly skipping a gap or accepting a later valid frame.
    pub(super) fn accept(
        &mut self,
        node: NodeId,
        subscription: SubscriptionId,
        update: &BrowserUpdate,
    ) -> Result<(), WatchError> {
        let result = self.validate(node, subscription, update);
        match result {
            Err(error) => {
                self.close();
                Err(error)
            }
            Ok(next) => {
                if let BrowserChange::State { revision, tab } = &update.change {
                    self.revision = *revision;
                    self.viewport = tab.viewport;
                    self.url = tab.url.clone();
                }
                self.phase = next.map_or(Phase::Closed, Phase::Open);
                Ok(())
            }
        }
    }
    fn validate(
        &self,
        node: NodeId,
        subscription: SubscriptionId,
        update: &BrowserUpdate,
    ) -> Result<Option<BrowserUpdateSeq>, WatchError> {
        let Phase::Open(expected) = self.phase else {
            return Err(WatchError::Closed);
        };
        if node != self.node || subscription != self.subscription {
            return Err(WatchError::Scope);
        }
        if update.sequence != expected {
            return Err(WatchError::Sequence);
        }
        match &update.change {
            BrowserChange::Ended { .. } => return Ok(None),
            BrowserChange::State { revision, tab } => {
                if tab.browser_id != self.browser
                    || tab.workspace != self.workspace
                    || tab.profile != self.profile
                {
                    return Err(WatchError::Scope);
                }
                if *revision < self.revision
                    || (*revision == self.revision
                        && (tab.viewport != self.viewport || tab.url != self.url))
                {
                    return Err(WatchError::Revision);
                }
            }
            BrowserChange::Frame { frame } => {
                if frame.browser_id != self.browser || frame.workspace != self.workspace {
                    return Err(WatchError::Scope);
                }
                if frame.revision != self.revision {
                    return Err(WatchError::Revision);
                }
                let announced = match self.viewport {
                    BrowserViewport::Responsive(size) | BrowserViewport::Fixed(size) => size,
                };
                if frame.viewport != announced {
                    return Err(WatchError::Viewport);
                }
            }
        }
        update
            .sequence
            .next()
            .map(Some)
            .ok_or(WatchError::Exhausted)
    }
}
