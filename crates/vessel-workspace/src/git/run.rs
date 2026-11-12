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
pub(crate) mod own_config;
mod pool;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use vessel_proc::CommandSpec;

use pool::{CLONE_SLOTS, CLONES_WAITING, LOCAL_SLOTS, MAX_WAITING, NETWORK_SLOTS, Pool};

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
    clones: Arc<Pool>,
}

impl Runner {
    pub(crate) fn new(setup: GitSetup) -> Self {
        Self {
            setup: Arc::new(setup),
            local: Pool::new(LOCAL_SLOTS, MAX_WAITING),
            network: Pool::new(NETWORK_SLOTS, MAX_WAITING),
            clones: Pool::new(CLONE_SLOTS, CLONES_WAITING),
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
            pool: PoolKind::Local,
            hooks: false,
            own_config: true,
            cancel: None,
        }
    }
}

/// Which of the node's pools a run takes a slot of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PoolKind {
    /// Status, diff, commit, a worktree.
    Local,
    /// Fetch, push, pull.
    Network,
    /// A clone (ADR 0030: kept apart, so clones never hold up a push).
    Clone,
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
    ("GIT_SEQUENCE_EDITOR", "true"),
    // ADR 0030: no transport that runs a program a configuration names
    // (`ext::`, `fd::`, a remote helper by `vcs` or `<helper>::`), even
    // where the repository's own configuration allows it. A local path
    // stays usable: its `uploadpack` and `receivepack`, set by the
    // repository, refuse the run (`own_config`). A clone narrows this
    // further (no `file`).
    ("GIT_ALLOW_PROTOCOL", "file:git:http:https:ssh"),
];

/// Configuration every run overrides, before the subcommand: no program a
/// repository's configuration names runs on a read (ADR 0023).
const OVERRIDES: &[&str] = &[
    "core.fsmonitor=false",
    "core.quotePath=false",
    "diff.external=",
    "log.showSignature=false",
    "color.ui=false",
    // ADR 0030: never into a submodule, whose own configuration the
    // listing of this repository's does not cover.
    "submodule.recurse=false",
    "fetch.recurseSubmodules=false",
    // ADR 0030: no signature checked or made, so no signing program runs,
    // except a commit's own (`commit.gpgSign` is left to the user's
    // configuration only on a run with hooks, `spec_of`).
    "merge.verifySignatures=false",
    "tag.gpgSign=false",
    "push.gpgSign=false",
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
    pool: PoolKind,
    hooks: bool,
    /// Whether the repository's own configuration is listed and judged
    /// before the run (ADR 0030): every run in a repository; not the
    /// version probe, a clone or an init, which run in no repository.
    own_config: bool,
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
        self.pool = PoolKind::Network;
        self.limits = Limits::NETWORK;
        self
    }

    /// A clone: [`Limits::CLONE`] and a slot of the clones' own pool. It
    /// runs in no repository, so no repository's configuration is read
    /// first.
    pub(crate) const fn clone_run(mut self) -> Self {
        self.pool = PoolKind::Clone;
        self.limits = Limits::CLONE;
        self.own_config = false;
        self
    }

    /// A run in no repository (the version probe, an init): no
    /// repository's configuration is read first.
    pub(crate) const fn outside_repository(mut self) -> Self {
        self.own_config = false;
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

    /// What this run does that a repository's configuration could name a
    /// program for (ADR 0030).
    const fn uses(&self) -> own_config::Uses {
        own_config::Uses {
            network: matches!(self.pool, PoolKind::Network),
            hooks: self.hooks,
        }
    }

    /// The command as `vessel-proc` runs it, with `settings` (what the
    /// repository's own configuration made this run turn off) after the
    /// fixed overrides.
    fn spec(&self, settings: &[String]) -> CommandSpec {
        self.spec_of(&self.args, settings)
    }

    /// A command with every run's protections and `args`, in this run's
    /// folder: the run itself, or the listing of its repository's own
    /// configuration before it.
    fn spec_of(&self, args: &[OsString], settings: &[String]) -> CommandSpec {
        let setup = &self.runner.setup;
        let mut spec = CommandSpec::new(&setup.program)
            .cwd(&self.dir)
            .arg("--no-pager");
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
            // A snapshot's `commit-tree` or a stash signs when asked to;
            // only a person's commit does (ADR 0030).
            spec = spec.args(["-c", "commit.gpgSign=false"]);
        }
        for setting in settings {
            spec = spec.args(["-c", setting.as_str()]);
        }
        spec.args(args)
    }
}

#[cfg(test)]
mod tests;
