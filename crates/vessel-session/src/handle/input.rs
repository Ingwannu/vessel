//! What a handle sends a session: input of either mode, and a structured
//! session's prompts, cancels and queue changes (protocol 10), which take
//! the actor's turn in order with its queue; answers go straight to the
//! agent.

use tokio::sync::{oneshot, watch};
use vessel_proto::conversation::QueueChanged;
use vessel_proto::{
    ContentBlock, Creator, Delivered, Delivery, EventSeq, QueueChange, SessionInput, TerminalInput,
};
use vessel_pty::WindowSize;

use super::{Input, SessionHandle, current};
use crate::delivery::{Cancel, Change, Deliver};
use crate::error::{DeliveryError, InputKind, ParkRefusal, SessionError};
use crate::moving::Control;
use crate::parking::Route;

impl SessionHandle {
    /// Send input to the session.
    ///
    /// Terminal input waits while the program's input queue is full.
    /// Structured input waits until the agent has applied it (and, while the
    /// ACP handshake runs, until it is ready).
    ///
    /// # Errors
    /// [`SessionError::Unsupported`] when the input does not match the
    /// session's mode, [`SessionError::Ended`] once the session has ended,
    /// [`SessionError::Input`] when the agent refuses it (for example a
    /// second answer to a permission request).
    /// [`SessionError::TerminalScreen`] when the bounded resize queue cannot admit it.
    pub async fn send(&self, input: SessionInput) -> Result<(), SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        match (&self.0.input, input) {
            (Input::Terminal { pty, screen }, SessionInput::Terminal { input }) => {
                send_terminal(pty, screen, input).await
            }
            // Protocol 10: prompts and cancels take the actor's turn, in
            // order with the queue (`delivery`).
            (Input::Structured(route), SessionInput::Prompt { content }) => {
                let _started = self
                    .order(route, |reply| {
                        Control::Deliver(Deliver {
                            content,
                            delivery: Delivery::Start,
                            by: Creator::Owner,
                            turn: None,
                            reply,
                        })
                    })
                    .await?;
                Ok(())
            }
            (Input::Structured(route), SessionInput::CancelTurn { turn }) => {
                self.order(route, |reply| Control::Cancel(Cancel { turn, reply }))
                    .await
            }
            (Input::Structured(route), SessionInput::AnswerPermission { id, outcome }) => {
                Ok(current(route)?.answer(id, outcome).await?)
            }
            // Protocol 10: straight to the agent, as a permission answer:
            // it settles a request of the running turn, it starts nothing.
            (Input::Structured(route), SessionInput::AnswerQuestion { id, answer }) => {
                Ok(current(route)?.answer_question(id, answer).await?)
            }
            (
                input_path @ Input::Terminal { .. },
                input @ (SessionInput::Prompt { .. }
                | SessionInput::CancelTurn { .. }
                | SessionInput::AnswerPermission { .. }
                | SessionInput::AnswerQuestion { .. }),
            )
            | (input_path @ Input::Structured(_), input @ SessionInput::Terminal { .. }) => {
                Err(SessionError::Unsupported {
                    input: InputKind::of(&input),
                    mode: input_path.mode(),
                })
            }
        }
    }

    /// Deliver a prompt as asked (protocol 10's `session.prompt`); `by`
    /// is who sent it, kept with it if it waits in the queue.
    ///
    /// # Errors
    /// [`SessionError::Delivery`] for an empty prompt or an interrupt whose
    /// turn did not end, [`SessionError::Queue`] when the queue is full,
    /// [`SessionError::Input`] when the agent refuses it (a turn runs for
    /// `start`, it cannot steer), [`SessionError::Terminal`],
    /// [`SessionError::Moving`], [`SessionError::Ended`].
    pub async fn prompt(
        &self,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<Delivered, SessionError> {
        if content.is_empty() {
            return Err(DeliveryError::Empty.into());
        }
        let route = self.structured("a queue")?;
        self.order(route, |reply| {
            Control::Deliver(Deliver {
                content,
                delivery,
                by,
                turn: None,
                reply,
            })
        })
        .await
    }

    /// As [`prompt`](Self::prompt), and which turn the prompt started or
    /// joined (protocol 11's `notify`): the receiver is given the seq of
    /// that turn's `turn_started` record once it is published (for a
    /// queued prompt, when its entry starts a turn), and closes without
    /// one if it never starts a turn (removed from the queue, or the
    /// session ended first).
    ///
    /// # Errors
    /// As [`prompt`](Self::prompt).
    pub async fn prompt_tracked(
        &self,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<(Delivered, oneshot::Receiver<EventSeq>), SessionError> {
        if content.is_empty() {
            return Err(DeliveryError::Empty.into());
        }
        let route = self.structured("a queue")?;
        let (ticket, turn) = oneshot::channel();
        let delivered = self
            .order(route, |reply| {
                Control::Deliver(Deliver {
                    content,
                    delivery,
                    by,
                    turn: Some(ticket),
                    reply,
                })
            })
            .await?;
        Ok((delivered, turn))
    }

    /// Change the queue (protocol 10's `session.queue`); the queue after,
    /// and how an entry sent now was delivered.
    ///
    /// # Errors
    /// [`SessionError::Delivery`] for an edit with no content;
    /// [`SessionError::Queue`] for an entry no longer queued, an edit from
    /// another revision, a full queue or `send_now` with `queue`; a
    /// `send_now`'s delivery errors, as [`prompt`](Self::prompt);
    /// [`SessionError::Terminal`], [`SessionError::Moving`],
    /// [`SessionError::Ended`].
    pub async fn change_queue(&self, change: QueueChange) -> Result<QueueChanged, SessionError> {
        // An entry is a prompt waiting: it needs content as one does.
        if let QueueChange::Edit { content, .. } = &change
            && content.is_empty()
        {
            return Err(DeliveryError::Empty.into());
        }
        let route = self.structured("a queue")?;
        self.order(route, |reply| Control::Queue(Change { change, reply }))
            .await
    }

    /// The input route of a live structured session; a terminal session
    /// is refused as parking refuses it (protocol 12).
    pub(super) fn structured_route(
        &self,
        terminal: ParkRefusal,
    ) -> Result<&watch::Receiver<Route>, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        match &self.0.input {
            Input::Terminal { .. } => Err(terminal.into()),
            Input::Structured(route) => Ok(route),
        }
    }

    /// The input route of a live structured session.
    pub(crate) fn structured(
        &self,
        what: &'static str,
    ) -> Result<&watch::Receiver<Route>, SessionError> {
        if !self.0.info.borrow().status.is_live() {
            return Err(SessionError::Ended);
        }
        match &self.0.input {
            Input::Terminal { .. } => Err(SessionError::Terminal(what)),
            Input::Structured(route) => Ok(route),
        }
    }

    /// Hand an order to the actor and wait for its answer. Refused while
    /// the session moves, as input always was; an order sent as a move
    /// begins waits for it.
    pub(crate) async fn order<T>(
        &self,
        route: &watch::Receiver<Route>,
        order: impl FnOnce(oneshot::Sender<Result<T, SessionError>>) -> Control,
    ) -> Result<T, SessionError> {
        if matches!(*route.borrow(), Route::Moving) {
            return Err(SessionError::Moving);
        }
        let control = self.0.control.as_ref().ok_or(SessionError::Ended)?;
        let (reply, answer) = oneshot::channel();
        control
            .send(order(reply))
            .await
            .map_err(|_ended| SessionError::Ended)?;
        answer.await.map_err(|_ended| SessionError::Ended)?
    }

    /// Whether the session's agent is in the middle of a turn (a prompt
    /// already on its way counts); `false` for a terminal session, whose
    /// turns Vessel cannot see. For copying a session's files away
    /// (protocol 6's `session.export`): between turns its transcript is
    /// complete, during one the harness is still appending to it.
    ///
    /// # Errors
    /// [`SessionError::Moving`] while the session moves to another profile,
    /// [`SessionError::Ended`] once its agent's connection has closed.
    pub async fn turn_running(&self) -> Result<bool, SessionError> {
        match &self.0.input {
            Input::Terminal { .. } => Ok(false),
            // A parked agent runs no turn (protocol 12).
            Input::Structured(route) if matches!(*route.borrow(), Route::Parked) => Ok(false),
            Input::Structured(route) => Ok(current(route)?.turn_running().await?),
        }
    }
}

/// Keep PTY/screen admission separate from the structured actor's delivery graph.
async fn send_terminal(
    pty: &vessel_pty::PtyInput,
    screen: &crate::terminal_screen::CaptureEndpoint,
    input: TerminalInput,
) -> Result<(), SessionError> {
    let sent = match input {
        TerminalInput::Data { data } => pty.write(data.into_bytes()).await,
        TerminalInput::Resize { size } => {
            screen.resize(size)?;
            let resized = pty
                .resize(WindowSize {
                    cols: size.cols,
                    rows: size.rows,
                })
                .await;
            if resized.is_err() {
                screen.invalidate();
            }
            resized
        }
    };
    sent.map_err(|vessel_pty::PtyClosed| SessionError::Ended)
}
