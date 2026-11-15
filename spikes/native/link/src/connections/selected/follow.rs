//! Staged private stream guard for W12's resident follow wiring (A10 analogy).
//! The consumer removes the parent's literal cfg(test), retaining this one
//! original-client guard. No legacy follow UI or raw-client escape is added.
use super::{RequestFailure, SelectedConnection, Validity};
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_client::{SessionStream, StreamError};
use vessel_proto::{EventRecord, EventSeq, SessionId, SessionInfo};

/// No underlying stream/client is exposed; every next can perform catch-up RPC.
pub(crate) struct SelectedFollow {
    selected: SelectedConnection,
    stream: SessionStream,
}
/// Attach provenance is distinct from actual shared-lease retirement.
#[derive(Debug)]
pub(crate) enum FollowFailure {
    /// Keep the original typed request failure, including local lease refusal.
    Connection(RequestFailure),
    /// The peer described another session; no record source was admitted.
    WrongSession,
}
impl SelectedConnection {
    pub(crate) async fn follow_session(
        &self,
        session: SessionId,
        from: EventSeq,
    ) -> Result<SelectedFollow, FollowFailure> {
        let stream = self
            .guard(self.lease.client.follow(session, from))
            .await
            .map_err(FollowFailure::Connection)?;
        // A valid physical lease cannot bless records from a subscription whose
        // own metadata names another session. Drop it before exposing any pump.
        if stream.info().id != session {
            return Err(FollowFailure::WrongSession);
        }
        Ok(SelectedFollow {
            selected: self.clone(),
            stream,
        })
    }
    pub(crate) async fn retirement(&self) -> Refusal {
        if let Err(why) = self.active() {
            return why;
        }
        let mut validity = self.lease.validity.clone();
        let why = tokio::select! {
            biased;
            _retired = validity.wait_for(|state| *state == Validity::Retired) => Refusal::Changed,
            () = self.lease.client.closed() => Refusal::Closed,
        };
        if why == Refusal::Closed {
            self.report_closed().await;
        }
        why
    }
}
impl SelectedFollow {
    pub(crate) fn context(&self) -> &ConnectionContext {
        &self.selected.lease.context
    }
    pub(crate) fn info(&self) -> Result<&SessionInfo, Refusal> {
        self.selected.active()?;
        Ok(self.stream.info())
    }
    pub(crate) async fn next(
        &mut self,
    ) -> Result<Option<Result<EventRecord, StreamError>>, Refusal> {
        self.selected.active()?;
        let result = tokio::select! {
            biased;
            why = self.selected.retirement() => return Err(why),
            value = self.stream.next() => value,
        };
        self.selected.fence(result).await
    }
}

mod tests;
