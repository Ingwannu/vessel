//! A dropped pause receipt releases the actor, including a lost requester.
use super::metadata::Order;
use crate::{
    SessionError, SessionHandle,
    moving::{Control, MoveError},
    parking::{ParkOrder, Route},
    structured::StructuredActor,
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::{EventSeq, checkpoint::CheckpointEvent};

/// Exclusive actor pause for a node-owned files-only restore.
/// Dropping it restores the previous input route and lets the queue continue.
/// This conveys no file authority; the node must retain its physical root lease.
#[derive(Debug)]
#[must_use = "dropping the pause resumes actor input and queue delivery"]
pub struct CheckpointPause {
    released: CancellationToken,
    pub(super) metadata: mpsc::Sender<Order>,
    route: tokio::sync::watch::Receiver<Route>,
}
impl CheckpointPause {
    /// Release input/queue and wait until the actor has restored its prior route.
    /// The node releases physical exclusion before calling this after a restore.
    ///
    /// # Errors
    /// The actor ended before acknowledging release.
    pub async fn resume(mut self) -> Result<(), SessionError> {
        self.released.cancel();
        self.route
            .wait_for(|route| !matches!(route, Route::FilesPaused))
            .await
            .map(|_restored| ())
            .map_err(|_ended| SessionError::Ended)
    }
}
impl Drop for CheckpointPause {
    fn drop(&mut self) {
        self.released.cancel();
    }
}
#[derive(Debug)]
struct Paused {
    released: CancellationToken,
    prior: Route,
    metadata: mpsc::Receiver<Order>,
    published: Option<(CheckpointEvent, EventSeq)>,
}

const METADATA_QUEUE: usize = 1;

pub(super) enum Publication<'a> {
    Released,
    Pending,
    Published {
        event: &'a CheckpointEvent,
        seq: EventSeq,
    },
}

pub(crate) enum Signal {
    Released,
    Metadata(Order),
}
#[derive(Debug, Default)]
pub(crate) struct State {
    paused: Option<Paused>,
}
impl State {
    pub(crate) const fn active(&self) -> bool {
        self.paused.is_some()
    }
    pub(crate) async fn next(&mut self) -> Signal {
        let Some(paused) = &mut self.paused else {
            return std::future::pending().await;
        };
        tokio::select! {
            biased;
            () = paused.released.cancelled() => Signal::Released,
            order = paused.metadata.recv() => order.map_or(Signal::Released, Signal::Metadata),
        }
    }
    pub(super) fn publication(&self) -> Publication<'_> {
        match &self.paused {
            None => Publication::Released,
            Some(paused) => match &paused.published {
                None => Publication::Pending,
                Some((event, seq)) => Publication::Published { event, seq: *seq },
            },
        }
    }
    pub(super) fn record(&mut self, event: CheckpointEvent, seq: EventSeq) {
        if let Some(paused) = &mut self.paused {
            paused.published = Some((event, seq));
        }
    }
}
impl SessionHandle {
    /// Pause an idle structured actor's input and queue for files-only restore.
    ///
    /// # Errors
    /// Terminal or ended session, an active turn/wait/start, or another pause.
    /// The node must bound this request and acquire physical exclusion first.
    pub async fn pause_for_checkpoint(&self) -> Result<CheckpointPause, SessionError> {
        let route = self.structured("file checkpoint restoration")?;
        self.order(route, Control::PauseFiles).await
    }
}
impl StructuredActor {
    pub(crate) fn pause_files(
        &mut self,
        reply: oneshot::Sender<Result<CheckpointPause, SessionError>>,
    ) {
        if !self.ready
            || self.move_state.active
            || self.desk.running()
            || self.life.out.waiting_for_answers()
            || self.file_pause.active()
        {
            let _gone = reply.send(Err(SessionError::CheckpointBusy));
            return;
        }
        let released = CancellationToken::new();
        let prior = self.route.send_replace(Route::FilesPaused);
        // One in-flight result per owned pause; callers wait instead of spawning
        // per-result jobs or using the general session control channel.
        let (metadata, commands) = mpsc::channel(METADATA_QUEUE);
        self.file_pause.paused = Some(Paused {
            released: released.clone(),
            prior,
            metadata: commands,
            published: None,
        });
        // A dropped receiver drops its guard too, so a cancelled admission cannot
        // strand the queue or require a separate release RPC.
        let _gone = reply.send(Ok(CheckpointPause {
            released,
            metadata,
            route: self.route.subscribe(),
        }));
    }
    pub(crate) async fn release_files(&mut self) {
        if let Some(paused) = self.file_pause.paused.take() {
            let _previous = self.route.send_replace(paused.prior);
        }
        let _advanced = self.advance().await;
    }
    pub(crate) fn refuse_file_paused(control: Control) {
        match control {
            Control::Move(order) => {
                let _gone = order.reply.send(Err(MoveError::FilesPaused));
            }
            Control::Deliver(order) => {
                let _gone = order.reply.send(Err(SessionError::FilesPaused));
            }
            Control::SideQuestion(order) => {
                let _gone = order.reply.send(Err(SessionError::FilesPaused));
            }
            Control::Queue(order) => {
                let _gone = order.reply.send(Err(SessionError::FilesPaused));
            }
            Control::Cancel(order) => {
                let _gone = order.reply.send(Err(SessionError::FilesPaused));
            }
            Control::Park(ParkOrder::Park(reply) | ParkOrder::Wake(reply)) => {
                let _gone = reply.send(Err(SessionError::FilesPaused));
            }
            Control::Rewind(order) => {
                let _gone = order.reply.send(Err(SessionError::FilesPaused));
            }
            Control::PauseFiles(reply) => {
                let _gone = reply.send(Err(SessionError::FilesPaused));
            }
            #[cfg(test)]
            Control::PrivateDir(reply) => {
                drop(reply);
            }
            #[cfg(test)]
            Control::RefuseNextStop { source: _, ready } => {
                drop(ready);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn checkpoint_lost_pause_receipt_releases_its_owner_without_a_second_call() {
        let released = CancellationToken::new();
        let (route, watching) = tokio::sync::watch::channel(Route::FilesPaused);
        let (metadata, commands) = mpsc::channel(METADATA_QUEUE);
        let mut state = State {
            paused: Some(Paused {
                released: released.clone(),
                prior: Route::Parked,
                metadata: commands,
                published: None,
            }),
        };
        let guard = CheckpointPause {
            released,
            metadata,
            route: watching,
        };
        let (reply, received) = oneshot::channel::<Result<CheckpointPause, SessionError>>();
        drop(received);
        drop(reply.send(Ok(guard)));
        tokio::time::timeout(std::time::Duration::from_secs(1), state.next())
            .await
            .unwrap();
        assert!(
            matches!(*route.borrow(), Route::FilesPaused),
            "only the actor may restore its route"
        );
    }
}
