//! Workspace invocation grammar stays distinct from legacy project/worktree identities.
use crate::error::CliError;
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;
use vessel_proto::SessionTitle;
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Isolation {
    Local,
    Worktree,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum WorktreeMode {
    BranchOff,
    CheckoutBranch,
    CheckoutPr,
}
#[derive(Debug, Subcommand)]
pub(crate) enum WorkspaceCommand {
    Setup {
        id: String,
    },
    Create(Box<CreateWorkspace>),
    Ls {
        #[arg(long)]
        background: bool,
    },
    Rename {
        id: String,
        title: Option<SessionTitle>,
        #[arg(long, conflicts_with = "title", required_unless_present = "title")]
        reset: bool,
    },
    Archive {
        id: String,
    },
}
#[derive(Debug, Args)]
pub(crate) struct CreateWorkspace {
    #[arg(long)]
    isolation: Isolation,
    #[arg(long)]
    path: Option<PathBuf>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    title: Option<SessionTitle>,
    #[arg(long)]
    mode: Option<WorktreeMode>,
    #[arg(long)]
    worktree_slug: Option<String>,
    #[arg(long)]
    new_branch: Option<String>,
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    pr_number: Option<std::num::NonZeroU32>,
    #[arg(long)]
    forge: Option<String>,
    #[arg(long, conflicts_with = "no_background")]
    background: bool,
    #[arg(long)]
    no_background: bool,
}
impl WorkspaceCommand {
    /// Validate source-specific combinations before the pending backend refusal.
    pub(crate) fn validate(&self) -> Result<(), CliError> {
        match self {
            Self::Create(args) => args.validate(),
            Self::Setup { id } | Self::Archive { id } | Self::Rename { id, .. } => {
                if id.trim().is_empty() || id.chars().any(char::is_control) {
                    Err(CliError::Usage(
                        "workspace ID must be nonempty text without controls",
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Ls { .. } => Ok(()),
        }
    }
}
impl CreateWorkspace {
    fn validate(&self) -> Result<(), CliError> {
        let mode = self.mode.unwrap_or(WorktreeMode::BranchOff);
        match self.isolation {
            Isolation::Local => {
                if self.mode.is_some()
                    || self.worktree_slug.is_some()
                    || self.new_branch.is_some()
                    || self.base.is_some()
                    || self.branch.is_some()
                    || self.pr_number.is_some()
                    || self.forge.is_some()
                {
                    return Err(CliError::Usage(
                        "worktree source flags require --isolation worktree",
                    ));
                }
            }
            Isolation::Worktree => match mode {
                WorktreeMode::BranchOff => {
                    if self.branch.is_some() || self.pr_number.is_some() || self.forge.is_some() {
                        return Err(CliError::Usage(
                            "branch-off accepts --new-branch and --base, not checkout source flags",
                        ));
                    }
                }
                WorktreeMode::CheckoutBranch => {
                    if self.branch.is_none()
                        || self.new_branch.is_some()
                        || self.base.is_some()
                        || self.pr_number.is_some()
                        || self.forge.is_some()
                    {
                        return Err(CliError::Usage(
                            "checkout-branch requires --branch and rejects other source flags",
                        ));
                    }
                }
                WorktreeMode::CheckoutPr => {
                    if self.pr_number.is_none()
                        || self.new_branch.is_some()
                        || self.base.is_some()
                        || self.branch.is_some()
                    {
                        return Err(CliError::Usage(
                            "checkout-pr requires --pr-number and rejects branch source flags",
                        ));
                    }
                }
            },
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Command, parity::ParityCommand};
    use clap::Parser as _;
    fn valid(args: &[&str]) -> bool {
        let parsed = Cli::try_parse_from(args);
        match parsed {
            Ok(Cli {
                command: Command::Parity(ParityCommand::Workspace { command }),
                ..
            }) => command.validate().is_ok(),
            _ => false,
        }
    }
    #[test]
    fn isolation_and_checkout_source_modes_are_never_interchanged() {
        assert!(valid(&[
            "vessel",
            "workspace",
            "create",
            "--isolation",
            "worktree",
            "--mode",
            "checkout-pr",
            "--pr-number",
            "1"
        ]));
        assert!(!valid(&[
            "vessel",
            "workspace",
            "create",
            "--isolation",
            "local",
            "--branch",
            "main"
        ]));
        assert!(!valid(&[
            "vessel",
            "workspace",
            "create",
            "--isolation",
            "worktree",
            "--mode",
            "checkout-branch"
        ]));
        assert!(!valid(&[
            "vessel",
            "workspace",
            "create",
            "--isolation",
            "local",
            "--background",
            "--no-background"
        ]));
    }
}
