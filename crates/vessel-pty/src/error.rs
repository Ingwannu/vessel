//! Errors of this crate.

/// A PTY session could not be started.
///
/// `portable-pty` reports errors as `anyhow::Error`; they are kept as the
/// source of an `io::Error` (`io::Error::other`), so `anyhow` stays out of
/// this crate's API (`AGENTS.md` section 6) and the cause is not lost.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    /// The OS would not create a pseudo-terminal.
    #[error("cannot open a pseudo-terminal")]
    Open(#[source] std::io::Error),
    /// The program could not be started in the PTY.
    #[error("cannot start {program} in a pseudo-terminal")]
    Spawn {
        /// The program.
        program: String,
        /// What `portable-pty` reported.
        #[source]
        source: std::io::Error,
    },
    /// The program started but reports no process id.
    #[error("{program} started without a process id")]
    NoPid {
        /// The program.
        program: String,
    },
    /// The program's pid cannot name its process group.
    #[error("{program} started with an unusable pid")]
    Group {
        /// The program.
        program: String,
        /// Why the pid was rejected.
        #[source]
        source: vessel_proc::InvalidGroup,
    },
    /// The PTY's reader or writer could not be obtained.
    #[error("cannot attach to the pseudo-terminal")]
    Attach(#[source] std::io::Error),
    /// An I/O thread could not be started.
    #[error("cannot start a PTY thread")]
    Thread(#[source] std::io::Error),
}

impl PtyError {
    pub(crate) fn with_diagnostics(self, spec: &vessel_proc::CommandSpec) -> Self {
        match spec.diagnostic_policy() {
            vessel_proc::CommandDiagnostics::Standard => self,
            vessel_proc::CommandDiagnostics::Redacted => {
                let program = || spec.diagnostic_program().display().to_string();
                match self {
                    Self::Open(source) => Self::Open(spec.diagnostic_io_error(source)),
                    Self::Spawn { source, .. } => Self::Spawn {
                        program: program(),
                        source: spec.diagnostic_io_error(source),
                    },
                    Self::NoPid { .. } => Self::NoPid { program: program() },
                    Self::Group { source, .. } => Self::Group {
                        program: program(),
                        source,
                    },
                    Self::Attach(source) => Self::Attach(spec.diagnostic_io_error(source)),
                    Self::Thread(source) => Self::Thread(spec.diagnostic_io_error(source)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

/// The PTY no longer accepts input: the program exited and the PTY was
/// closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the terminal is closed")]
pub struct PtyClosed;
