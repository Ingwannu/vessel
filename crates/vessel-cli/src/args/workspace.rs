//! `vessel project` and `vessel worktree` (protocol 10): the folders a
//! node works in, and worktrees of them.

use std::path::PathBuf;

use clap::Subcommand;

/// `vessel project …`. A project is named by its id, a unique prefix of
/// one, or its name.
#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    /// Register a directory, defaulting to this directory for a local target.
    Create { path: Option<PathBuf> },
    /// Set/reset a display name through the workspace registry contract.
    Rename {
        project: String,
        name: Option<vessel_proto::workspace::ProjectName>,
        #[arg(long, conflicts_with = "name", required_unless_present = "name")]
        reset: bool,
    },
    /// Archive owned workspaces before removing the project registration.
    Delete { project: String },
    /// List the node's projects and the git it found.
    #[command(alias = "ls")]
    List,
    /// Add a folder on the node (inside a repository: its top level).
    Add {
        /// An absolute folder on the node.
        path: PathBuf,
        /// What to call it (default: the folder's name).
        #[arg(long)]
        name: Option<String>,
    },
    /// Clone a repository into a folder on the node, following its
    /// progress.
    Clone {
        /// `https://`, `ssh://`, `git://` or `user@host:path`.
        url: String,
        /// The absolute folder to clone into (it gets the repository's
        /// folder).
        parent: PathBuf,
        /// The new folder's name (default: the repository's).
        #[arg(long)]
        folder: Option<String>,
        /// Return once the clone started.
        #[arg(long)]
        no_wait: bool,
    },
    /// Start a new repository in an empty or new folder on the node.
    Init {
        /// An absolute folder on the node.
        path: PathBuf,
    },
    /// Forget a project; its files stay.
    Remove {
        /// The project.
        project: String,
    },
    /// Show what the project's `vessel.toml` would run in a new worktree,
    /// and the digest to approve it by.
    Commands {
        /// The project.
        project: String,
    },
    /// Approve the set of commands whose digest `commands` printed: it
    /// runs in new worktrees until the file changes.
    Approve {
        /// The project.
        project: String,
        /// The digest `vessel project commands` printed.
        digest: String,
        /// Withdraw the approval instead.
        #[arg(long)]
        withdraw: bool,
    },
}

/// `vessel worktree …`. A worktree is named by its id or a unique prefix.
#[derive(Debug, Subcommand)]
pub(crate) enum WorktreeCommand {
    /// List this repository's worktrees through canonical workspace ownership.
    #[command(hide = true)]
    Ls {
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    /// Create from a repository path through the canonical workspace registry.
    Create(super::parity::WorktreeCreate),
    /// Archive a named owned worktree through the canonical workspace registry.
    Archive {
        name: String,
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    /// List worktrees, or a project's.
    List {
        #[arg(long, conflicts_with = "project")]
        cwd: Option<PathBuf>,
        /// Only this project's.
        #[arg(long)]
        project: Option<String>,
    },
    /// Make a worktree and follow its setup.
    New {
        /// The project.
        project: String,
        /// The new branch's name (default: vessel/<8 hex>), or with
        /// `--existing` the branch to check out.
        #[arg(long)]
        branch: Option<String>,
        /// Check out `--branch` as it is.
        #[arg(long, requires = "branch", conflicts_with = "base")]
        existing: bool,
        /// Where the new branch starts (default: the project's default
        /// branch).
        #[arg(long)]
        base: Option<String>,
        /// Do not run the project's setup commands.
        #[arg(long)]
        skip_setup: bool,
        /// Return without following setup.
        #[arg(long)]
        no_wait: bool,
    },
    /// Print a worktree's setup output (`--follow` until setup ends).
    Output {
        /// The worktree.
        worktree: String,
        /// Keep printing until setup ends.
        #[arg(long)]
        follow: bool,
    },
    /// Run setup again, with the commands of the worktree's file as it is.
    Setup {
        /// The worktree.
        worktree: String,
    },
    /// Remove a worktree; one with work in it is kept unless told.
    Remove {
        /// The worktree.
        worktree: String,
        /// Keep a snapshot of its work first.
        #[arg(long, conflicts_with = "discard")]
        snapshot: bool,
        /// Remove it with its work.
        #[arg(long)]
        discard: bool,
    },
    /// List a project's snapshots.
    Snapshots {
        /// The project.
        project: String,
    },
}

impl WorktreeCommand {
    pub(crate) fn require_contract(&self) -> Result<(), crate::error::CliError> {
        use crate::error::CliError;
        match self {
            Self::Create(args) => {
                args.validate()?;
                Err(CliError::ParityBlocked(
                    "path-based worktree creation awaits W11's reviewed workspace source/registry contract",
                ))
            }
            Self::Ls { .. } | Self::Archive { .. } | Self::List { cwd: Some(_), .. } => {
                Err(CliError::ParityBlocked(
                    "repository-scoped worktree list/archive awaits W11's reviewed workspace ownership contract",
                ))
            }
            Self::List { cwd: None, .. }
            | Self::New { .. }
            | Self::Output { .. }
            | Self::Setup { .. }
            | Self::Remove { .. }
            | Self::Snapshots { .. } => Ok(()),
        }
    }
}
