//! `vessel schedule` and `vessel prompt` (protocol 11, ADR 0033):
//! schedules and saved prompts. Schedules and saved prompts are named by
//! their id, a unique prefix of one, or their name.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use vessel_proto::{HarnessKind, ProfileName};

/// `vessel schedule …`.
#[derive(Debug, Subcommand)]
pub(crate) enum ScheduleCommand {
    /// List the node's schedules, newest first.
    List,
    /// Compute upcoming runs on the node without saving a schedule.
    Preview {
        /// Five-field cron expression.
        #[arg(long, conflicts_with = "every", required_unless_present = "every")]
        cron: Option<String>,
        /// Elapsed interval (for example 30m).
        #[arg(long)]
        every: Option<String>,
        /// IANA zone; defaults to this CLI machine's zone, as create does.
        #[arg(long, requires = "cron")]
        tz: Option<String>,
        /// Start strictly after this RFC 3339 instant; defaults to node time.
        #[arg(long)]
        after: Option<DateTime<Utc>>,
        /// Number of upcoming runs, from 1 to 10.
        #[arg(long, default_value_t = 5)]
        count: u32,
    },
    /// Show one schedule and its last run.
    Show {
        /// The schedule.
        schedule: String,
    },
    /// A schedule's newest runs (at most 50), newest first.
    Runs {
        /// The schedule.
        schedule: String,
    },
    /// Make a schedule: when, what it prompts, and with what.
    Create(Box<ScheduleArgs>),
    /// Change a schedule: what is given replaces what it had; the rest
    /// stays. Whoever changes it is whom its runs act as from then on.
    Edit {
        /// The schedule.
        schedule: String,
        #[command(flatten)]
        change: Box<ScheduleArgs>,
    },
    /// Stop a schedule's runs until `resume`.
    Pause {
        /// The schedule.
        schedule: String,
    },
    /// Let a paused schedule run again, from now.
    Resume {
        /// The schedule.
        schedule: String,
    },
    /// Run a schedule once now and print what the run came to.
    Run {
        /// The schedule.
        schedule: String,
    },
    /// Remove a schedule and its runs; a session it started is not
    /// touched.
    Delete {
        /// The schedule.
        schedule: String,
    },
}

/// What a schedule is: when, what, and with what. For `create`, `--every`
/// or `--cron`, a target and a prompt are needed; for `edit`, only what
/// changes.
#[derive(Debug, Args)]
pub(crate) struct ScheduleArgs {
    /// What to call it.
    #[arg(long)]
    pub(crate) name: Option<String>,
    /// Run every so long: `90s`, `30m`, `2h` or `1d` (at least a minute).
    #[arg(long, conflicts_with = "cron")]
    pub(crate) every: Option<String>,
    /// Run at the times of a cron expression (five fields, or `@daily`
    /// and its kind), in `--tz`.
    #[arg(long)]
    pub(crate) cron: Option<String>,
    /// The time zone of `--cron` (an IANA name, such as Europe/London);
    /// the one this machine is in when left out.
    #[arg(long, requires = "cron")]
    pub(crate) tz: Option<String>,
    /// Prompt this session (through its queue) each run.
    #[arg(long, conflicts_with_all = ["harness", "profile", "project", "worktree", "model", "keep"])]
    pub(crate) session: Option<String>,
    /// Start a new session of this harness each run.
    #[arg(long)]
    pub(crate) harness: Option<HarnessKind>,
    /// With `--harness`: under this profile (`vessel profiles`).
    #[arg(long, requires = "harness")]
    pub(crate) profile: Option<ProfileName>,
    /// With `--harness`: in this project's folder (`vessel project list`).
    #[arg(long, requires = "harness", conflicts_with = "worktree")]
    pub(crate) project: Option<String>,
    /// With `--harness`: in this worktree (`vessel worktree list`).
    #[arg(long, requires = "harness")]
    pub(crate) worktree: Option<String>,
    /// With `--harness`: the model to ask for.
    #[arg(long, requires = "harness")]
    pub(crate) model: Option<String>,
    /// With `--harness`: keep each run's session after its first turn
    /// (by default it is archived, unless something waits in its queue).
    #[arg(long, requires = "harness")]
    pub(crate) keep: bool,
    /// The prompt each run sends.
    #[arg(long, conflicts_with = "prompt_file")]
    pub(crate) prompt: Option<String>,
    /// The prompt each run sends, read from a file on this machine.
    #[arg(long)]
    pub(crate) prompt_file: Option<PathBuf>,
    /// End after this many runs.
    #[arg(long)]
    pub(crate) max_runs: Option<u32>,
    /// End at this time (RFC 3339, such as 2026-12-31T18:00:00Z).
    #[arg(long)]
    pub(crate) ends_at: Option<DateTime<Utc>>,
    /// Due times missed while the node was stopped: `once` (one run for
    /// the latest, if less than a day old) or `skip`.
    #[arg(long)]
    pub(crate) catch_up: Option<String>,
    /// Make it paused (`create` only).
    #[arg(long)]
    pub(crate) paused: bool,
}

/// `vessel prompt …`.
#[derive(Debug, Subcommand)]
pub(crate) enum PromptCommand {
    /// List the saved prompts: every one, or a project's and the
    /// machine's.
    List {
        /// Only this project's and the machine's.
        #[arg(long)]
        project: Option<String>,
    },
    /// Print a saved prompt's text.
    Show {
        /// The saved prompt.
        prompt: String,
    },
    /// Save a prompt under a name, or replace the text of the one that
    /// has it.
    Save {
        /// Its name.
        name: String,
        /// The text.
        #[arg(long, conflicts_with = "file", required_unless_present = "file")]
        text: Option<String>,
        /// The text, read from a file on this machine.
        #[arg(long)]
        file: Option<PathBuf>,
        /// The project it belongs to; the machine's when left out.
        #[arg(long)]
        project: Option<String>,
    },
    /// Remove a saved prompt.
    Delete {
        /// The saved prompt.
        prompt: String,
    },
    /// Send a saved prompt: into a session's queue, or to a new session
    /// started with what the prompt says (in its project).
    Run {
        /// The saved prompt.
        prompt: String,
        /// Queue it into this session.
        #[arg(long, conflicts_with = "new", required_unless_present = "new")]
        session: Option<String>,
        /// Start a session with it and print its id.
        #[arg(long)]
        new: bool,
    },
}
