//! Process primitives: what to run, and how to stop all of it.
//!
//! This crate owns these OS mechanisms and nothing else:
//!
//! - [`CommandSpec`]: a plain-data description of a command (program, args,
//!   environment changes, working directory). `vessel-harness` builds them;
//!   `vessel-pty` and [`spawn`] run them. It carries no harness knowledge.
//! - [`spawn`]: start a command with piped stdio as the leader of a new
//!   process group (a new job object on Windows), returning a [`Child`]
//!   that stops the whole group.
//! - [`ProcessGroup`] (Unix): signal every process in a group, for runners
//!   such as `vessel-pty` that spawn through another library.
//! - What the trees it started still hold (protocol 11, ADR 0031): the TCP
//!   ports they listen on ([`listening`]), and the processes left in a
//!   terminal's session once its shell has exited ([`kill_session`]); and
//!   the account's login shell a terminal falls back to
//!   ([`account_shell`]).
//!
//! - Generic selected-file kernel observations ([`lock`], ADR 0001), without
//!   node data-directory layouts or authority to signal a snapshot's PID.
//!
//! It must not know about sessions, harnesses or the wire protocol, so it
//! depends on nothing from Vessel.
//!
//! [Decision Log]
//! - Purpose: a harness starts its own children (MCP servers, shells,
//!   language servers). Killing only the harness leaves those running and,
//!   worse, holding the session's output pipe or PTY open forever.
//! - Alternatives: `tokio`'s `kill_on_drop` (kills only the direct child);
//!   Linux-only subreapers or cgroups (not portable, need privileges).
//! - Choice: every child is the leader of its own process group, and stopping
//!   it signals the group (`killpg`). Windows has no such groups; there each
//!   child starts suspended in its own kill-on-close job object (through
//!   `process-wrap`, which keeps the Win32 `unsafe` out of this workspace),
//!   and stopping it terminates the job. Before that (until 2026-11-06)
//!   Windows terminated only the direct child, which for an npm `.cmd` shim
//!   is `cmd.exe`: the real adapter kept running and holding the pipes.
//!   Details in `child.rs`.
//! - Constraint: after the leader is reaped its group id can only be reused
//!   once every member is gone, so sweeping the group right after the leader
//!   exits reaches exactly its leftovers. The one remaining race (the group
//!   empties and a brand-new process takes the same id as a group leader
//!   within microseconds) is accepted; closing it needs pidfds or a
//!   subreaper.

#[cfg(not(any(unix, windows)))]
compile_error!("vessel-proc supports Unix process groups and Windows job objects only");

mod account;
mod child;
mod error;
#[cfg(unix)]
mod group;
mod lifetime;
pub mod listening;
pub mod lock;
mod process_id;
mod session;
mod signal;
mod spec;

pub use account::account_shell;
pub use child::{Child, Spawned, spawn};
pub use error::{InvalidGroup, SignalError, SpawnError, Stream, WaitError};
#[cfg(unix)]
pub use group::{ProcessGroup, REAP_GRACE};
pub use lifetime::{ProcessLifetime, ProcessLifetimeGuard};
pub use process_id::{InvalidProcessId, ProcessId};
pub use session::kill_session;
pub use signal::Signal;
pub use spec::{CommandSpec, EnvValue, Environment};
