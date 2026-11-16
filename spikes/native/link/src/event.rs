//! What the link reports ([`Event`], [`KeptLists`]), moved unchanged out of the crate root.
//!
//! [Decision Log]
//! - Purpose: every package registers its commands and events here, so the
//!   crate root stays under the 500-line budget and leaves the size list.
//! - Constraint: the type, its variants and their documentation are exactly
//!   what they were in `lib.rs`; the crate root re-exports it, so every
//!   caller keeps `vessel_app_link::Event`.
//! - Alternatives: raise the root's recorded size for each package (the
//!   list only shrinks), or each package extracting its own piece.
//! - Choice: one shared move before the packages start.
//! - Cost: one more file; no API, behaviour or dependency change.

use super::{
    ActAnswer, agents, attachments, devices, machines, node_settings, search, sync, workspace,
};
use vessel_app_view::{Card, Fleet};
use vessel_proto::{AgentCommand, EventRecord, HarnessRef, SessionId, SessionOption};

/// What the link reports.
#[derive(Debug)]
pub enum Event {
    /// Keyed result retaining original workspace, tab, admission and query.
    GitPanel(crate::git_panel::Answer),
    /// An original-admission guarded history result or correlated refusal.
    History(crate::history::Answer),
    /// An Agents response, including background install progress.
    Agents {
        /// Originating machine.
        machine: agents::MachineKey,
        /// Original request ticket.
        request: vessel_proto::RequestId,
        /// Facts or typed failure.
        answer: Result<agents::Answer, agents::AgentsError>,
    },

    /// A saved-machine operation completed.
    Machines(machines::Answer),
    /// The machines and their sessions, read again.
    Fleet(Fleet),
    /// A Devices request answered, with its identity.
    Devices(devices::Answer),
    /// A known physical admission closed; never a semantic read refusal.
    DeviceRetired(vessel_app_view::connection::ConnectionTag),
    /// Safe, correlated sharing status.
    Sync(sync::Answer),
    /// A machine's settings in force, after a read or a change.
    NodeSettings(node_settings::Answer),
    /// Records of the session being followed.
    Records {
        /// The session.
        session: SessionId,
        /// The records, in order.
        records: Vec<EventRecord>,
    },
    /// A session started from the app, to open.
    Started(Card),
    /// A step of a composer's file on its way to its session.
    Upload {
        /// The session.
        session: SessionId,
        /// The file's place in the composer's tray.
        key: vessel_app_view::composer::tray::ItemKey,
        /// What happened.
        step: attachments::Step,
    },
    /// A prompt was delivered.
    Sent {
        /// The session.
        session: SessionId,
    },
    /// Pairing worked.
    Paired {
        /// The machine's name in the book (as the sidebar shows it).
        node: String,
        /// How the machine answered, said in words.
        via: vessel_app_view::Via,
        /// The address it answered at (a tooltip's detail only).
        address: String,
    },
    /// Pairing failed; the words to show beside the form.
    PairFailed(vessel_app_view::pairing::PairProblem),
    /// A session act's answer.
    Acted(ActAnswer),
    /// An ended session runs again, and the prompt was sent to it.
    Resumed {
        /// The session.
        session: SessionId,
    },
    /// An ended session could not be started again; the node's words.
    ResumeFailed {
        /// The session.
        session: SessionId,
        /// Why.
        why: String,
    },
    /// What an agent offers, or why the node could not say.
    Options {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
        /// Its options in its order, or the words of the failure.
        outcome: Result<Vec<SessionOption>, String>,
    },
    /// A machine connected: the lists its agents gave last time, kept on
    /// this device (newest first), to show until they answer again. Sent
    /// before the fleet that has the machine online, only when there are
    /// any.
    OptionsKept {
        /// The machine.
        machine: String,
        /// Each agent's lists.
        lists: Vec<KeptLists>,
    },
    /// Something failed; the words to show.
    Failed(String),
    /// The session's machine is not connected any more (its records
    /// stopped, or a prompt or input found no connection): said once in the
    /// session's bar in the app's words; `why`, the transport's own words,
    /// only in that line's tooltip.
    SessionLost {
        /// The session.
        session: SessionId,
        /// The transport's words.
        why: String,
    },
    /// The machine list does not read (the app words it, in its language;
    /// a pairing under way stops with the same words).
    BookUnreadable(vessel_app_view::files::FileFault),
    /// A machine answered a workspace ask.
    Workspace {
        /// The machine.
        machine: String,
        /// The answer, with what was asked.
        answer: workspace::Answer,
    },
    /// A machine answered a search ([`Command::Search`]).
    Searched(search::Answer),
}

/// What an agent on a machine said last time, kept on this device
/// ([`Event::OptionsKept`]): either half may be unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptLists {
    /// The agent.
    pub harness: HarnessRef,
    /// Its options (`harness.options`), when it answered.
    pub options: Option<Vec<SessionOption>>,
    /// Its `/` commands, when a session of it reported them.
    pub commands: Option<Vec<AgentCommand>>,
}
