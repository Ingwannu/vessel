//! CLI controls expose ownership and availability separately from installation progress.

/// Only this application’s exact symlink or Windows PATH directory is owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// No entry exists at the user-local destination.
    Absent,
    /// This installation owns the entry exactly.
    Installed,
    /// An unrelated entry survives install and uninstall.
    Foreign,
}

/// A release without its CLI cannot offer a successful installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The expected bundled executable is present.
    Bundled,
    /// This build omitted the CLI.
    Missing,
}

/// A successful install may still need a terminal PATH change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    /// The inherited terminal environment includes the directory.
    Available,
    /// A known shell rc or Windows user PATH supplies it for future terminals.
    Configured,
    /// Unknown shells or refused platform updates need manual terminal configuration.
    Manual,
}

/// Installation and removal have different ownership gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Add an absent owned link/shim.
    Install,
    /// Remove only this application's exact link/shim.
    Remove,
}

/// A completed status read or mutation, shared by Settings and deferred onboarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    /// Missing companions must not be offered as an onboarding choice.
    pub availability: Availability,
    /// Installation owns only the exact app-provided destination.
    pub status: Status,
    /// A successful mutation can still require manual terminal configuration.
    pub path: PathStatus,
}

/// The screen retains honest I/O progress and exposes no placeholder actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// Status is being read away from the render thread.
    #[default]
    Checking,
    /// Presence, ownership and PATH facts came from the host.
    Ready {
        /// Presence of the actual bundled CLI executable.
        availability: Availability,
        /// Exact ownership of the user-local destination.
        status: Status,
        /// Whether a fresh terminal can find that destination.
        path: PathStatus,
    },
    /// One bounded operation is in progress; duplicate activation is rejected.
    Working(Operation),
    /// Windows transactions are disabled; PATH must be configured manually.
    UserPathUnavailable,
    /// A typed I/O refusal can be retried by refreshing status.
    Failed,
}

impl From<Snapshot> for State {
    fn from(snapshot: Snapshot) -> Self {
        Self::Ready {
            availability: snapshot.availability,
            status: snapshot.status,
            path: snapshot.path,
        }
    }
}

impl State {
    /// Ownership and release contents gate the offered operation before I/O starts.
    #[must_use]
    pub fn allows(self, operation: Operation) -> bool {
        match self {
            Self::Checking | Self::Working(_) | Self::Failed | Self::UserPathUnavailable => false,
            Self::Ready {
                availability,
                status,
                ..
            } => match operation {
                Operation::Install => match availability {
                    Availability::Bundled => status == Status::Absent,
                    Availability::Missing => false,
                },
                Operation::Remove => status == Status::Installed,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_foreign_and_inflight_states_never_offer_a_mutating_action() {
        for state in [
            State::Checking,
            State::Working(Operation::Install),
            State::Working(Operation::Remove),
            State::Failed,
            State::UserPathUnavailable,
            State::Ready {
                availability: Availability::Bundled,
                status: Status::Foreign,
                path: PathStatus::Manual,
            },
            State::Ready {
                availability: Availability::Missing,
                status: Status::Absent,
                path: PathStatus::Manual,
            },
        ] {
            assert!(!state.allows(Operation::Install));
            assert!(!state.allows(Operation::Remove));
        }
        let owned = State::Ready {
            availability: Availability::Missing,
            status: Status::Installed,
            path: PathStatus::Manual,
        };
        assert!(owned.allows(Operation::Remove));
        assert!(!owned.allows(Operation::Install));
    }
}
