//! Protocol 12's `session.fork` and `session.rewind` (track A2, ADR 0052):
//! the point read from the session's log, a fork started through
//! `session.create`'s own path, a rewind handed to the session's actor.
//!
//! [Decision Log]
//! - Purpose: a person forks a conversation from a turn's end into a new
//!   session, or takes a session back to one.
//! - The point is the `turn_ended` record's (`TurnPoint`): the agent's
//!   message and the conversation it is in, set by the agent's driver. So
//!   a fork from a turn a rewind left behind takes the conversation that
//!   still holds it, and nothing here scans the log for the id in force.
//! - A fork is a session its caller creates (`create_starting`): the same
//!   refusals, grants, credentials, limits and model token; its MCP
//!   servers are resolved for it, never copied from the source's launch.
//!   It runs in the source's folder (a place elsewhere would put a Claude
//!   fork's transcript where a later move of the fork would not find it).
//! - A rewind counts what it leaves behind that changed files, from the
//!   stored records: the files are not put back, and the record says so.

use vessel_agent::ForkFrom;
use vessel_proto::forking::{ForkSession, RewindSession};
use vessel_proto::{
    AgentEvent, EventSeq, SessionEvent, SessionId, SessionInfo, SessionMode, SessionTitle,
    ToolKind, TurnPoint, WireError,
};
use vessel_session::ForkStart;

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

/// How many records a rewind reads at a time to count what it leaves.
const PAGE: u32 = 512;

impl NodeService {
    /// `session.fork`: a new session continuing `request.session`'s
    /// conversation from a turn's end, or as it is.
    pub(crate) async fn fork_session(
        &self,
        connection: &Connection,
        request: ForkSession,
    ) -> Result<SessionInfo, WireError> {
        let (spec, fork) = self.fork_of(request).await.map_err(|error| wire(&error))?;
        self.create_starting(&connection.principal, spec, (None, None), Some(fork))
            .await
    }

    /// `session.rewind`: the session goes back to a turn's end.
    pub(crate) async fn rewind_session(
        &self,
        _: &Connection,
        request: RewindSession,
    ) -> Result<SessionInfo, WireError> {
        let rewound = async {
            let handle = self.sessions.acting_on(request.session).await?;
            let point = self.point(request.session, request.after).await?;
            let files_changed = self.files_changed(request.session, request.after).await?;
            handle
                .rewind(point, request.after, files_changed)
                .await
                .map_err(ServiceError::Session)
        };
        rewound
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }

    /// What a fork of `request` starts as: the source's spec, in its
    /// folder, with its choices as they stand, and where its conversation
    /// comes from.
    async fn fork_of(
        &self,
        request: ForkSession,
    ) -> Result<(vessel_proto::SessionSpec, ForkStart), ServiceError> {
        let id = request.session;
        let source = self.sessions.get(id).await?;
        let SessionMode::Structured { model, .. } = &source.spec.mode else {
            return Err(ServiceError::NotAPoint {
                session: id,
                why: "a terminal session's conversation is its harness's",
            });
        };
        let agent = if let Some(after) = request.after {
            let point = self.point(id, after).await?;
            ForkFrom {
                session: point.conversation,
                point: Some(point.message),
            }
        } else {
            if let Some(handle) = self.sessions.live(id)
                && handle.turn_running().await.unwrap_or(false)
            {
                return Err(ServiceError::ForkRunning(id));
            }
            let Some(conversation) = source.harness_session.clone() else {
                return Err(ServiceError::NotAPoint {
                    session: id,
                    why: "its agent has not started a conversation",
                });
            };
            ForkFrom {
                session: conversation,
                point: None,
            }
        };
        let choices = self.sessions.choices(id).await?;
        let mut spec = source.spec.clone();
        spec.mode = SessionMode::Structured {
            model: model.clone(),
            resume: None,
            options: choices,
        };
        // The folder the source's place resolved to stays in `cwd`.
        spec.place = None;
        spec.title = fork_title(source.title.as_ref());
        Ok((
            spec,
            ForkStart {
                from: id,
                after: request.after,
                agent,
            },
        ))
    }

    /// The point record `after` of session `id` carries.
    async fn point(&self, id: SessionId, after: EventSeq) -> Result<TurnPoint, ServiceError> {
        let page = self.sessions.read(id, after, 1).await?;
        let record = page
            .records
            .into_iter()
            .find(|record| record.seq == after)
            .ok_or(ServiceError::NoRecord {
                session: id,
                seq: after,
            })?;
        let SessionEvent::Agent {
            event: AgentEvent::TurnEnded { point, .. },
        } = record.event
        else {
            return Err(ServiceError::NotAPoint {
                session: id,
                why: "a fork or a rewind starts at a turn's end (its turn_ended record)",
            });
        };
        point.ok_or(ServiceError::NotAPoint {
            session: id,
            why: "the agent named no message at that turn's end (an older record, an agent that \
                  sends no message ids, or a turn that ended on a tool call)",
        })
    }

    /// Tool calls after record `after` that edited, deleted or moved
    /// files: what a rewind to it leaves changed.
    async fn files_changed(&self, id: SessionId, after: EventSeq) -> Result<u32, ServiceError> {
        let mut count = 0_u32;
        let mut from = EventSeq::new(after.get().saturating_add(1));
        loop {
            let page = self.sessions.read(id, from, PAGE).await?;
            for record in &page.records {
                if let SessionEvent::Agent {
                    event: AgentEvent::ToolCall(call),
                } = &record.event
                    && matches!(
                        call.kind,
                        ToolKind::Edit | ToolKind::Delete | ToolKind::Move
                    )
                {
                    count = count.saturating_add(1);
                }
            }
            if page.at_end || page.records.is_empty() {
                return Ok(count);
            }
            from = page.next;
        }
    }
}

/// "<title> (fork)", cut to a title's bound.
fn fork_title(title: Option<&SessionTitle>) -> Option<SessionTitle> {
    let base = title.map_or("a session", SessionTitle::as_str);
    SessionTitle::from_prompt(&format!("{base} (fork)"))
}
