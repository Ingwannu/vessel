//! The calling node follows a destination-issued exact-turn receipt. No callback
//! can name an arbitrary parent: that parent comes from the direct agent principal.

use super::{
    Arc, Connection, Duration, NodeService, Notice, Principal, SessionMode, Teller, Turn, wire,
};
use crate::fleet::Links;
use crate::prompt_receipts::RECEIPT_LIFETIME;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::conversation::PromptSession;
use vessel_proto::mcp::{
    ClosePromptReceipt, PromptReceiptState, ReadPromptReceipt, TrackPrompt, TrackedPrompt,
};
use vessel_proto::{
    Deliver, ErrorCode, Forwarded, GetSession, NodeId, Origin, ReadEvents, RelayCall, Reply,
    WireError,
};
use vessel_rpc::Unary;

mod supervision;

const PAGE: u32 = 200;
const WAIT_SECS: u32 = 30;
const EARLY_PAUSE: Duration = Duration::from_secs(1);

struct Remote {
    links: Arc<Links>,
    origin: Origin,
    node: NodeId,
}
impl Remote {
    async fn call<R: Unary>(&self, request: R) -> Result<R::Response, WireError> {
        let call = Box::new(request.into_call());
        let origin = self.origin.clone();
        let result = if self.links.is_home() {
            self.links
                .call_member(self.node, Deliver { call, origin })
                .await
        } else if self.links.is_member() {
            self.links
                .call_home(RelayCall {
                    node: self.node,
                    call,
                    origin,
                })
                .await
        } else {
            Err(WireError::new(
                ErrorCode::Unavailable,
                "this node has no fleet route",
            ))
        }?;
        R::response_from(*result.reply).map_err(|_wrong| {
            WireError::new(
                ErrorCode::Internal,
                "remote node returned a different reply",
            )
        })
    }
}

impl NodeService {
    pub(crate) async fn prompt_remote_told(
        &self,
        connection: &Connection,
        node: NodeId,
        request: PromptSession,
    ) -> Result<Forwarded, WireError> {
        let principal = connection.principal();
        let Principal::Agent { session: told, .. } = principal else {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "remote notify requires a direct session agent",
            ));
        };
        principal.permit(vessel_proto::Method::PromptSession)?;
        let own = self
            .sessions
            .get(*told)
            .await
            .map_err(|error| wire(&error))?;
        if matches!(own.spec.mode, SessionMode::Terminal { .. }) {
            return Err(WireError::new(
                ErrorCode::Unsupported,
                "a terminal parent has no notification queue",
            ));
        }
        let following = self
            .notices
            .follow(*told)
            .map_err(|error| error.to_wire())?;
        let remote = Remote {
            links: Arc::clone(&self.links),
            origin: self.origin_of(principal)?,
            node,
        };
        let teller = Teller {
            sessions: Arc::clone(&self.sessions),
            notices: Arc::clone(&self.notices),
            _following: following,
            node,
            told: *told,
            prompted: request.session,
        };
        let ends = self.sessions.ends();
        let stop = self.lifecycle.stopping();
        let tasks = self.forwarders.clone();
        // The setup and accepted follower outlive cancellation of the MCP/RPC.
        self.forwarders
            .spawn(async move {
                let info = remote
                    .call(GetSession {
                        session: request.session,
                    })
                    .await?;
                let tracked = remote
                    .call(TrackPrompt {
                        session: request.session,
                        content: request.content,
                        delivery: request.delivery,
                    })
                    .await?;
                let delivered = tracked.delivered;
                let runner = RemoteTeller {
                    teller,
                    remote,
                    title: info.title.map(|title| title.as_str().to_owned()),
                };
                runner.start(tracked, ends, stop, &tasks);
                Ok(Forwarded {
                    reply: Box::new(Reply::PromptSession(delivered)),
                })
            })
            .await
            .map_err(|_lost| {
                WireError::new(ErrorCode::Internal, "remote notification setup failed")
            })?
    }
}

struct RemoteTeller {
    teller: Teller,
    remote: Remote,
    title: Option<String>,
}
impl RemoteTeller {
    fn start(
        self,
        tracked: TrackedPrompt,
        mut ends: tokio::sync::broadcast::Receiver<vessel_proto::SessionId>,
        stop: CancellationToken,
        tasks: &TaskTracker,
    ) {
        drop(tasks.spawn(async move {
            let teller = &self.teller;
            let cleanup = supervision::run(
                self.follow(&tracked),
                teller.told_ended(&mut ends),
                stop,
                |end| async move {
                    match end {
                        supervision::End::Stopped | supervision::End::Followed(Ok(())) => {}
                        supervision::End::ParentEnded => teller.notices.forget(teller.told),
                        supervision::End::Followed(Err(error)) => {
                            let _sent = teller.send(unavailable(&error)).await;
                        }
                    }
                },
                self.remote.call(ClosePromptReceipt {
                    session: teller.prompted,
                    receipt: tracked.receipt,
                }),
            )
            .await;
            if cleanup != supervision::Cleanup::Confirmed {
                tracing::warn!(?cleanup, "remote turn receipt cleanup was not confirmed");
            }
        }));
    }
    async fn follow(&self, tracked: &TrackedPrompt) -> Result<(), WireError> {
        let started = tokio::time::timeout(RECEIPT_LIFETIME, self.started(tracked))
            .await
            .map_err(|_late| {
                WireError::new(
                    ErrorCode::Unavailable,
                    "remote prompt did not start before receipt expiry",
                )
            })??;
        let Some(turn) = started else {
            return Ok(());
        };
        let mut reading = Turn::new(self.teller.prompted, self.title.clone(), turn);
        let mut from = turn;
        loop {
            let began = tokio::time::Instant::now();
            let page = self
                .remote
                .call(ReadEvents {
                    session: self.teller.prompted,
                    from,
                    limit: PAGE,
                    wait: Some(WAIT_SECS),
                })
                .await?;
            for record in &page.records {
                if let Some(notice) = reading.take(&record.event) {
                    let last = notice.last;
                    if !self.teller.send(notice).await || last {
                        return Ok(());
                    }
                }
            }
            if page.records.is_empty() {
                let info = self
                    .remote
                    .call(GetSession {
                        session: self.teller.prompted,
                    })
                    .await?;
                if !info.status.is_live() {
                    let _sent = self.teller.send(reading.gone(&info.status)).await;
                    return Ok(());
                }
                if began.elapsed() < EARLY_PAUSE {
                    tokio::time::sleep(EARLY_PAUSE).await;
                }
            }
            from = page.next;
        }
    }
    async fn started(
        &self,
        tracked: &TrackedPrompt,
    ) -> Result<Option<vessel_proto::EventSeq>, WireError> {
        loop {
            match self
                .remote
                .call(ReadPromptReceipt {
                    session: self.teller.prompted,
                    receipt: tracked.receipt,
                })
                .await?
            {
                PromptReceiptState::Started { turn } => return Ok(Some(turn)),
                PromptReceiptState::Removed => {
                    let info = self
                        .remote
                        .call(GetSession {
                            session: self.teller.prompted,
                        })
                        .await?;
                    if !info.status.is_live() {
                        let reading = Turn::new(
                            self.teller.prompted,
                            self.title.clone(),
                            vessel_proto::EventSeq::ZERO,
                        );
                        let _sent = self.teller.send(reading.gone(&info.status)).await;
                    }
                    return Ok(None);
                }
                PromptReceiptState::Pending => tokio::time::sleep(EARLY_PAUSE).await,
            }
        }
    }
}

/// Peer messages are input; only the closed code reaches diagnostic output.
fn unavailable(error: &WireError) -> Notice {
    tracing::warn!(code = ?error.code, "remote turn notification could not be followed");
    Notice {
        text: "Vessel: the remote worker's notification stream became unavailable. Inspect that worker's status before treating its task as complete.".into(),
        last: true,
    }
}

#[cfg(test)]
mod tests;
