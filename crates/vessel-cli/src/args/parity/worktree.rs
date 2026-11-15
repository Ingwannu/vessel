//! Hidden legacy worktree forms preserve their source mode until registry integration.
use super::workspace::WorktreeMode;
use crate::error::CliError;
use clap::Args;
use std::{num::NonZeroU32, path::PathBuf};
use vessel_proto::workspace::{BranchName, RefName};

#[derive(Debug, Args)]
pub(crate) struct WorktreeCreate {
    #[arg(long)]
    mode: Option<WorktreeMode>,
    #[arg(long)]
    new_branch: Option<BranchName>,
    #[arg(long)]
    base: Option<RefName>,
    #[arg(long)]
    branch: Option<BranchName>,
    #[arg(long)]
    pr_number: Option<NonZeroU32>,
    #[arg(long)]
    cwd: Option<PathBuf>,
}
impl WorktreeCreate {
    pub(crate) fn validate(&self) -> Result<(), CliError> {
        if self
            .cwd
            .as_ref()
            .is_some_and(|cwd| cwd.as_os_str().is_empty())
        {
            return Err(CliError::Usage("--cwd must be a nonempty source path"));
        }
        let valid = match self.mode.unwrap_or(WorktreeMode::BranchOff) {
            WorktreeMode::BranchOff => self.branch.is_none() && self.pr_number.is_none(),
            WorktreeMode::CheckoutBranch => {
                self.branch.is_some()
                    && self.new_branch.is_none()
                    && self.base.is_none()
                    && self.pr_number.is_none()
            }
            WorktreeMode::CheckoutPr => {
                self.pr_number.is_some()
                    && self.new_branch.is_none()
                    && self.base.is_none()
                    && self.branch.is_none()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(CliError::Usage(
                "worktree source flags conflict with the selected creation mode",
            ))
        }
    }
}
