//! The ten schedule calls, one command.
//!
//! `link`'s root is at its line budget, so the calls share
//! [`crate::Command::Schedules`] the way workspace and devices share one
//! ask. A forwarded peer is not sent create, update or run, and an older
//! peer is not sent anything: a bug in the screen must not change
//! `defined_by` or invent a preview (ADR 0072). Definitions are not logged.

use vessel_app_view::schedules::{
    BlockedAct, BookName, PreviewLane, SchedulePeer, ScheduleReach, Ticket,
};
use vessel_proto::automation::{
    CreateSchedule, PreviewSchedule, ScheduleInfo, ScheduleList, SchedulePreview, ScheduleRun,
    UpdateSchedule,
};
use vessel_proto::{Method, ProtocolVersion, ScheduleId, UnexpectedReply, WireError};

#[cfg(not(target_arch = "wasm32"))]
use vessel_client::{CallError, NodeClient};

#[cfg(not(target_arch = "wasm32"))]
use crate::State;

/// One schedule call. The screen fills `peer` from the handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// Correlates the answer with this ask.
    pub ticket: Ticket,
    /// The node-book name the connection map uses.
    pub machine: BookName,
    /// Older, direct, or forwarded, and whether preview is offered.
    pub peer: SchedulePeer,
    /// Which call.
    pub kind: CallKind,
}

/// The ten calls. Pause, resume and delete stay available on a forwarded
/// peer; create, update and run do not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallKind {
    /// `schedule.list`.
    List,
    /// `schedule.get`.
    Get(ScheduleId),
    /// `schedule.runs`.
    Runs(ScheduleId),
    /// `schedule.preview`. Built only when the peer offers preview.
    Preview(PreviewSchedule),
    /// `schedule.create`. `paused` is the protocol field; the screen sends false.
    Create(CreateSchedule),
    /// `schedule.update`. The node sets `defined_by` to the caller.
    Update(UpdateSchedule),
    /// `schedule.pause`. Does not change `defined_by`.
    Pause(ScheduleId),
    /// `schedule.resume`. Does not change `defined_by`.
    Resume(ScheduleId),
    /// `schedule.run`.
    Run(ScheduleId),
    /// `schedule.delete`. Does not change `defined_by`.
    Delete(ScheduleId),
}

impl Call {
    /// The answer when this call is not sent.
    #[must_use]
    pub fn refused(self, failure: Failure) -> Answer {
        Answer {
            ticket: self.ticket,
            machine: self.machine,
            body: Err(failure),
        }
    }
}

/// Why a call was not sent, or why the node or the connection refused it.
/// `Remote` keeps the [`WireError`], including a preview's typed detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No connection for that machine.
    NotConnected,
    /// The link's queue did not take the command.
    Unsent,
    /// Create, update or run toward a node reached only by forwarding.
    Forwarded(BlockedAct),
    /// The handshake is older than the call. Nothing was sent.
    OlderPeer,
    /// This build has no node transport (the browser).
    Unsupported,
    /// The connection closed before the response.
    Disconnected,
    /// The node answered with an error.
    Remote(WireError),
    /// The node answered for a different method.
    Unexpected(UnexpectedReply),
    /// The connection has used every request id.
    RequestIdsExhausted,
    /// The call needs a newer protocol than the connection negotiated.
    NotInProtocol {
        /// The method.
        method: Method,
        /// The version the call needs.
        needs: ProtocolVersion,
        /// The connection's version.
        negotiated: ProtocolVersion,
    },
}

/// What the node answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// `schedule.list`.
    Listed(ScheduleList),
    /// `schedule.get`.
    Got(ScheduleInfo),
    /// `schedule.runs`, newest first, at most fifty.
    Runs(Vec<ScheduleRun>),
    /// `schedule.preview`.
    Preview(SchedulePreview),
    /// `schedule.create`.
    Created(ScheduleInfo),
    /// `schedule.update`.
    Updated(ScheduleInfo),
    /// `schedule.pause`.
    Paused(ScheduleInfo),
    /// `schedule.resume`.
    Resumed(ScheduleInfo),
    /// `schedule.run`.
    Ran(ScheduleRun),
    /// `schedule.delete`.
    Deleted,
}

/// A schedule call's answer, tagged with the ask that made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The ask's ticket.
    pub ticket: Ticket,
    /// The machine that was asked.
    pub machine: BookName,
    /// The reply, or why there is none.
    pub body: Result<Reply, Failure>,
}

/// `Some` when this peer must not be sent this call. Pause, resume and
/// delete on a forwarded peer are `None`: the node may still refuse, and
/// that refusal comes back as [`Failure::Remote`].
#[must_use]
pub fn refuse(peer: &SchedulePeer, kind: &CallKind) -> Option<Failure> {
    let SchedulePeer::Ready(ready) = peer else {
        return Some(Failure::OlderPeer);
    };
    if ready.preview == PreviewLane::Older && matches!(kind, CallKind::Preview(_)) {
        return Some(Failure::OlderPeer);
    }
    let blocked = match kind {
        CallKind::Create(_) => Some(BlockedAct::Create),
        CallKind::Update(_) => Some(BlockedAct::Update),
        CallKind::Run(_) => Some(BlockedAct::Run),
        CallKind::List
        | CallKind::Get(_)
        | CallKind::Runs(_)
        | CallKind::Preview(_)
        | CallKind::Pause(_)
        | CallKind::Resume(_)
        | CallKind::Delete(_) => None,
    };
    match (ready.reach, blocked) {
        (ScheduleReach::Forwarded, Some(act)) => Some(Failure::Forwarded(act)),
        (ScheduleReach::Direct, _) | (ScheduleReach::Forwarded, None) => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    /// Send one schedule call, or answer without sending when [`refuse`] says so.
    pub(crate) fn schedules(&self, call: Call) {
        let events = self.events.clone();
        if let Some(failure) = refuse(&call.peer, &call.kind) {
            drop(tokio::spawn(async move {
                let answer = call.refused(failure);
                if events.send(crate::Event::Schedule(answer)).await.is_err() {
                    tracing::debug!("a schedule answered after the app closed");
                }
            }));
            return;
        }
        let client = self
            .clients
            .get(call.machine.as_str())
            .map(|connected| connected.client.clone());
        drop(tokio::spawn(async move {
            let body = match client {
                Some(client) => perform(&client, call.kind).await,
                None => {
                    tracing::debug!("a schedule call found no connection");
                    Err(Failure::NotConnected)
                }
            };
            let answer = Answer {
                ticket: call.ticket,
                machine: call.machine,
                body,
            };
            if events.send(crate::Event::Schedule(answer)).await.is_err() {
                tracing::debug!("a schedule answered after the app closed");
            }
        }));
    }
}

/// Run a call the peer allows. The definition is not logged.
///
/// # Errors
/// The client's [`CallError`], mapped once. `Remote` keeps the wire error.
#[cfg(not(target_arch = "wasm32"))]
async fn perform(client: &NodeClient, kind: CallKind) -> Result<Reply, Failure> {
    match kind {
        CallKind::List => lift(client.schedules().await, |schedules| {
            Reply::Listed(ScheduleList { schedules })
        }),
        CallKind::Get(id) => lift(client.schedule(id).await, Reply::Got),
        CallKind::Runs(id) => lift(client.schedule_runs(id).await, Reply::Runs),
        CallKind::Preview(request) => lift(client.preview_schedule(request).await, Reply::Preview),
        CallKind::Create(CreateSchedule { definition, paused }) => lift(
            client.create_schedule(definition, paused).await,
            Reply::Created,
        ),
        CallKind::Update(UpdateSchedule {
            schedule,
            revision,
            definition,
        }) => lift(
            client.update_schedule(schedule, revision, definition).await,
            Reply::Updated,
        ),
        CallKind::Pause(id) => lift(client.pause_schedule(id).await, Reply::Paused),
        CallKind::Resume(id) => lift(client.resume_schedule(id).await, Reply::Resumed),
        CallKind::Run(id) => lift(client.run_schedule(id).await, Reply::Ran),
        CallKind::Delete(id) => lift(client.delete_schedule(id).await, |()| Reply::Deleted),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn lift<T>(result: Result<T, CallError>, map: impl FnOnce(T) -> Reply) -> Result<Reply, Failure> {
    result.map(map).map_err(from_call)
}

#[cfg(not(target_arch = "wasm32"))]
fn from_call(error: CallError) -> Failure {
    match error {
        CallError::Disconnected => Failure::Disconnected,
        CallError::Remote(error) => Failure::Remote(error),
        CallError::UnexpectedReply(reply) => Failure::Unexpected(reply),
        CallError::RequestIdsExhausted => Failure::RequestIdsExhausted,
        CallError::NotInProtocol {
            method,
            needs,
            negotiated,
        } => Failure::NotInProtocol {
            method,
            needs,
            negotiated,
        },
    }
}

#[cfg(test)]
mod tests {
    use vessel_app_view::schedules::{PreviewLane, ReadyPeer, SchedulePeer, ScheduleReach};
    use vessel_proto::automation::{
        Cadence, CatchUp, EverySeconds, PreviewCount, PreviewSchedule, PromptText,
        ScheduleDefinition, ScheduleTarget, UpdateSchedule,
    };
    use vessel_proto::{ScheduleId, SessionId};

    use super::{BlockedAct, BookName, Call, CallKind, Failure, Ticket, refuse};

    fn every() -> Cadence {
        Cadence::Every {
            seconds: EverySeconds::try_from(3600).expect("an hour"),
            anchor: None,
        }
    }

    fn definition() -> ScheduleDefinition {
        ScheduleDefinition {
            name: None,
            cadence: every(),
            target: ScheduleTarget::Session {
                session: SessionId::random(),
            },
            prompt: "Review the queue".parse::<PromptText>().expect("a prompt"),
            catch_up: CatchUp::Once,
            max_runs: None,
            ends_at: None,
        }
    }

    fn ready(reach: ScheduleReach, preview: PreviewLane) -> SchedulePeer {
        SchedulePeer::Ready(ReadyPeer {
            preview,
            zone: None,
            reach,
        })
    }

    fn kinds() -> Vec<CallKind> {
        let id = ScheduleId::random();
        let definition = definition();
        vec![
            CallKind::List,
            CallKind::Get(id),
            CallKind::Runs(id),
            CallKind::Preview(PreviewSchedule {
                cadence: every(),
                after: None,
                count: PreviewCount::default(),
            }),
            CallKind::Create(vessel_proto::automation::CreateSchedule {
                definition: definition.clone(),
                paused: false,
            }),
            CallKind::Update(UpdateSchedule {
                schedule: id,
                revision: 1,
                definition,
            }),
            CallKind::Pause(id),
            CallKind::Resume(id),
            CallKind::Run(id),
            CallKind::Delete(id),
        ]
    }

    #[test]
    fn an_older_peer_refuses_every_call_and_a_direct_peer_refuses_none() {
        for kind in kinds() {
            assert_eq!(
                refuse(&SchedulePeer::Older, &kind),
                Some(Failure::OlderPeer)
            );
            assert_eq!(
                refuse(&ready(ScheduleReach::Direct, PreviewLane::Offered), &kind),
                None
            );
        }
    }

    #[test]
    fn a_forwarded_peer_refuses_create_update_and_run_only() {
        let peer = ready(ScheduleReach::Forwarded, PreviewLane::Offered);
        for kind in kinds() {
            let refused = refuse(&peer, &kind);
            match &kind {
                CallKind::Create(_) => {
                    assert_eq!(refused, Some(Failure::Forwarded(BlockedAct::Create)))
                }
                CallKind::Update(_) => {
                    assert_eq!(refused, Some(Failure::Forwarded(BlockedAct::Update)))
                }
                CallKind::Run(_) => assert_eq!(refused, Some(Failure::Forwarded(BlockedAct::Run))),
                CallKind::List
                | CallKind::Get(_)
                | CallKind::Runs(_)
                | CallKind::Preview(_)
                | CallKind::Pause(_)
                | CallKind::Resume(_)
                | CallKind::Delete(_) => assert_eq!(refused, None),
            }
        }
    }

    #[test]
    fn an_older_preview_lane_refuses_only_preview() {
        let direct = ready(ScheduleReach::Direct, PreviewLane::Older);
        let forwarded = ready(ScheduleReach::Forwarded, PreviewLane::Older);
        for kind in kinds() {
            let direct_refusal = if matches!(kind, CallKind::Preview(_)) {
                Some(Failure::OlderPeer)
            } else {
                None
            };
            assert_eq!(refuse(&direct, &kind), direct_refusal);
            assert_eq!(
                refuse(&forwarded, &kind),
                match &kind {
                    CallKind::Preview(_) => Some(Failure::OlderPeer),
                    CallKind::Create(_) => Some(Failure::Forwarded(BlockedAct::Create)),
                    CallKind::Update(_) => Some(Failure::Forwarded(BlockedAct::Update)),
                    CallKind::Run(_) => Some(Failure::Forwarded(BlockedAct::Run)),
                    CallKind::List
                    | CallKind::Get(_)
                    | CallKind::Runs(_)
                    | CallKind::Pause(_)
                    | CallKind::Resume(_)
                    | CallKind::Delete(_) => None,
                }
            );
        }
    }

    #[test]
    fn a_refusal_keeps_the_ticket_and_the_machine() {
        let call = Call {
            ticket: Ticket::new(7),
            machine: BookName::from_book("laptop"),
            peer: SchedulePeer::Older,
            kind: CallKind::List,
        };
        let answer = call.refused(Failure::Unsent);
        assert_eq!(answer.ticket, Ticket::new(7));
        assert_eq!(answer.machine.as_str(), "laptop");
        assert_eq!(answer.body, Err(Failure::Unsent));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_remote_error_keeps_the_wire_error() {
        use vessel_client::CallError;
        use vessel_proto::{ErrorCode, WireError};

        let wire = WireError {
            code: ErrorCode::Forbidden,
            message: "no".to_owned(),
            detail: None,
        };
        assert_eq!(
            super::from_call(CallError::Remote(wire.clone())),
            Failure::Remote(wire)
        );
    }
}
