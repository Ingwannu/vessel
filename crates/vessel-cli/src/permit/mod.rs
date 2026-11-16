//! Read-only pending requests use the canonical session snapshot and shared output.
//! Response commands stay test-only until their durable rich contract is checked.

#[cfg(test)]
mod confirmation;
#[cfg(test)]
mod input;
mod pending;
#[cfg(test)]
mod question_input;
#[cfg(test)]
mod selection;
mod source;

#[cfg(test)]
use crate::selector::{IdPrefix, SessionSelector};
#[cfg(test)]
use clap::Args;
use clap::Subcommand;
#[cfg(test)]
pub(crate) use input::ModifiedInput;
#[cfg(test)]
pub(crate) use selection::{Direction, Scope, SelectionError};

/// External flags stay at the decoding edge; domain selection is an enum.
#[cfg(test)]
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

#[cfg(test)]
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
#[cfg(test)]
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
    #[cfg(test)]
    Allow {
        #[command(flatten)]
        requests: Requests,
        /// JSON is validated before connecting; provider validation still applies.
        #[arg(long, value_parser = input::Parser)]
        input: Option<ModifiedInput>,
    },
    /// Deny an explicitly named request, or every request with --all.
    #[cfg(test)]
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
            #[cfg(test)]
            Self::Allow { requests, input } => formatter
                .debug_struct("Allow")
                .field("requests", requests)
                .field("input", input)
                .finish(),
            #[cfg(test)]
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

#[cfg(test)]
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

pub(crate) use source::SourceError as ListError;

pub(crate) async fn run(
    target: &crate::args::Target,
    presentation: crate::args::parity::Presentation,
    command: &Command,
) -> Result<(), crate::error::CliError> {
    match command {
        Command::Ls => {
            let mut client = crate::connect::connect(target).await?.client;
            let rows = source::list(&mut client).await?;
            crate::parity::output::list(presentation, &rows).await
        }
        // Private parser fixtures must never reach a legacy response fallback.
        #[cfg(test)]
        Command::Allow { .. } | Command::Deny { .. } => Err(crate::error::CliError::Usage(
            "permit responses are not available; no answer was sent",
        )),
    }
}

#[cfg(test)]
mod tests;
