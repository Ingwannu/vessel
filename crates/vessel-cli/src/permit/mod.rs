//! Staged permit command edge. Product registration waits for ADR 0084's
//! acknowledged response contract and W17's reviewed output interface.

mod confirmation;
mod input;
mod pending;
mod question_input;
mod selection;
mod source;

use crate::selector::{IdPrefix, SessionSelector};
use clap::{Args, Subcommand};
pub(crate) use input::ModifiedInput;
pub(crate) use selection::{Direction, Scope, SelectionError};

/// External flags stay at the decoding edge; domain selection is an enum.
#[derive(Debug, Args)]
pub(crate) struct Requests {
    /// Agent session id, or a unique prefix on the selected node.
    #[arg(value_name = "AGENT")]
    pub(crate) session: SessionSelector,
    /// Pending request id or prefix. Allow defaults to all; deny does not.
    request: Option<IdPrefix>,
    /// Explicitly target every pending request on this session.
    #[arg(long)]
    all: bool,
}

impl Requests {
    /// Normalize the asymmetric defaults before connecting to a node.
    pub(crate) fn scope(&self, direction: Direction) -> Result<Scope<'_>, SelectionError> {
        if self.all {
            return Ok(Scope::All);
        }
        selection::scope(direction, self.request.as_ref())
    }
}

/// Whether denial also asks the provider to interrupt its current work.
/// A provider-visible interruption request, kept apart from permission denial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interruption {
    KeepRunning,
    Interrupt,
}

/// `permit` mirrors the surveyed command family, not `session answer` defaults.
#[derive(Subcommand)]
pub(crate) enum Command {
    /// List pending requests, including archived sessions.
    Ls,
    /// Allow one request, or all when no request is given.
    Allow {
        #[command(flatten)]
        requests: Requests,
        /// JSON is validated before connecting; provider validation still applies.
        #[arg(long, value_parser = input::Parser)]
        input: Option<ModifiedInput>,
    },
    /// Deny an explicitly named request, or every request with --all.
    Deny {
        #[command(flatten)]
        requests: Requests,
        /// Provider-visible explanation, without synthesizing a prompt.
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        interrupt: bool,
    },
}

// An explanation is a provider payload, not a diagnostic label. Keep argument
// correlation without printing that text through an enclosing Cli::Debug.
impl std::fmt::Debug for Command {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ls => formatter.write_str("Ls"),
            Self::Allow { requests, input } => formatter
                .debug_struct("Allow")
                .field("requests", requests)
                .field("input", input)
                .finish(),
            Self::Deny {
                requests,
                interrupt,
                ..
            } => formatter
                .debug_struct("Deny")
                .field("requests", requests)
                .field("interrupt", interrupt)
                .finish_non_exhaustive(),
        }
    }
}

impl Command {
    /// Validate selection policy before any lookup or mutation.
    pub(crate) fn validate(&self) -> Result<(), SelectionError> {
        match self {
            Self::Ls => Ok(()),
            Self::Allow { requests, .. } => requests.scope(Direction::Allow).map(|_| ()),
            Self::Deny { requests, .. } => requests.scope(Direction::Deny).map(|_| ()),
        }
    }

    /// Decode the external boolean once; provider work takes a typed intention.
    pub(crate) const fn interruption(&self) -> Interruption {
        match self {
            Self::Deny {
                interrupt: true, ..
            } => Interruption::Interrupt,
            Self::Ls
            | Self::Allow { .. }
            | Self::Deny {
                interrupt: false, ..
            } => Interruption::KeepRunning,
        }
    }
}

#[cfg(test)]
mod tests;
