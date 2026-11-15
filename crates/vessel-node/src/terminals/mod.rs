//! Terminals beside sessions as the node serves them (protocol 11, ADR
//! 0031): where one may be opened and by whom, how many, the environment
//! and the shell, scripts' runs under the approval of ADR 0027, and the
//! ports their trees listen on. The terminals themselves are
//! `vessel_session::terminals`'.
//!
//! [Decision Log]
//! - Purpose: the session runtime runs a terminal and keeps its log; the
//!   node decides who may open one where, keeps the registry the calls
//!   read, and ties terminals to the sessions and projects they are in
//!   (ADR 0001's split, as `crate::workspace` does for worktrees).
//! - State: one map of handles behind a `std` lock held only to read or
//!   change it, never across an `.await`; each terminal's own state is its
//!   actor's. A terminal being started holds a reservation in the same
//!   lock, so starts at once cannot all pass a limit (track B's lesson,
//!   review item 8).
//! - Kept in memory only: a terminal ends with the node, and its output is
//!   never written to disk (ADR 0031).
//! - Limits: 64 live a node, 8 a session, 16 a project folder, 4 an
//!   agent's; 32 ended kept, the oldest forgotten first.

mod env;
// Staged until a fresh owned route drives health and publishes checked results.
#[cfg(test)]
mod health;
mod ports;
// Staged until the service plan owns reservations and approval-checked starts.
#[cfg(test)]
mod port_allocator;
// Staged until the approval-checked service allocator connects one invocation.
#[cfg(test)]
mod port_script;
// Staged until canonical ownership/peer/scheme admission and browser wiring qualify it.
#[cfg(test)]
pub(crate) mod preview;
mod registry;
mod scripts;
mod subscribe;

use std::ffi::OsString;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_session::terminals::Scrollback;

pub use env::{KNOWN_CREDENTIALS, Shell, credentials_in};
pub(crate) use env::{scrubbed, shell_spec};
pub(crate) use ports::Ports;
pub(crate) use registry::Terminals;
pub(crate) use registry::{Entry, Owner, Start, TerminalsError};
pub(crate) use scripts::{CLOSE_WAIT, Plan};
pub(crate) use subscribe::open as subscribe;

/// The most live terminals (shells and runs) a node keeps.
pub(crate) const MAX_LIVE: usize = 64;
/// The most live terminals beside one session.
pub(crate) const MAX_PER_SESSION: usize = 8;
/// The most live terminals placed in one project.
pub(crate) const MAX_PER_PROJECT: usize = 16;
/// The most live terminals one agent opened.
pub(crate) const MAX_PER_AGENT: usize = 4;
/// The most ended terminals kept.
pub(crate) const MAX_ENDED: usize = 32;
/// The longest a long poll (`terminal.get`, `terminal.read`, `port.list`)
/// waits.
pub(crate) const MAX_WAIT_SECS: u32 = 30;

/// How the node's terminals are set up.
#[derive(Debug, Clone)]
pub struct TerminalsConfig {
    /// The shell, found by the binary at start; without one,
    /// `terminal.create` is `unsupported`.
    pub shell: Option<Shell>,
    /// Removed from the environment of every terminal, script and service,
    /// and of the setup and teardown commands of a project's file:
    /// Vessel's credentials (`credentials_in` over the binary's own
    /// environment; the known names by default) and the forge's sign-in
    /// (`vessel_forge::SIGN_IN_VARIABLES`, ADR 0029).
    pub removed: Vec<OsString>,
    /// How much of a terminal's log is kept.
    pub scrollback: Scrollback,
}

impl TerminalsConfig {
    /// What a binary sets at start: the shell (`Shell::detect` on this
    /// platform, `on_path` finding `pwsh.exe` and `powershell.exe`) and the
    /// credentials among `names`, its environment's variable names. The
    /// binary passes its own environment: the library reads none (AGENTS.md
    /// section 8).
    #[must_use]
    pub fn detect(
        var: impl Fn(&str) -> Option<OsString>,
        names: impl IntoIterator<Item = OsString>,
        on_path: impl Fn(&str) -> Option<std::path::PathBuf>,
    ) -> Self {
        let shell = Shell::detect(
            cfg!(windows),
            cfg!(target_os = "macos"),
            var,
            vessel_proc::account_shell,
            on_path,
            env::executable,
        );
        let mut removed = credentials_in(names);
        // The known names too: a variable the binary did not have is
        // harmless to remove, and one set later (a shell's profile) is not
        // the node's.
        removed.extend(always_removed());
        removed.sort();
        removed.dedup();
        Self {
            shell: Some(shell),
            removed,
            scrollback: Scrollback::DEFAULT,
        }
    }
}

impl Default for TerminalsConfig {
    fn default() -> Self {
        Self {
            shell: None,
            removed: always_removed().collect(),
            scrollback: Scrollback::DEFAULT,
        }
    }
}

/// Removed whatever the binary's environment holds: Vessel's credentials
/// by name, and the variables the forge's sign-in is read from. A token
/// the node reads there is its sign-in (ADR 0029), and what the node
/// starts in a repository's folder (a shell an agent may type into, a
/// repository's script) must not hold it; a person's `gh` in a terminal
/// still finds its own stored login.
fn always_removed() -> impl Iterator<Item = OsString> {
    KNOWN_CREDENTIALS
        .iter()
        .chain(vessel_forge::SIGN_IN_VARIABLES.iter())
        .map(OsString::from)
}

/// What a terminal needs from the node to run.
#[derive(Debug, Clone)]
pub(crate) struct Runner {
    pub(crate) config: Arc<TerminalsConfig>,
    pub(crate) shutdown: CancellationToken,
    pub(crate) tasks: TaskTracker,
}
