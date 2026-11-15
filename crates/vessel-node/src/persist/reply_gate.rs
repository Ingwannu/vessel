//! Test-only observation of a real committed Arm ACK, never a synthetic commit.
//! Holding only its reply leaves the writer free to commit a person's suppression.
use super::{JournalError, RestartCheckpoint, SessionId};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;
type Reply = oneshot::Sender<Result<bool, JournalError>>;
static GATES: OnceLock<Mutex<HashMap<SessionId, oneshot::Sender<Held>>>> = OnceLock::new();
fn gates() -> &'static Mutex<HashMap<SessionId, oneshot::Sender<Held>>> {
    GATES.get_or_init(Mutex::default)
}
pub(crate) struct Registration {
    session: SessionId,
    arrived: oneshot::Receiver<Held>,
}
pub(crate) struct Held {
    reply: Option<Reply>,
    committed: bool,
}
impl Drop for Held {
    fn drop(&mut self) {
        if let Some(reply) = self.reply.take() {
            let _gone = reply.send(Ok(self.committed));
        }
    }
}
impl Registration {
    pub(crate) async fn arrived(&mut self) -> Held {
        (&mut self.arrived).await.unwrap()
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        let _removed = gates().lock().unwrap().remove(&self.session);
    }
}
pub(crate) fn arm(session: SessionId) -> Registration {
    let (entered, arrived) = oneshot::channel();
    assert!(gates().lock().unwrap().insert(session, entered).is_none());
    Registration { session, arrived }
}
pub(super) fn hold(
    session: SessionId,
    change: RestartCheckpoint,
    reply: Reply,
    committed: bool,
) -> Result<(), Reply> {
    if matches!(change, RestartCheckpoint::Arm { .. })
        && let Some(entered) = gates().lock().unwrap().remove(&session)
    {
        // Only run after the actual Store call returned this exact result.
        // Drop also releases it when a test panics or its receiver disappears.
        drop(entered.send(Held {
            reply: Some(reply),
            committed,
        }));
        Ok(())
    } else {
        Err(reply)
    }
}
