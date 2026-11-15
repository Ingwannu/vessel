//! Lifecycle actions reuse the existing session RPC dispatcher with correlation.
use crate::{ActFailed, Acted};
use vessel_app_view::sessions::LifecycleTicket;
/// One authoritative result; the app must not remove a tab on a refusal.
#[derive(Debug)]
pub struct Answer {
    /// The originating lifecycle request.
    pub ticket: LifecycleTicket,
    /// Node result or bounded failure. Existing typed RPC errors are retained.
    pub result: Result<Acted, Failure>,
}
/// A timeout can leave a mutation committed; it is never retried automatically.
#[derive(Debug)]
pub enum Failure {
    /// No connected host or the node refused its existing operation.
    Refused(ActFailed),
    /// Refresh before retrying because a mutation's outcome is uncertain.
    Uncertain,
    /// The local queue accepted no mutation.
    Unsent,
    /// A saved alias now authenticates a different node; no RPC was issued.
    IdentityChanged,
}
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{Answer, Failure};
    use crate::{Act, Event, State};
    use std::time::Duration;
    use vessel_app_view::sessions::{LifecycleAction, PendingLifecycle};
    use vessel_proto::Mark;
    /// Archiving can wait for graceful process shutdown; keep a finite outer bound.
    const ACTION_DEADLINE: Duration = Duration::from_secs(30);
    impl State {
        pub(crate) fn session_lifecycle(&self, request: &PendingLifecycle) {
            let client = self
                .clients
                .get(request.target.host.as_str())
                .map(|connected| connected.client.clone());
            let events = self.events.clone();
            let ticket = request.ticket;
            let session = request.target.session;
            let action = request.action;
            let expected_node = request.target.node;
            drop(tokio::spawn(async move {
                let result = match client {
                    None => Err(Failure::Unsent),
                    Some(client) if expected_node.is_some_and(|node| client.node().id != node) => {
                        Err(Failure::IdentityChanged)
                    }
                    Some(client) => {
                        let act = match action {
                            LifecycleAction::Archive
                            | LifecycleAction::Exit
                            | LifecycleAction::Fresh => Act::Mark(Mark::Archive),
                            LifecycleAction::Unarchive => Act::Mark(Mark::Unarchive),
                            LifecycleAction::Resume => Act::StartAgain,
                            LifecycleAction::Delete => Act::Delete,
                        };
                        match tokio::time::timeout(
                            ACTION_DEADLINE,
                            crate::session::run(&client, session, act),
                        )
                        .await
                        {
                            Ok(result) => result.map_err(Failure::Refused),
                            Err(_) => Err(Failure::Uncertain),
                        }
                    }
                };
                let _closed = events
                    .send(Event::SessionLifecycle(Answer { ticket, result }))
                    .await;
            }));
        }
    }
}
