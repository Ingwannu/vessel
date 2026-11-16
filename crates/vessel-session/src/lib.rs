//! The session runtime the node uses.
//!
//! Owns: one actor task per session that owns its process (a PTY program
//! from `vessel-pty`, or an ACP agent from `vessel-agent`), numbers every event (`EventSeq`, gap-free
//! from zero), keeps a bounded in-memory ring of recent records, hands every
//! record to the node through a bounded durable channel, and broadcasts it
//! to live followers. It also keeps each session's place on the board
//! (`SessionInfo::board`, protocol 8, ADR 0019): [`board`] is the reducer,
//! a port of agent-orchestrator's, and the actor feeds it what its agent is
//! doing. Must not: persist anything (the node owns the store, ADR 0001),
//! speak the RPC protocol, or know harness specifics (it asks
//! `vessel-harness`).
//!
//! This API is the seam with the node (`docs/plans/backend-m1.md`, R6).
//!
//! ```text
//!             SessionHandle (cheap clone, many holders)
//!              | send(input) ----------------------> PTY input (bounded)
//!              | kill() -----> CancellationToken --> actor
//!              | follow(from) -> ring snapshot + broadcast receiver
//!              | info()/watch_info() <- watch <---- actor
//!
//!   actor (owns the process): output -> seq -> durable channel (node)
//!                                           -> ring + broadcast (followers)
//!                                           -> activity -> board::derive
//!                                              -> info.board (watch)
//! ```
//!
//! [Decision Log]
//! - Purpose: many clients follow one session, attach late, and resume by
//!   sequence number; the node persists everything; a slow or crashed
//!   client must never lose or reorder records for others.
//! - Alternatives: Phase 0's shared `Arc<Session>` with mutexes around the
//!   child, backlog and stdin (any holder could mutate anything, and the
//!   backlog was the only history); one channel per subscriber fed by the
//!   actor (the actor would have to manage subscribers and their overflow).
//! - Choice: the actor alone mutates session state. Followers take a ring
//!   snapshot and a `broadcast` receiver under one short lock, so nothing
//!   falls between the two; a follower that lags behind the broadcast
//!   buffer gets `Lagged` from tokio and resumes from the store through the
//!   node. The durable channel is the only path that may slow the session:
//!   records are never dropped for the store, so when the node stops
//!   draining, the actor stops reading output, and the program stalls (a
//!   structured session's agent through `vessel-agent`'s gated transport;
//!   ADR 0003 lists the little the ACP SDK still reads regardless).
//! - Sharing: each record is allocated once as an `Arc<EventRecord>`; the
//!   durable channel, the ring and every follower hold that same `Arc`, so
//!   fan-out costs a reference count per reader, not a copy.
//! - Order per record: number it, wait for a durable slot, then append to
//!   the ring and broadcast in one step. A follower therefore never sees a
//!   record twice (snapshot and live are disjoint) and never sees a record
//!   the store will not get.
//! - Stopping: `kill` asks politely (`SIGHUP` for a PTY, `SIGTERM` for an
//!   ACP agent), then kills the whole process group after a grace period.
//!   The actor keeps reacting to `kill` even while it waits for a durable
//!   slot.
//! - Structured status: `Starting` until the agent's `Ready` event, then
//!   `Running`; a session that never became ready ends `Failed` with the
//!   reason, unless it was killed while starting: that ends `Exited`, as a
//!   killed terminal session does.
//! - The board: every record the actor commits also passes the session's
//!   activity tracker (`activity`), so what the agent is doing is a
//!   projection of its log, as the store's `sessions` row is: a turn
//!   started is active, a permission asked is waiting on the owner, a turn
//!   ended is idle, a cancelled one is interrupted until the next, any
//!   final status is exited (in building as `exited`, ADR 0026; an archived
//!   session is in the archive). A terminal has no turns: output is active,
//!   and one resettable timer in the actor's loop turns it idle after a
//!   quiet stretch (`terminal::QUIET`). [`board::derive()`] turns those facts
//!   into a lane and a phrase. The info watch wakes only when the lane,
//!   the phrase or the activity changes; the time of the last activity
//!   moves silently, as `next_seq` does, so output never wakes watchers.
//! - What a session runs with (protocol 9, ADR 0022): the agent's option
//!   reports become `options_changed` records (one before `Ready`, then
//!   every change, also from an agent a move restarts); the publisher keeps
//!   the last one for [`SessionHandle::choices`], which a move or an export
//!   carries on so the session keeps its permissions mode. Options are read
//!   and changed through the agent's input, as prompts are.
//! - Titles (protocol 9): `SessionInfo::title` is the one truth. An
//!   untitled structured session takes the first line of its first prompt
//!   with text; [`SessionHandle::rename`] sends the actor a rename, which it
//!   publishes as `renamed` in order with the log and answers once the info
//!   has it. Only the actor publishes, so a title can never disagree with
//!   the log the store rebuilds it from.
//! - Marks (protocol 10, ADR 0026, `marks`): the node applies a mark in the
//!   store and hands the store's result to [`SessionHandle::set_marks`];
//!   the publisher notes each record that asks for attention.
//!   [`SessionHandle::info`] adds both to the info, so a session the
//!   runtime has carries the marks the store would give it.
//! - Pull requests (protocol 11, ADR 0029, `forge`): the node reads a
//!   session's pull request from its forge and hands its view to
//!   [`SessionHandle::set_pull_request`], a `watch` the actor reads in its
//!   loop (the latest wins; nothing queues). The tracker keeps the facts,
//!   so the one reducer that places the session places it by its pull
//!   request too; the summary goes in `SessionInfo::pull_request`. Both
//!   are dropped at the final status: nothing watches an ended session.
//! - Attachments (protocol 10, ADR 0025, `attachments`): a structured
//!   session checks a prompt's attachments before it queues or sends it
//!   and, right before it sends it, asks the node's loader
//!   ([`AttachmentLoader`], given with [`SessionRuntime::with_attachments`])
//!   for their bodies, awaiting the answer as it awaits the agent, so it
//!   never blocks on file I/O; the log keeps the references only.
//! - Starting again (protocol 10, ADR 0026, `again`):
//!   [`SessionRuntime::start_again`] runs an ended structured session's
//!   agent again with `session/load`, as a profile move does, its log
//!   continuing at its stored `next_seq`; the session publishes nothing
//!   until the node, having reopened its row in the store, opens its
//!   [`Gate`].
//! - Lifetime: an ended session stays in the runtime, ring and all, until
//!   its owner calls `SessionRuntime::remove` (the node does once the store
//!   has its whole log). Eviction is the owner's call because only the
//!   owner knows when the store is complete; a runtime-side timer would
//!   either drop rings the store still lacks or keep them for nothing.

mod activity;
mod again;
mod attachments;
pub mod board;
mod config;
mod conversation;
mod delivery;
mod error;
mod forge;
mod forking;
mod handle;
mod lifecycle;
mod limits;
pub use limits::LimitAssociation;
mod marks;
mod moving;
mod parking;
mod publish;
mod queue;
mod ring;
mod runtime;
mod side_questions;
mod stop;
mod structured;
mod terminal;
mod terminal_screen;
pub mod terminals;

pub use again::{Ended, Gate, NotResumable, Relaunch, Reopening, resumable};
pub use attachments::{AttachmentError, AttachmentLoader, LoadAttachments};
pub use config::{DEFAULT_DURABLE_CAPACITY, DEFAULT_RING_CAPACITY, RuntimeConfig};
pub use error::{
    DeliveryError, InputKind, ModeKind, ParkRefusal, RemoveError, RewindRefusal, SessionError,
};
pub use forge::ForgeView;
pub use handle::{SessionHandle, TitleUpdate};
pub use moving::{MoveError, ProfileMove, Restart};
pub use queue::QueueError;
pub use ring::{Follow, FollowError};
pub use runtime::{Created, ForkStart, LaunchAdmission, LaunchOptions, SessionRuntime};
/// Suggested [`RuntimeConfig::agent_handshake_timeout`].
pub use vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT;
/// What `SessionRuntime::probe` learned (re-exported so the node needs no
/// edge to `vessel-agent`).
pub use vessel_agent::Probed;
/// What a loader answers with (`LoadAttachments`), re-exported for the
/// same reason.
pub use vessel_agent::{AttachmentBody, Attachments};
