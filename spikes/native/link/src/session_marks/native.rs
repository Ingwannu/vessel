//! Effects capture the sole original selection once; no alias lookup occurs in tasks.
use super::{Answer, Ask, Failure, Reply, delivery};
use crate::{
    Event, State,
    connections::{RequestFailure, SelectedConnection},
};
use std::{sync::Arc, time::Duration};
use vessel_proto::{Mark, Method, SessionInfo, conversation::MarkSession};

const MAX_CALLS: usize = 8;
const ACTION_DEADLINE: Duration = Duration::from_secs(30);
pub(super) const PUBLICATION_DEADLINE: Duration = Duration::from_secs(5);
/// Bounded work only, not an identity registry or request/ticket map.
#[derive(Debug)]
pub(crate) struct Hub(pub(super) Arc<tokio::sync::Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(tokio::sync::Semaphore::new(MAX_CALLS)))
    }
}
impl State {
    pub(crate) async fn session_mark(&self, ask: Ask) {
        let selected = match self.selected_connection(ask.target().context()) {
            Ok(selected) => selected,
            Err(_) => {
                self.emit(Event::SessionMarked(Answer::refused(ask, Failure::Changed)))
                    .await;
                return;
            }
        };
        let guard = selected.delivery_guard();
        let failure = if guard.validate(ask.target().context()).is_err() {
            Some(Failure::Changed)
        } else if Method::MarkSession
            .since()
            .is_none_or(|since| since > selected.protocol())
        {
            Some(Failure::Unsupported)
        } else {
            None
        };
        if let Some(failure) = failure {
            self.emit(Event::SessionMarked(Answer::refused(ask, failure)))
                .await;
            return;
        }
        let Ok(permit) = Arc::clone(&self.session_mark_calls.0).try_acquire_owned() else {
            self.emit(Event::SessionMarked(Answer::refused(ask, Failure::Unsent)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let result = run(&selected, &ask).await;
            delivery::publish(&selected, &events, Reply { ask, result }, guard).await;
            drop(permit);
        }));
    }
}
async fn run(selected: &SelectedConnection, ask: &Ask) -> Result<Box<SessionInfo>, Failure> {
    let request = MarkSession {
        session: ask.target().session(),
        mark: ask.mark(),
    };
    let result = tokio::time::timeout(ACTION_DEADLINE, selected.call(request)).await;
    match result {
        Ok(Ok(info)) if info.id == request.session && agrees(request.mark, &info) => {
            Ok(Box::new(info))
        }
        Ok(Ok(_)) => Err(Failure::InvalidReply),
        Ok(Err(RequestFailure::Call(vessel_client::CallError::Remote(error)))) => {
            Err(Failure::Refused(error.code))
        }
        Ok(Err(RequestFailure::Refused(_) | RequestFailure::Call(_))) | Err(_) => {
            Err(Failure::Uncertain)
        }
    }
}
fn agrees(mark: Mark, info: &SessionInfo) -> bool {
    match mark {
        Mark::Pin => info.marks.pinned,
        Mark::Unpin => !info.marks.pinned,
        Mark::Archive => info.marks.archived_at.is_some() && !info.status.is_live(),
        Mark::Unarchive => info.marks.archived_at.is_none(),
        Mark::Unread => info.marks.unread,
        // New attention after the original boundary may legitimately stay unread.
        Mark::Read { .. } => true,
    }
}
