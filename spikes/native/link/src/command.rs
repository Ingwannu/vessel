//! What a screen asks of the link ([`Command`]), moved unchanged out of the crate root.
//!
//! [Decision Log]
//! - Purpose: every package registers its commands and events here, so the
//!   crate root stays under the 500-line budget and leaves the size list.
//! - Constraint: the type, its variants and their documentation are exactly
//!   what they were in `lib.rs`; the crate root re-exports it, so every
//!   caller keeps `vessel_app_link::Command`.
//! - Alternatives: raise the root's recorded size for each package (the
//!   list only shrinks), or each package extracting its own piece.
//! - Choice: one shared move before the packages start.
//! - Cost: one more file; no API, behaviour or dependency change.

use super::{
    Act, Offer, Prefs, agents, attachments, devices, machines, node_settings, search, workspace,
};
use vessel_proto::Delivery;
use vessel_proto::{
    AgentCommand, HarnessRef, OptionChoice, PermissionId, PermissionOptionId, SessionId,
};

/// What a screen asks of the link.
#[derive(Debug)]
pub enum Command {
    /// Directory metadata intent bound to its original rendered admission.
    SessionMark(crate::session_marks::Ask),
    /// Scoped own-history discovery and import.
    History(crate::history::Ask),
    /// A request for one machine’s Agents settings.
    Agents {
        /// Validated book key.
        machine: agents::MachineKey,
        /// Correlation ticket.
        request: vessel_proto::RequestId,
        /// The operation.
        ask: agents::Ask,
    },

    /// Read or edit the saved machine book.
    Machines(machines::Ask),
    /// Original-context readonly root, summary or diff acquisition.
    CodeSource(workspace::code_source::Request),
    /// Read the book again and connect to every node in it.
    Reload,
    /// Reconcile a known closed admission on the same actor as Fleet snapshots.
    /// An obsolete tag cannot retire a replacement, and carries no `NodeId` map.
    RetireConnection(vessel_app_view::connection::ConnectionTag),
    /// Ask about a machine's paired devices.
    Devices {
        /// The authenticated admission that supplied the page’s facts.
        context: vessel_app_view::devices::ConnectionContext,
        /// The page request identity.
        ticket: vessel_app_view::devices::Ticket,
        /// The device operation.
        ask: devices::Ask,
    },
    /// Read or change a machine's own settings.
    NodeSettings(node_settings::Ask),
    /// Read-only sharing status for one selected machine.
    Sync {
        /// Its parsed node-book identity.
        machine: vessel_app_view::devices::MachineKey,
        /// Correlation identity reserved by the page.
        ticket: vessel_app_view::devices::Ticket,
    },
    /// Show a session's conversation: its records from the start, then as
    /// they arrive. Replaces the session followed before.
    Open {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
    },
    /// Send a prompt to a session (queued when a turn runs).
    Prompt {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What was typed.
        text: String,
        /// What it does while a turn runs (Settings' default send).
        delivery: Delivery,
    },
    /// Start a conversation: create the session, send its first prompt,
    /// and report it [`Event::Started`].
    Start {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
        /// The folder on the machine; `None` for its home.
        folder: Option<String>,
        /// The first prompt.
        prompt: String,
        /// The agent's options to start with (only those that change
        /// something, in the agent's order).
        options: Vec<OptionChoice>,
    },
    /// Ask what `harness` offers on `machine` before any session of it
    /// (`harness.options`); answered [`Event::Options`]. Takes seconds the
    /// first time (the node starts the agent to ask it), so it runs beside
    /// the other commands.
    Options {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
    },
    /// Keep the composers' drafts on this device (written on the blocking
    /// pool; nothing is answered). A browser keeps none.
    SaveDrafts(vessel_app_view::composer::drafts::Drafts),
    /// What a composer's files ask (track P's row c): a file sent, a
    /// conversation started for them, the prompt that names them.
    Files(attachments::Files),
    /// Keep `harness`'s `/` commands, as a session of it on `machine`
    /// reported them, on this device for the next run (its options' file;
    /// nothing is answered). A browser keeps none.
    KeepCommands {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
        /// Its commands, in its order.
        commands: Vec<AgentCommand>,
    },
    /// Start an ended or interrupted session again with its conversation
    /// (`session.resume`, protocol 10), then send it `text`; answered
    /// [`Event::Resumed`] or [`Event::ResumeFailed`].
    Resume {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What was typed.
        text: String,
    },
    /// Stop the running turn.
    Stop {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
    },
    /// Answer a permission the agent asked for.
    Answer {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// The request.
        id: PermissionId,
        /// The answer chosen.
        option: PermissionOptionId,
    },
    /// Remember the person's choices (machine, agent, folders).
    // Prefs grew past large_enum_variant; a save is infrequent.
    SavePrefs(Box<Prefs>),
    /// Act on a session: rename, mark, start again, delete, kill, move,
    /// terminal input (answered by `Event::Acted`).
    Act {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What to do.
        act: Act,
    },
    /// Pair with a node from a pairing link, as `name` in the book.
    Pair {
        /// The decoded link.
        offer: Offer,
        /// The machine's name in the book.
        name: String,
        /// The person left the name empty (`name` is then a free name the
        /// app chose): use the node's own name instead when it makes a free
        /// book name.
        prefer_node_name: bool,
    },
    /// Ask a machine about its projects, worktrees, git or files; answered
    /// by [`Event::Workspace`] (`workspace`'s module says how).
    Workspace {
        /// The machine.
        machine: String,
        /// What is asked.
        ask: workspace::Ask,
    },
    /// Search one machine's messages (`session.search`); answered by
    /// [`Event::Searched`].
    Search(search::Ask),
}
