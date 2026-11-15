//! Starting an ended structured session again (protocol 10's
//! `session.resume`, ADR 0026): the same session, its log continued after
//! its final status, its agent started under its current profile with
//! `session/load`.
//!
//! Must not: touch the store (the node reopens the session's row, ADR
//! 0001), or decide who may start a session again (the node's checks).
//!
//! [Decision Log]
//! - Purpose: the references' restore and resume. The session keeps its
//!   id, its subscribers' way back to it, its title, its choices and its
//!   queue; only a new run of its agent starts.
//! - Reuse, not a fork: the launch is a new session's (`run_structured`,
//!   the agent's command and setup built as `create` builds them, with
//!   `session/load` of its harness session and its choices applied after
//!   the load, as a move does), and the agent's replay of the conversation
//!   is dropped until `ready` as a profile move drops it
//!   (`MoveState::replaying`), except that this run's `ready` is published:
//!   a move's session was running already, this one starts.
//! - Its records continue its stored log: the ring and the publisher start
//!   at its `next_seq`, the conversation's projection from its stored
//!   queue, context report and abilities, the attention point from its
//!   stored one. Its queue comes back held `ended` (`Desk::continuing`).
//! - The gate. The node must reopen the row before anything is published
//!   (so retention never prunes it, and the store takes its next record),
//!   and readers must find the session in one place throughout. So the
//!   session is registered, its agent already starting, and publishes
//!   nothing until the node opens the gate; a node that could not reopen
//!   the row drops it, and the session ends without a record.
//!   Alternatives: reopen first and register after (in between, the store
//!   shows a live row the runtime does not have, which a subscription
//!   served from the store cannot tell from a row a failed store left
//!   live); start the agent only once the gate opens (the actor owns a
//!   running agent from its first line).
//! - Cost: an agent that cannot even be executed (its `spawn` fails) is
//!   refused before the row is reopened, so that attempt leaves no record;
//!   one that starts and fails its handshake or `session/load` leaves
//!   `starting`, then `failed` with the reason, as a new session does.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::oneshot;
use vessel_agent::AgentSetup;
use vessel_harness::{ModelProxy, Profile};
use vessel_proto::{
    ContextUsage, EventSeq, HarnessSessionId, OptionChoice, ProfileName, PromptQueue,
    ResolvedMcpServers, SessionInfo, SessionMarks, SessionMode, SessionStatus,
};

use crate::conversation::Seen;
use crate::delivery::Desk;
use crate::error::SessionError;
use crate::moving::Launch;
use crate::runtime::{ActorStart, AgentCommand, Begin, Created, SessionRuntime};

/// An ended structured session as its stored log left it, to start it
/// again ([`SessionRuntime::start_again`]).
#[derive(Debug, Clone)]
pub struct Ended {
    /// Its stored info: id, spec, creator, origin, the home it came
    /// through, its model route, current profile, title, the harness
    /// session `session/load` resumes, its recorded abilities, and
    /// `next_seq`, where its log continues.
    pub info: SessionInfo,
    /// The choices it last ran with, applied after the load (as a move
    /// applies them), so it keeps its mode rather than the agent's default.
    pub choices: Vec<OptionChoice>,
    /// The prompts that waited when it ended.
    pub queue: PromptQueue,
    /// Its agent's last context report, until the new run reports one.
    pub usage: Option<ContextUsage>,
    /// The last record of its log that asked for attention.
    pub attention: Option<EventSeq>,
    /// The profiles its moves linked its files into (either side of each
    /// `profile_changed` its log holds; `None` the default). Its
    /// conversation is one file under each of them, so a start again
    /// claims it under all (`runtime::claims`).
    pub moved_through: BTreeSet<Option<ProfileName>>,
}

/// What a start again runs with: the node's choices for this launch, as
/// [`LaunchOptions`](crate::LaunchOptions) gives them for a new session.
/// Who the session is (its id, creator, origin, route) is its stored
/// info's.
#[derive(Debug, Clone, Default)]
pub struct Relaunch {
    /// Its current profile, resolved; `None` is the harness's default
    /// (right only when the session runs under no other).
    pub profile: Option<Profile>,
    /// Local quota binding captured with this actual relaunch.
    pub limits: crate::LimitAssociation,
    /// MCP servers, values resolved, the agent's own `vessel` server with
    /// a new credential included (ADR 0013).
    pub mcp_servers: ResolvedMcpServers,
    /// Lines for the log right after `starting`, as
    /// [`LaunchOptions::notices`](crate::LaunchOptions::notices).
    pub notices: Vec<String>,
    /// The model proxy, with a token for this run, when its profile has a
    /// model route (ADR 0015).
    pub model_proxy: Option<ModelProxy>,
    /// Whether its agent may ask questions, as for a new session of its
    /// creator.
    pub questions: bool,
}

/// A session starting again: registered, its agent starting, publishing
/// nothing until [`gate`](Self::gate) is opened.
#[derive(Debug)]
pub struct Reopening {
    /// The session. `durable` gets its first record once the gate opens;
    /// it closes without one if the gate is dropped.
    pub created: Created,
    /// Open it once the store has reopened the session's row.
    pub gate: Gate,
}

/// Lets a session started again publish ([`Reopening::gate`]). Dropped
/// unopened, the session ends without a record and its agent is stopped.
#[derive(Debug)]
pub struct Gate(oneshot::Sender<()>);

impl Gate {
    /// The session's row is reopened: its run may publish.
    pub fn open(self) {
        if self.0.send(()).is_err() {
            tracing::debug!("the session was gone before it could start again");
        }
    }
}

/// Why an ended session cannot start again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotResumable {
    /// A terminal session: its harness's own interface owns the
    /// conversation, and Vessel cannot load it into another.
    #[error("a terminal session cannot start again; its harness's own interface owns it")]
    Terminal,
    /// Its agent said, in its last handshake, that it cannot resume a
    /// conversation (no ACP `session/load`).
    #[error("its agent cannot resume a conversation (it does not offer `session/load`)")]
    AgentCannot,
    /// It never had a conversation of its agent's to load: it ended before
    /// its agent was ready.
    #[error("it ended before its agent was ready, so there is no conversation to resume")]
    NoConversation,
}

/// The conversation `info`'s session would resume, if it can start again.
/// A session stored before protocol 10 has no recorded abilities: it is
/// tried, and the agent's handshake decides.
///
/// # Errors
/// Why it cannot.
pub fn resumable(info: &SessionInfo) -> Result<&HarnessSessionId, NotResumable> {
    if matches!(info.spec.mode, SessionMode::Terminal { .. }) {
        return Err(NotResumable::Terminal);
    }
    if info.abilities.is_some_and(|abilities| !abilities.resume) {
        return Err(NotResumable::AgentCannot);
    }
    info.harness_session
        .as_ref()
        .ok_or(NotResumable::NoConversation)
}

impl SessionRuntime {
    /// Start an ended structured session again (protocol 10's
    /// `session.resume`, ADR 0026): its agent under its current profile
    /// with `session/load`, its log continuing at its `next_seq`, its
    /// title, choices and creator its own, its queue back held. The
    /// session is registered at once and publishes nothing until the gate
    /// is opened (see the module's Decision Log); then `starting`, its
    /// agent's replay dropped, `ready` and `running` as for a new session,
    /// or `failed` with the reason if the agent does not start.
    ///
    /// # Errors
    /// [`SessionError::NotResumable`], [`SessionError::DuplicateId`] for a
    /// session the runtime still has, [`SessionError::ConversationTaken`]
    /// while another live session is on its conversation, as
    /// [`create`](Self::create) for the launch, and
    /// [`SessionError::ShuttingDown`].
    pub async fn start_again(
        &self,
        ended: Ended,
        launch: Relaunch,
    ) -> Result<Reopening, SessionError> {
        // Counts as a running task, as a creation does.
        let _in_flight = self.tasks_token();
        if self.is_shutting_down() {
            return Err(SessionError::ShuttingDown);
        }
        let Ended {
            mut info,
            choices,
            queue,
            usage,
            attention,
            moved_through,
        } = ended;
        let harness_session = resumable(&info)?.clone();
        if self.get(info.id).is_some() {
            return Err(SessionError::DuplicateId(info.id));
        }
        // One agent per conversation (`runtime::claims`), under every
        // profile its files are in: the one it was created under, the ones
        // its moves linked, and its current one. Held until the session is
        // registered, then kept.
        let mut profiles = moved_through;
        let _created = profiles.insert(info.spec.profile.clone());
        let _current = profiles.insert(info.profile.clone());
        let claim =
            Arc::new(self.claim(info.id, &info.spec.harness, profiles, &harness_session)?);
        let Relaunch {
            profile,
            limits,
            mcp_servers,
            mut notices,
            model_proxy,
            questions,
        } = launch;
        let profile = profile.map_or_else(|| Profile::default_of(&info.spec.harness), Ok)?;
        // Launched under its current profile, which a move may have
        // changed since its spec was written.
        let spec = info.spec.clone().with_profile(profile.name().clone());
        // Its driver chosen again, as for a new session: it ended, so
        // nothing of its last run's agent carries on (`runtime::drivers`).
        let AgentCommand {
            command,
            cwd,
            driver,
            notices: chosen,
        } = self
            .agent_command_owned(spec, profile, model_proxy, Arc::clone(&claim))
            .await?;
        notices.extend(chosen);
        // The model it asked for at creation, as a move passes it on.
        let model = match &info.spec.mode {
            SessionMode::Structured { model, .. } => model.clone(),
            SessionMode::Terminal { .. } => None,
        };
        let setup = AgentSetup {
            cwd,
            model,
            resume: Some(harness_session),
            options: choices,
            mcp_servers,
            handshake_timeout: self.handshake_timeout(),
            questions,
            fork: None,
            system_prompt: None,
            private_dir: self.private_dir(),
        };
        // The run begins: its first record is its stored log's next.
        info.status = SessionStatus::Starting;
        info.ended_at = None;
        info.marks = SessionMarks::default();
        info.board = None;
        let seen = Seen::continuing(queue.clone(), usage, info.abilities);
        let begin = Begin {
            limits: limits.clone(),
            info,
            since: Utc::now(),
            seen,
            attention,
        };
        let (gate, opened) = oneshot::channel();
        let actor = ActorStart {
            desk: Desk::continuing(queue),
            replaying: true,
            gate: Some(opened),
            opening: None,
        };
        let launch = Launch {
            limits,
            command,
            setup,
            driver,
        };
        let created = self.run_structured(launch, begin, notices, actor)?;
        claim.keep();
        Ok(Reopening {
            created,
            gate: Gate(gate),
        })
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::{AgentAbilities, HarnessKind, SessionId, SessionSpec, TerminalSize};

    use super::*;

    fn info(mode: SessionMode) -> SessionInfo {
        SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(HarnessKind::ClaudeCode, mode),
            Utc::now(),
        )
    }

    /// A terminal session, an agent that said it cannot load, and a
    /// session with no conversation are refused; one stored before its
    /// abilities were recorded is tried.
    #[test]
    fn what_can_start_again() {
        let terminal = info(SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize::default(),
        });
        assert_eq!(resumable(&terminal), Err(NotResumable::Terminal));
        let mut structured = info(SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        });
        assert_eq!(resumable(&structured), Err(NotResumable::NoConversation));
        let conversation = HarnessSessionId::new("conversation");
        structured.harness_session = Some(conversation.clone());
        assert_eq!(resumable(&structured), Ok(&conversation), "never said");
        structured.abilities = Some(AgentAbilities::default());
        assert_eq!(resumable(&structured), Err(NotResumable::AgentCannot));
        structured.abilities = Some(AgentAbilities {
            resume: true,
            ..AgentAbilities::default()
        });
        assert_eq!(resumable(&structured), Ok(&conversation));
    }
}
