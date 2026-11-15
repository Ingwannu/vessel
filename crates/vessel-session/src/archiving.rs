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
    pub(crate) condition: ArchiveCondition,
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
        if !self.info().status.is_live() {
            return Ok(self.info());
        }
        match &self.0.input {
            Input::Terminal(_) => match condition {
                ArchiveCondition::IfIdle => return Err(ArchiveRefusal::Terminal.into()),
                ArchiveCondition::Force => self.kill(),
            },
            Input::Structured(route) => {
                let admitted = self
                    .order(route, |reply| {
                        Control::Archive(ArchiveOrder { condition, reply })
                    })
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
        let refusal = match order.condition {
            ArchiveCondition::Force => None,
            ArchiveCondition::IfIdle if !self.ready || self.move_state.active => {
                Some(ArchiveRefusal::NotReady)
            }
            ArchiveCondition::IfIdle if self.desk.running() => Some(ArchiveRefusal::TurnRunning),
            ArchiveCondition::IfIdle => None,
        };
        if let Some(refusal) = refusal {
            let _gone = order.reply.send(Err(refusal.into()));
            return ArchiveAdmission::Refused;
        }
        // The stop token also fences after_turn's automatic queue advance. The
        // route blocks direct input and closing intake refuses future prompts.
        let _live = self.route.send_replace(Route::Moving);
        self.control.close();
        self.life.stop.request();
        let _gone = order.reply.send(Ok(()));
        ArchiveAdmission::Stopping
    }
}
