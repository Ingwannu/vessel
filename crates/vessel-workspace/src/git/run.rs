//! One run of the `git` program: never through a shell, with a time limit,
//! output limits, no prompt, and a slot among a node's few concurrent runs
//! (ADR 0023). This module builds the command and holds what every run
//! gets; running it is `output`'s, its limits are `limits`' and its slot
//! `pool`'s.
//!
//! [Decision Log]
//! - Purpose: every git call of the node goes through here, so every one
//!   gets the same protections and none can forget one.
//! - Constraint: a node serves many clients and must never hang on a git
//!   that waits for a password, pile up git processes, or hold a 2 GB diff
//!   in memory.
//! - Choice: a builder whose only way to run is [`GitCommand::output`]
//!   (or [`GitCommand::output_streaming`] for clone's progress). Arguments
//!   are a vector handed to the program (`vessel-proc`, its own process
//!   group, killed as a group); the environment turns every prompt off
//!   (`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`) so a push that
//!   needs a credential fails instead of waiting; `-c core.fsmonitor=false`
//!   keeps a repository's configuration from starting a monitor program on
//!   a read.
//! - A run that needs no lock it does not take (status refreshes the
//!   index if it may) gets `GIT_OPTIONAL_LOCKS=0`, so reading never makes a
//!   user's own git command fail on `index.lock`.

mod limits;
mod output;
mod pool;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use vessel_proc::CommandSpec;

use pool::{LOCAL_SLOTS, NETWORK_SLOTS, Pool};

pub(crate) use limits::Limits;
pub(crate) use output::GitOutput;

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;

/// How the node runs git: the program, an empty folder to point the hooks
/// at, and variables every run gets.
#[derive(Debug, Clone)]
pub struct GitSetup {
    /// The `git` program, absolute.
    pub program: PathBuf,
    /// An empty folder of the node's: `core.hooksPath` points here, so no
    /// hook runs unless a call asks for the repository's own.
    pub no_hooks: PathBuf,
    /// Variables every run gets besides Vessel's own (tests isolate git from
    /// the machine's configuration this way; a node passes none).
    pub env: Vec<(String, OsString)>,
}

/// The `git` program a node runs, and its two pools.
#[derive(Debug, Clone)]
pub(crate) struct Runner {
    setup: Arc<GitSetup>,
    local: Arc<Pool>,
    network: Arc<Pool>,
}

impl Runner {
    pub(crate) fn new(setup: GitSetup) -> Self {
        Self {
            setup: Arc::new(setup),
            local: Pool::new(LOCAL_SLOTS),
            network: Pool::new(NETWORK_SLOTS),
        }
    }

    /// A command run in `dir`, with [`Limits::LOCAL`] and no hooks.
    pub(crate) fn command(&self, dir: &Path) -> GitCommand {
        GitCommand {
            runner: self.clone(),
            dir: dir.to_path_buf(),
            args: Vec::new(),
            env: Vec::new(),
            limits: Limits::LOCAL,
            network: false,
            hooks: false,
            cancel: None,
        }
    }
}

/// Set on every run.
const ENVIRONMENT: &[(&str, &str)] = &[
    // Never ask on a terminal; a credential helper that needs no person
    // still works. An empty `GIT_ASKPASS` also stops git falling back to
    // `core.askPass` and `SSH_ASKPASS`.
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_ASKPASS", ""),
    ("GCM_INTERACTIVE", "never"),
    ("SSH_ASKPASS_REQUIRE", "never"),
    // Reads never take an optional lock a user's own git would trip on.
    ("GIT_OPTIONAL_LOCKS", "0"),
    // A path is a path: no `*` and no `:(magic)` from a client's text.
    ("GIT_LITERAL_PATHSPECS", "1"),
    // A partial clone does not fetch missing objects on a read.
    ("GIT_NO_LAZY_FETCH", "1"),
    // Messages in English, so refusals can be told apart.
    ("LC_ALL", "C"),
    ("LANGUAGE", "C"),
    // No pager, no editor: nothing here has a terminal.
    ("GIT_PAGER", "cat"),
    ("GIT_EDITOR", "true"),
];

/// Configuration every run overrides, before the subcommand: no program a
/// repository's configuration names runs on a read (ADR 0023).
const OVERRIDES: &[&str] = &[
    "core.fsmonitor=false",
    "core.quotePath=false",
    "diff.external=",
    "log.showSignature=false",
    "color.ui=false",
];

/// A git command being built. Arguments go to the program as a vector,
/// never through a shell.
#[derive(Debug)]
pub(crate) struct GitCommand {
    runner: Runner,
    dir: PathBuf,
    args: Vec<OsString>,
    env: Vec<(String, OsString)>,
    limits: Limits,
    network: bool,
    hooks: bool,
    cancel: Option<CancellationToken>,
}

impl GitCommand {
    /// One argument.
    pub(crate) fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Several arguments.
    pub(crate) fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// An environment variable for this run.
    pub(crate) fn env(mut self, name: &str, value: impl Into<OsString>) -> Self {
        self.env.push((name.to_owned(), value.into()));
        self
    }

    /// Other limits than [`Limits::LOCAL`].
    pub(crate) const fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// A run that talks to a remote: [`Limits::NETWORK`] and a slot of the
    /// network pool.
    pub(crate) const fn network(mut self) -> Self {
        self.network = true;
        self.limits = Limits::NETWORK;
        self
    }

    /// Run the repository's own hooks, as the user's git would (commit and
    /// push only).
    pub(crate) const fn with_hooks(mut self) -> Self {
        self.hooks = true;
        self
    }

    /// Stopped (killed, [`GitError::Cancelled`](super::GitError::Cancelled))
    /// when `token` is.
    pub(crate) fn cancel_on(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// The command as `vessel-proc` runs it.
    fn spec(&self) -> CommandSpec {
        let setup = &self.runner.setup;
        let mut spec = CommandSpec::new(&setup.program).cwd(&self.dir);
        for (name, value) in ENVIRONMENT {
            spec = spec.env(*name, *value);
        }
        for (name, value) in setup.env.iter().chain(&self.env) {
            spec = spec.env(name, value);
        }
        for setting in OVERRIDES {
            spec = spec.args(["-c", setting]);
        }
        if !self.hooks {
            let mut hooks = OsString::from("core.hooksPath=");
            hooks.push(&setup.no_hooks);
            spec = spec.arg("-c").arg(hooks);
        }
        spec.args(&self.args)
    }
}

#[cfg(test)]
mod tests;
