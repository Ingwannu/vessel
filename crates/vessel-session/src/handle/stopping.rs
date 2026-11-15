//! A person's durable stop and its effect share input admission, not actor state.
//!
//! [Decision Log]
//! - Purpose: a newer prompt must not rearm in the gap between a stop's
//!   durable ACK and its actual kill/cancel enqueue.
//! - Constraint: neither a slow driver nor a full input channel may own the
//!   stop lock. Reserve channel room first; release ordinary admission at
//!   enqueue, before awaiting the driver's answer.
//! - Choice: one per-session input gate, shared by typed stop and input.
//!   Compared with a second suppression write this gives one linear order
//!   without another disk barrier; it costs one short lock per enqueue.
use super::SessionHandle;
use crate::SessionError;
use crate::delivery::Cancel;
use crate::moving::Control;
use crate::parking::Route;
use tokio::sync::oneshot;
use vessel_proto::EventSeq;

/// A person's stop; internal cleanup continues to use the raw kill signal.
#[derive(Debug, Clone, Copy)]
pub enum SessionStop {
    /// End the process after suppression commits.
    Kill,
    /// Cancel only the named turn, or whichever turn is current when absent.
    Cancel {
        /// Only this turn; absence cancels whichever turn is current.
        turn: Option<EventSeq>,
    },
}
impl SessionHandle {
    /// Persist suppression and admit its effect without a newer input gap.
    ///
    /// Capacity is reserved before the input lock. A full actor lane cannot
    /// hold that lock and prevent a kill. The lock covers a stop's journal
    /// ACK and effect enqueue, but ordinary input releases it before waiting
    /// for the driver, so a stuck provider does not hold up a person's kill.
    ///
    /// # Errors
    /// Persistence failed, the turn counter exhausted, or cancel input ended.
    pub async fn stop(&self, action: SessionStop) -> Result<(), SessionError> {
        match action {
            SessionStop::Kill => {
                let _input = self.0.input_admission.lock().await;
                let _matched = self.0.journal.suppress(None).await?;
                self.kill();
                Ok(())
            }
            SessionStop::Cancel { turn } => self.cancel_committed(turn).await,
        }
    }

    async fn cancel_committed(&self, turn: Option<EventSeq>) -> Result<(), SessionError> {
        let route = self.structured("a conversation")?;
        if matches!(*route.borrow(), Route::Moving) {
            return Err(SessionError::Moving);
        }
        let control = self.0.control.as_ref().ok_or(SessionError::Ended)?;
        let permit = control
            .reserve()
            .await
            .map_err(|_ended| SessionError::Ended)?;
        let answer = {
            let _input = self.0.input_admission.lock().await;
            self.accepting_input()?;
            let _matched = self.0.journal.suppress(turn).await?;
            let (reply, answer) = oneshot::channel();
            permit.send(Control::Cancel(Cancel { turn, reply }));
            answer
        };
        answer.await.map_err(|_ended| SessionError::Ended)?
    }
}
