//! A dropped pause receipt releases the actor, including a lost requester.
use crate::{
    SessionError, SessionHandle,
    moving::{Control, MoveError},
    parking::{ParkOrder, Route},
    structured::StructuredActor,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// Exclusive actor pause for a node-owned files-only restore.
/// Dropping it restores the previous input route and lets the queue continue.
/// This conveys no file authority; the node must retain its physical root lease.
#[derive(Debug)]
#[must_use = "dropping the pause resumes actor input and queue delivery"]
pub struct CheckpointPause {
    released: CancellationToken,
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
}
#[derive(Debug, Default)]
pub(crate) struct State {
    paused: Option<Paused>,
}
impl State {
    pub(crate) const fn active(&self) -> bool {
        self.paused.is_some()
    }
    pub(crate) async fn released(&self) {
        match &self.paused {
            Some(paused) => paused.released.cancelled().await,
            None => std::future::pending().await,
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
        self.file_pause.paused = Some(Paused {
            released: released.clone(),
            prior,
        });
        // A dropped receiver drops its guard too, so a cancelled admission cannot
        // strand the queue or require a separate release RPC.
        let _gone = reply.send(Ok(CheckpointPause {
            released,
            route: self.route.subscribe(),
        }));
    }
    pub(crate) async fn release_files(&mut self) {
        if let Some(paused) = self.file_pause.paused.take() {
            let _previous = self.route.send_replace(paused.prior);
        }
        let _advanced = self.advance().await;
    }
    pub(crate) fn refuse_file_paused(&mut self, control: Control) {
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
                let _gone = reply.send(self.launch.setup.private_dir.clone());
            }
            #[cfg(test)]
            Control::RefuseNextStop { source, ready } => {
                self.stop_signal_error = Some(source);
                let _gone = ready.send(());
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
        let state = State {
            paused: Some(Paused {
                released: released.clone(),
                prior: Route::Parked,
            }),
        };
        let guard = CheckpointPause {
            released,
            route: watching,
        };
        let (reply, received) = oneshot::channel::<Result<CheckpointPause, SessionError>>();
        drop(received);
        drop(reply.send(Ok(guard)));
        tokio::time::timeout(std::time::Duration::from_secs(1), state.released())
            .await
            .unwrap();
        assert!(
            matches!(*route.borrow(), Route::FilesPaused),
            "only the actor may restore its route"
        );
    }
}
