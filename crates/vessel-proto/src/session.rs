//! Sessions: what to run, how it is going, and what can be sent to it.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SessionLabels;
use crate::agent::{ContentBlock, PermissionOutcome};
use crate::agents::HarnessRef;
use crate::board::SessionBoard;
use crate::conversation::{AgentAbilities, QuestionAnswer, SessionMarks};
use crate::id::{
    DeviceId, EventSeq, HarnessSessionId, ModelId, NodeId, PermissionId, QuestionId, SessionId,
};
use crate::model::SessionModel;
use crate::name::ProfileName;
use crate::options::OptionChoice;
use crate::profile::Creator;
use crate::terminal::{TerminalInput, TerminalSize};
use crate::title::SessionTitle;
use crate::workspace::{SessionPlace, SessionWorkspace};

/// How a harness is driven.
///
/// [Decision Log]
/// - Purpose: Vessel must keep every harness feature as-is (ORIGIN item 2)
///   and also give clients a structured, harness-independent view.
/// - Alternatives: only a PTY (no structure; clients scrape a TUI); only
///   structured protocols (loses TUI-only features and the familiar UI);
///   one hand-written parser per harness's own JSON stream (Paseo's path:
///   highest fidelity, but N adapters of 5-8k lines each).
/// - Choice: two modes. `Terminal` runs the real TUI in a PTY. `Structured`
///   speaks ACP (Agent Client Protocol) to the harness or its adapter, so one
///   client implementation covers all four harnesses. See ADR 0003.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SessionMode {
    /// The harness's own TUI in a pseudo-terminal; clients attach and see
    /// exactly what a local terminal would.
    Terminal {
        /// Extra arguments passed verbatim to the harness binary.
        #[serde(default)]
        args: Vec<String>,
        /// Initial window size.
        #[serde(default)]
        size: TerminalSize,
    },
    /// The harness driven over ACP; output is a typed event timeline.
    Structured {
        /// Model to request, if not the harness default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<ModelId>,
        /// Resume this harness session instead of starting a new one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resume: Option<HarnessSessionId>,
        /// The agent's config options to set once the session exists, in
        /// order, after `model` (protocol 9, ADR 0022): a permissions mode,
        /// an effort, anything the agent offers. A choice the agent does
        /// not offer fails the session, as a model it does not offer does.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<OptionChoice>,
    },
}

impl SessionMode {
    /// The agent options chosen at creation (protocol 9); none for a
    /// terminal session, whose harness sets its own.
    #[must_use]
    pub fn options(&self) -> &[OptionChoice] {
        match self {
            Self::Structured { options, .. } => options,
            Self::Terminal { .. } => &[],
        }
    }
}

/// What to run. Body of `session.create`.
///
/// `#[non_exhaustive]` with a constructor (protocol 5): every field added
/// since protocol 2 broke each struct literal in the workspace. Build one
/// with [`SessionSpec::new`] and the `with_*` methods; fields stay public
/// for reading and for changing after construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[non_exhaustive]
pub struct SessionSpec {
    /// Harness to launch: one of the four, or (protocol 11) an agent added
    /// on the node.
    pub harness: HarnessRef,
    /// Working directory. Defaults to the node's home directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// The profile to launch with (protocol 5, ADR 0011); `None` is the
    /// harness's default, which is how every earlier session ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// What to call it, as asked at creation (protocol 9). The session's
    /// title now is `SessionInfo.title`, which starts as this and changes
    /// on its own; this stays what was asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SessionTitle>,
    /// Labels asked for at creation (ADR 0063). The map now is
    /// [`SessionInfo::labels`], which starts as this and changes on its
    /// own; this stays what was asked. Left out when there are none.
    #[serde(default, skip_serializing_if = "SessionLabels::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SessionLabels>", optional))]
    pub labels: SessionLabels,
    /// Creation-only discovery override (ADR 0063). Omission can inherit
    /// the creating agent; reusable templates retain that omission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listing: Option<crate::SessionListing>,
    /// Where it runs (protocol 10, ADR 0027): a project's folder or a
    /// worktree. With a place, `cwd` is left out by the client and filled
    /// by the node with the folder the place resolves to, so a client that
    /// predates places still sees where the session runs. Boxed: a spec is
    /// held by every session's info and copied into its futures, and most
    /// sessions have no place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Box<SessionPlace>>,
    /// A separate side conversation of this node-local parent (protocol 19).
    /// Never a shared native conversation id. Defaults absent for older peers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side_question_of: Option<SessionId>,
    /// How to drive it.
    #[serde(flatten)]
    pub mode: SessionMode,
}

impl SessionSpec {
    /// Run `harness` in `mode`, in the node's home directory, with the
    /// default profile.
    #[must_use]
    pub fn new(harness: impl Into<HarnessRef>, mode: SessionMode) -> Self {
        Self {
            harness: harness.into(),
            cwd: None,
            profile: None,
            title: None,
            labels: SessionLabels::new(),
            listing: None,
            place: None,
            side_question_of: None,
            mode,
        }
    }

    /// Running in `place`.
    #[must_use]
    pub fn with_place(mut self, place: SessionPlace) -> Self {
        self.place = Some(Box::new(place));
        self
    }

    /// In this working directory.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// With this profile.
    #[must_use]
    pub fn with_profile(mut self, profile: ProfileName) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Called this.
    #[must_use]
    pub fn with_title(mut self, title: SessionTitle) -> Self {
        self.title = Some(title);
        self
    }

    /// An explicit discovery override, including shown instead of inheritance.
    #[must_use]
    pub fn with_listing(mut self, listing: crate::SessionListing) -> Self {
        self.listing = Some(listing);
        self
    }

    /// With these labels (ADR 0063).
    #[must_use]
    pub fn with_labels(mut self, labels: SessionLabels) -> Self {
        self.labels = labels;
        self
    }
}

/// Lifecycle of the harness process behind a session.
///
/// This is process state only. Whether a structured session is mid-turn is
/// derived from `TurnStarted`/`TurnEnded` events, so there is one source of
/// truth for each fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SessionStatus {
    /// Spawning, or (structured) waiting for the ACP handshake.
    Starting,
    /// The process is alive and accepting input.
    Running,
    /// The process exited on its own or was killed.
    Exited {
        /// Exit code, if the OS reported one (none when killed by a signal).
        code: Option<i32>,
    },
    /// The process could not be started or the handshake failed, or (a
    /// rewind, protocol 14) its conversation could not be started again.
    Failed {
        /// Human-readable reason.
        message: String,
        /// A fork that never held a conversation (protocol 14, ADR 0052):
        /// its agent refused the fork or did not start. Nothing can be
        /// resumed; a client offers to remove it (`session.delete`).
        /// Absent (false) for every other failure and from older nodes.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        empty_fork: bool,
    },
    /// The node stopped while the session was live. The log is intact; a
    /// structured session can be resumed through its `harness_session`.
    Interrupted,
}

impl SessionStatus {
    /// Failed, saying why (not an empty fork).
    #[must_use]
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed {
            message: message.into(),
            empty_fork: false,
        }
    }

    /// Whether the session can still accept input.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

/// A session as reported by the node.
///
/// `#[non_exhaustive]` for the reason given on [`SessionSpec`]; build one
/// with [`SessionInfo::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[non_exhaustive]
pub struct SessionInfo {
    /// Session id.
    pub id: SessionId,
    /// What was asked for.
    pub spec: SessionSpec,
    /// The profile the harness runs with now: `spec.profile` until
    /// `session.move_profile` changes it (protocol 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Who created it (protocol 5, ADR 0013). Absent from older nodes, which
    /// only ever had the owner.
    #[serde(default)]
    pub created_by: Creator,
    /// Durable current-parent boundary (ADR 0090), separate from creation audit.
    /// Absent on older peers and before a human explicitly detaches the child.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detach_receipt: Option<crate::subagents::DetachReceipt>,
    /// Current process state.
    pub status: SessionStatus,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the process ended, if it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Sequence number the next event will get (= number of events so far).
    pub next_seq: EventSeq,
    /// The harness's own session id, once a structured session has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_session: Option<HarnessSessionId>,
    /// The home a call creating it came through, when another node asked
    /// for it (protocol 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<NodeId>,
    /// Where it came from, when not from this node (protocol 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<SessionOrigin>,
    /// The model route it was launched with, and the provider's model its
    /// harness is pinned to (protocol 7). Absent for a session on its
    /// harness's own provider, and from older nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<SessionModel>,
    /// Its place on the board: the lane and the phrase the node derived
    /// from what its agent is doing (protocol 8, ADR 0019). Absent from
    /// older nodes; a client then places it by `status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<SessionBoard>,
    /// What it is called now (protocol 9), the one truth for its current
    /// title: `spec.title` when one was asked for; otherwise, for a
    /// structured session, the first line of its first prompt with text
    /// (`SessionTitle::from_prompt`); and whatever `session.rename` gave it
    /// last. Every change is also a `renamed` event in its log, and the
    /// node's store keeps it across restarts. Absent until it has one, and
    /// from older nodes; a client then names it by its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SessionTitle>,
    /// The labels it carries now (ADR 0063): `spec.labels` at creation,
    /// then whatever `session.labels` set. Left out when there are none,
    /// and stripped for a peer that predates them.
    #[serde(default, skip_serializing_if = "SessionLabels::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SessionLabels>", optional))]
    pub labels: SessionLabels,
    /// Immutable discovery classification, independent of process state and
    /// archival. Older or unassigned peers receive no listing metadata.
    #[serde(default, skip_serializing_if = "crate::SessionListing::is_shown")]
    #[cfg_attr(feature = "ts", ts(as = "Option<crate::SessionListing>", optional))]
    pub listing: crate::SessionListing,
    /// Who produced [`title`](Self::title) (ADR 0062). Absent means
    /// [`TitleOrigin::Unknown`](crate::title::TitleOrigin::Unknown): this
    /// build does not speak the field yet, or the row was stored before
    /// it. A `renamed` event carries the same origin.
    #[serde(default, skip_serializing_if = "crate::title::TitleOrigin::is_unknown")]
    pub title_origin: crate::title::TitleOrigin,
    /// What a person marked on it: pinned, archived, unread (protocol 10,
    /// ADR 0026). Kept by the node, so every client sees the same; left
    /// out when nothing is marked, and absent from older nodes.
    #[serde(default, skip_serializing_if = "SessionMarks::is_empty")]
    pub marks: SessionMarks,
    /// What its agent said it can do in its last handshake (protocol 10):
    /// steer, start again, take images or audio. Kept by the store, so an
    /// ended session says whether `session.resume` can work. Absent for a
    /// terminal session, before the handshake, and from older nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abilities: Option<AgentAbilities>,
    /// Its project, worktree and branch (protocol 10), when its spec named
    /// a place. Absent from older nodes. Boxed, as `SessionSpec.place` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Box<SessionWorkspace>>,
    /// Its pull request, for a live session with a project or a worktree
    /// (protocol 11, ADR 0029): whether the node can see it, the last one
    /// read, and whether feedback goes to the session. Set by the node;
    /// cleared when the session ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<Box<crate::forge::SessionPullRequest>>,
    /// Its agent's process is parked (protocol 12, ADR 0038): stopped
    /// while the session stays `running`, started again by the next
    /// prompt. Absent while it runs, for an ended session, and from older
    /// nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked: Option<crate::parking::Parked>,
}

/// Where a session came from, when another node of the fleet had a hand in
/// it (protocol 6). `created_by` says who created it on this node; an
/// agent's own node is in `created_by` already, so `remote` names only the
/// owner or a device of the other node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionOrigin {
    /// Created by a call forwarded from another node.
    Remote {
        /// The node the call started on.
        node: NodeId,
        /// Who started it there.
        by: RemoteCreator,
    },
    /// Imported from a session on another node (`session.import`).
    Imported {
        /// The node it was copied from.
        node: NodeId,
        /// The session there.
        session: SessionId,
    },
}

/// The owner or a device of another node, for [`SessionOrigin::Remote`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteCreator {
    /// That node's owner.
    Owner,
    /// A device paired with that node.
    Device {
        /// Its id on that node.
        id: DeviceId,
    },
}

impl SessionInfo {
    /// A session that is just starting: no events yet, running the profile
    /// it asked for, called what it asked to be, created by the owner.
    #[must_use]
    pub fn new(id: SessionId, spec: SessionSpec, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            profile: spec.profile.clone(),
            title: spec.title.clone(),
            labels: spec.labels.clone(),
            listing: spec.listing.unwrap_or_default(),
            title_origin: if spec.title.is_some() {
                crate::title::TitleOrigin::Typed
            } else {
                crate::title::TitleOrigin::Unknown
            },
            spec,
            created_by: Creator::Owner,
            detach_receipt: None,
            status: SessionStatus::Starting,
            created_at,
            ended_at: None,
            next_seq: EventSeq::ZERO,
            harness_session: None,
            via: None,
            origin: None,
            model: None,
            board: None,
            marks: SessionMarks::default(),
            abilities: None,
            workspace: None,
            pull_request: None,
            parked: None,
        }
    }

    /// Created by `creator`.
    #[must_use]
    pub fn with_creator(mut self, creator: Creator) -> Self {
        self.created_by = creator;
        self
    }
}

/// Something a client sends into a live session. Body of `session.send`.
///
/// Sending a variant that does not match the session's mode fails with
/// `ErrorCode::Unsupported`; the node never guesses a conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionInput {
    /// Terminal mode: keystrokes or a resize.
    Terminal {
        /// The input.
        input: TerminalInput,
    },
    /// Structured mode: start a user turn.
    Prompt {
        /// The user's message.
        content: Vec<ContentBlock>,
    },
    /// Structured mode: cancel the turn in progress.
    CancelTurn {
        /// The turn meant: its `turn_started` record's `seq` (protocol 10).
        /// When another turn runs by the time the cancel lands (the queue
        /// started the next one as the first ended), that turn goes on
        /// and only the queue is held. Absent: whatever turn runs, as
        /// before protocol 10.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn: Option<EventSeq>,
    },
    /// Structured mode: answer a pending permission request. The first answer
    /// wins; later answers fail with `ErrorCode::Conflict`.
    AnswerPermission {
        /// Which request.
        id: PermissionId,
        /// The choice.
        outcome: PermissionOutcome,
    },
    /// Structured mode: answer a question the agent asks (protocol 10).
    /// The first answer wins; later answers fail with
    /// `ErrorCode::Conflict`, and one that does not fit the question's
    /// form with `ErrorCode::InvalidRequest`.
    AnswerQuestion {
        /// Which question.
        id: QuestionId,
        /// The answer.
        answer: QuestionAnswer,
    },
}
