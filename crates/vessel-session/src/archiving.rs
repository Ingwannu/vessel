//! Archive admission shares the actor mailbox with every prompt/queue advance.
//! A successful admission closes control intake before it acknowledges the stop.
use crate::handle::Input;
use crate::moving::Control;
use crate::parking::Route;
use crate::structured::StructuredActor;
use crate::{SessionError, SessionHandle};
use tokio::sync::oneshot;
use vessel_proto::{SessionInfo, session_archive::ArchiveCondition};

/// Why archive admission made no change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ArchiveRefusal {
    /// A TUI's turn boundary is not observable to this runtime.
    #[error("a terminal session requires an explicit forced archive")]
    Terminal,
    /// The initial handshake or a launch transition has not settled.
    #[error("the session is not ready for an idle archive")]
    NotReady,
    /// A prompt already admitted through this mailbox is still running.
    #[error("a turn is running; wait for it to end or force the archive")]
    TurnRunning,
    /// The durable mark already exists; this action is not an unarchive.
    #[error("the session is already archived")]
    AlreadyArchived,
}

#[derive(Debug)]
pub(crate) struct ArchiveOrder {
    pub(crate) reply: oneshot::Sender<Result<(), SessionError>>,
}
pub(crate) enum ArchiveAdmission {
    Stopping,
    Refused,
}

impl SessionHandle {
    /// Admit a stop atomically with turn delivery, then wait for the actor's end.
    /// This does not write an archive mark: the node keeps the admission/store
    /// lock and cancellation-safe task until that durable write has completed.
    /// # Errors
    /// `NotArchived` leaves a running actor untouched; ordinary input/actor
    /// failures retain their typed error. Terminal sessions require Force.
    pub async fn stop_for_archive(
        &self,
        condition: ArchiveCondition,
    ) -> Result<SessionInfo, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Ok(self.info());
        }
        if condition == ArchiveCondition::Force {
            // A force request interrupts a move as ordinary kill already does;
            // it cannot wait behind that move's actor mailbox order.
            self.kill();
            return Ok(self.ended().await);
        }
        match &self.0.input {
            Input::Terminal(_) => return Err(ArchiveRefusal::Terminal.into()),
            Input::Structured(route) => {
                let admitted = self
                    .order(route, |reply| Control::Archive(ArchiveOrder { reply }))
                    .await;
                match admitted {
                    Ok(()) => {}
                    Err(SessionError::Ended) if self.has_ended() => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(self.ended().await)
    }
}

impl StructuredActor {
    pub(crate) fn archive_order(&mut self, order: ArchiveOrder) -> ArchiveAdmission {
        let refusal = if !self.ready || self.move_state.active {
            Some(ArchiveRefusal::NotReady)
        } else if self.desk.running() {
            Some(ArchiveRefusal::TurnRunning)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            let _gone = order.reply.send(Err(refusal.into()));
            return ArchiveAdmission::Refused;
        }
        // The stop token also fences after_turn's automatic queue advance. The
        // route blocks direct input and closing intake refuses future prompts.
        let _live = self.route.send_replace(Route::Stopping);
        self.control.close();
        self.life.stop.request();
        let _gone = order.reply.send(Ok(()));
        ArchiveAdmission::Stopping
    }
}
