//! `session.prompt { notify }` (protocol 11, ADR 0033: the starter told):
//! who may ask, and the turn followed until its starter has been told.
//! The bounds and the words are `automation::notices`'.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, oneshot};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proto::conversation::PromptSession;
use vessel_proto::{
    ContentBlock, Creator, Delivered, Delivery, DisplayStatus, ErrorCode, EventSeq, NodeId,
    SessionId, SessionMode, WireError,
};

use super::{Connection, NodeService, wire};
use crate::automation::notices::{Following, Notice, Notices, Turn};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::sessions::Sessions;

/// Records read at once while following a turn.
const PAGE: u32 = 200;
/// One read's wait for a record.
const WAIT: Duration = Duration::from_secs(60);

impl NodeService {
    /// Deliver a prompt and tell the caller's own session about the turn
    /// it starts: only a session's agent asks, through its credential.
    pub(super) async fn prompt_told(
        &self,
        connection: &Connection,
        request: PromptSession,
    ) -> Result<Delivered, WireError> {
        let principal = connection.principal();
        let (node, told) = match principal {
            Principal::Agent { node, session, .. } => (*node, *session),
            Principal::Owner | Principal::Device { .. } => {
                return Err(WireError::new(
                    ErrorCode::InvalidRequest,
                    "`notify` tells an agent through its own session's queue; a person is told \
                     by the page",
                ));
            }
            Principal::Delegated { .. } => {
                return Err(WireError::new(
                    ErrorCode::Forbidden,
                    "`notify` is asked on the agent's own node: a notice cannot reach a session \
                     of another node",
                ));
            }
        };
        let own = self
            .sessions
            .get(told)
            .await
            .map_err(|error| wire(&error))?;
        if matches!(own.spec.mode, SessionMode::Terminal { .. }) {
            return Err(WireError::new(
                ErrorCode::Unsupported,
                "a terminal session has no queue to be told through",
            ));
        }
        let following = self.notices.follow(told).map_err(|error| error.to_wire())?;
        // Heard before the prompt goes, so the told session's end is never
        // missed: its follower then stops and gives back its place.
        let told_ends = self.sessions.ends();
        let (delivered, turn) = self
            .sessions
            .prompt_tracked(
                request.session,
                request.content,
                request.delivery,
                creator(principal),
            )
            .await
            .map_err(|error| wire(&error))?;
        let teller = Teller {
            sessions: Arc::clone(&self.sessions),
            notices: Arc::clone(&self.notices),
            _following: following,
            node,
            told,
            prompted: request.session,
        };
        let stop = self.lifecycle.stopping();
        drop(tokio::spawn(teller.run(turn, told_ends, stop)));
        Ok(delivered)
    }
}

/// Follows one turn and tells its starter.
struct Teller {
    sessions: Arc<Sessions>,
    notices: Arc<Notices>,
    /// Its place among the turns followed, given back when it ends.
    _following: Following,
    node: NodeId,
    told: SessionId,
    prompted: SessionId,
}

impl Teller {
    async fn run(
        self,
        turn: oneshot::Receiver<EventSeq>,
        mut told_ends: broadcast::Receiver<SessionId>,
        stop: CancellationToken,
    ) {
        tokio::select! {
            biased;
            () = stop.cancelled() => {}
            () = self.told_ended(&mut told_ends) => self.notices.forget(self.told),
            () = self.follow(turn) => {}
        }
    }

    /// Once the told session has ended: nobody is left to tell, so the
    /// turn is not followed further.
    async fn told_ended(&self, ends: &mut broadcast::Receiver<SessionId>) {
        loop {
            match ends.recv().await {
                Ok(ended) if ended == self.told => return,
                Ok(_other) => {}
                // Ends were missed: look at the session itself.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let live = self
                        .sessions
                        .get(self.told)
                        .await
                        .is_ok_and(|info| info.status.is_live());
                    if !live {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => std::future::pending::<()>().await,
            }
        }
    }

    async fn follow(&self, turn: oneshot::Receiver<EventSeq>) {
        let started = turn.await;
        let title = match self.sessions.get(self.prompted).await {
            Ok(info) => info.title.map(|title| title.as_str().to_owned()),
            Err(_gone) => return,
        };
        let Ok(from) = started else {
            // The prompt started no turn: removed from the queue (nothing
            // to tell), or the session ended with it waiting.
            if let Ok(info) = self.sessions.get(self.prompted).await
                && !info.status.is_live()
            {
                let reading = Turn::new(self.prompted, title, EventSeq::ZERO);
                let _told = self.send(reading.gone(&info.status)).await;
            }
            return;
        };
        let mut reading = Turn::new(self.prompted, title, from);
        let mut at = from;
        loop {
            let page = match self
                .sessions
                .read_waiting(self.prompted, at, PAGE, WAIT)
                .await
            {
                Ok(page) => page,
                Err(error) => {
                    tracing::debug!(session = %self.prompted, %error, "a told turn cannot be read");
                    return;
                }
            };
            for record in &page.records {
                if let Some(notice) = reading.take(&record.event) {
                    let last = notice.last;
                    if !self.send(notice).await || last {
                        return;
                    }
                }
            }
            if page.records.is_empty() {
                // Nothing more will come from a session that ended.
                match self.sessions.get(self.prompted).await {
                    Ok(info) if info.status.is_live() => {}
                    Ok(info) => {
                        let _told = self.send(reading.gone(&info.status)).await;
                        return;
                    }
                    Err(_gone) => return,
                }
            }
            at = page.next;
        }
    }

    /// Queue `notice` into the told session; whether to go on (the told
    /// session is still there to be told).
    async fn send(&self, notice: Notice) -> bool {
        let info = match self.sessions.get(self.told).await {
            Ok(info) if info.status.is_live() => info,
            Ok(_) | Err(_) => {
                self.notices.forget(self.told);
                return false;
            }
        };
        // Nothing automatic starts a turn in a session a person stopped
        // (ADR 0024's rule, as for a schedule's prompt).
        if info
            .board
            .as_ref()
            .is_some_and(|board| board.display_status == DisplayStatus::Interrupted)
        {
            tracing::info!(told = %self.told, prompted = %self.prompted,
                "a notice was not sent into a session a person stopped");
            return true;
        }
        if !self.notices.may_send(self.told, Instant::now()) {
            tracing::warn!(told = %self.told, prompted = %self.prompted,
                "a notice was dropped: the session had its hour's notices");
            return true;
        }
        let content = vec![ContentBlock::Text { text: notice.text }];
        let by = Creator::Agent {
            node: self.node,
            session: self.prompted,
        };
        if let Err(error) = self
            .sessions
            .prompt(self.told, content, Delivery::Queue, by)
            .await
        {
            tracing::warn!(told = %self.told, %error, "a notice could not be queued; dropped");
        }
        true
    }
}
