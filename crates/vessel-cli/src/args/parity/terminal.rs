//! Terminal names are parsed once; aliases never select the first ambiguous match.
use crate::error::CliError;
use clap::Subcommand;
use std::{path::PathBuf, str::FromStr};
use vessel_proto::{SessionTitle, terminals::CaptureLine};

const MAX_SELECTOR_CHARS: usize = 1024;
#[derive(Debug, Clone)]
pub(crate) struct TerminalSelector(String);
impl FromStr for TerminalSelector {
    type Err = CliError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.trim().is_empty()
            || text.chars().any(char::is_control)
            || text.chars().count() > MAX_SELECTOR_CHARS
        {
            return Err(CliError::Usage(
                "terminal selector must be nonempty bounded text without controls",
            ));
        }
        Ok(Self(text.to_owned()))
    }
}
impl TerminalSelector {
    pub(crate) fn text(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Subcommand)]
pub(crate) enum TerminalCommand {
    Ls {
        #[arg(long,conflicts_with_all=["cwd","workspace"])]
        all: bool,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long, conflicts_with = "cwd")]
        workspace: Option<String>,
    },
    Create {
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        name: Option<SessionTitle>,
    },
    Kill {
        id: TerminalSelector,
    },
    Capture {
        id: TerminalSelector,
        #[arg(long, allow_hyphen_values = true)]
        start: Option<CaptureLine>,
        #[arg(long, allow_hyphen_values = true)]
        end: Option<CaptureLine>,
        #[arg(short = 'S', long)]
        scrollback: bool,
        #[arg(long)]
        ansi: bool,
    },
    SendKeys {
        id: TerminalSelector,
        #[arg(required = true)]
        keys: Vec<String>,
        #[arg(short = 'l', long)]
        literal: bool,
    },
}
