//! `vessel shell`, `vessel script` and `vessel ports` (protocol 11, ADR
//! 0031): terminals beside sessions, a project's scripts and services, and
//! the ports their processes listen on.

use clap::{Args, Subcommand};
use vessel_proto::EventSeq;

use crate::selector::SessionSelector;

/// `vessel shell`: without a subcommand, open a shell beside a session or
/// in a project and attach to it.
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Shell {
    #[command(subcommand)]
    pub(crate) command: Option<ShellCommand>,
    #[command(flatten)]
    pub(crate) open: OpenShell,
}

/// Where a new shell runs.
#[derive(Debug, Args)]
pub(crate) struct OpenShell {
    /// Beside this session, in its folder.
    #[arg(long, conflicts_with = "project")]
    pub(crate) session: Option<SessionSelector>,
    /// In this project's folder (its id, a unique prefix, or its name).
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// With `--project`: a folder inside it.
    #[arg(long, requires = "project")]
    pub(crate) dir: Option<String>,
    /// What to call it.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Run this terminal profile of the node's (its id) instead of its
    /// shell; its name is the title unless `--title` is given.
    #[arg(long)]
    pub(crate) profile: Option<String>,
    /// Open it and print its id, without attaching.
    #[arg(long)]
    pub(crate) detached: bool,
}

/// `vessel shell …`. A terminal is named by its id or a unique prefix.
#[derive(Debug, Subcommand)]
pub(crate) enum ShellCommand {
    /// List the node's terminals (shells and scripts' runs), live first.
    List {
        /// Only those beside this session.
        #[arg(long, conflicts_with = "project")]
        session: Option<SessionSelector>,
        /// Only those placed in this project.
        #[arg(long)]
        project: Option<String>,
    },
    /// Attach to a terminal: a shell's screen, or a run's output.
    Attach {
        /// The terminal.
        terminal: String,
        /// Replay from this record (0 replays what is kept).
        #[arg(long, default_value = "0", value_parser = crate::args::parse_seq)]
        from: EventSeq,
    },
    /// Rename a terminal.
    Rename {
        /// The terminal.
        terminal: String,
        /// Its new title.
        title: String,
    },
    /// End a terminal's process; it is kept, ended, with its output.
    Kill {
        /// The terminal.
        terminal: String,
    },
    /// End a terminal if it runs, and forget it.
    Close {
        /// The terminal.
        terminal: String,
    },
}

/// A script's checkout: a session's, or a project's folder.
#[derive(Debug, Args)]
pub(crate) struct ScriptPlaceArgs {
    /// The checkout this session runs in (its worktree, or its project).
    #[arg(long, conflicts_with = "project")]
    pub(crate) session: Option<SessionSelector>,
    /// This project's folder.
    #[arg(long)]
    pub(crate) project: Option<String>,
}

/// `vessel script …`: the `[scripts]` and `[services]` of a project's
/// `vessel.toml`, run once a person approved the set
/// (`vessel project approve`).
#[derive(Debug, Subcommand)]
pub(crate) enum ScriptCommand {
    /// List a checkout's scripts and services, their approval and runs.
    List {
        #[command(flatten)]
        place: ScriptPlaceArgs,
    },
    /// Run a script or start a service, and follow its output.
    Run {
        /// Its name in `vessel.toml`.
        script: String,
        #[command(flatten)]
        place: ScriptPlaceArgs,
        /// Start it and print its terminal, without following.
        #[arg(long)]
        detached: bool,
    },
    /// Start a run again: in place while it runs, anew once it ended.
    Restart {
        /// The run's terminal.
        terminal: String,
    },
    /// Stop a run (a service is not started again).
    Stop {
        /// The run's terminal.
        terminal: String,
    },
    /// Print a run's output, following it until it ends.
    Output {
        /// The run's terminal.
        terminal: String,
        /// Replay from this record (0 replays what is kept).
        #[arg(long, default_value = "0", value_parser = crate::args::parse_seq)]
        from: EventSeq,
    },
}

/// `vessel ports`.
#[derive(Debug, Args)]
pub(crate) struct Ports {
    /// Keep printing the list each time it changes, until interrupted.
    #[arg(long)]
    pub(crate) watch: bool,
}
