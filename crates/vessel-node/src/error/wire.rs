//! The node's single service-error-to-wire boundary, separate from error storage.
//! Keeping this mapping beside its owner avoids growing the error enum file
//! beyond its budget; existing codes and cause chains remain shared.

use super::ServiceError;
use crate::profiles::ProfilesFileError;
use vessel_proto::launch_profiles::LaunchProfileRefusal;
use vessel_proto::{DeviceInviteError, ErrorCode, ErrorDetail, WireError};

impl ServiceError {
    /// The error as the client sees it: a code to branch on and the full
    /// chain of causes as text (none of which contains secrets).
    pub(crate) fn to_wire(&self) -> WireError {
        let code = self.code();
        let mut message = self.to_string();
        let mut source = std::error::Error::source(self);
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        let mut error = WireError::new(code, message);
        if matches!(self, Self::NoPairingRoutes) {
            error.detail = Some(ErrorDetail::DeviceInvite {
                reason: DeviceInviteError::NoRoutes,
            });
        }
        let problem = self.launch_problem();
        if let Some(problem) = problem {
            error.detail = Some(ErrorDetail::LaunchProfile { problem });
        }
        error
    }
    // One exhaustive classification remains the error-code owner.
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::UnknownAgent(_) | Self::NoRecord { .. } => {
                ErrorCode::NotFound
            }
            // `Profiles(Parse)`: the call is fine; the node's file is not
            // (and stays unused).
            Self::LaunchChanged
            | Self::LaunchStale(_)
            | Self::LaunchExhausted
            | Self::Ended(_)
            | Self::Profiles(
                ProfilesFileError::Parse(_)
                | ProfilesFileError::ParseAgents(_)
                | ProfilesFileError::ReadAgents(vessel_store::AgentsFileError::Unsafe { .. }),
            )
            | Self::ProfileGone(_)
            | Self::HomeUnknown(_)
            | Self::NotLoggedIn(_)
            | Self::AgentGone(_)
            | Self::MoveRunning(_)
            | Self::ForkRunning(_)
            | Self::Live(_)
            | Self::LogWriting(_)
            | Self::AlreadyLive(_)
            | Self::StartingAgain(_)
            | Self::FolderGone { .. }
            | Self::WorktreeBusy(_)
            | Self::AgentSecret { .. }
            | Self::Installing(_)
            | Self::Models(_)
            | Self::NoPairingRoutes => ErrorCode::Conflict,
            Self::LaunchAgentOrigin
            | Self::AgentDepth { .. }
            | Self::AgentLimit { .. }
            | Self::NotOwned(_)
            | Self::AgentMarksRead => ErrorCode::Forbidden,
            Self::Draining | Self::TooManyInvitations | Self::LaunchBusy(_) => {
                ErrorCode::Unavailable
            }
            Self::Ahead { .. }
            | Self::ReadAhead { .. }
            | Self::InvalidTtl
            | Self::SideCreationLabels
            | Self::NoScopes
            | Self::LaunchHarnessGone
            | Self::UnknownProfile(_)
            | Self::ProfileRemoved { .. }
            | Self::UnknownRoute { .. }
            | Self::ProfileHarness { .. } => ErrorCode::InvalidRequest,
            Self::DidNotStart { .. } => ErrorCode::SpawnFailed,
            Self::MoveRoute { .. }
            | Self::NotAPoint { .. }
            | Self::LabelsUnassigned
            | Self::LaunchStructuredUnavailable(_) => ErrorCode::Unsupported,
            Self::SideQuestion(error) => error.code(),
            Self::Session(error) => error.code(),
            Self::Transfer(error) => error.code(),
            Self::Upload(error) => error.code(),
            Self::Store(error)
            | Self::Profiles(
                ProfilesFileError::Read(error)
                | ProfilesFileError::ReadAgents(vessel_store::AgentsFileError::Read(error)),
            ) => error.code(),
            Self::Added(error) => error.code(),
            Self::Outbound(error) => error.code(),
            Self::LaunchValues(_)
            | Self::LaunchWorker(_)
            | Self::Unpersisted(_)
            | Self::Task(_)
            | Self::Random(_)
            | Self::Link(_)
            | Self::AgentTools(_)
            | Self::AgentToolsPath
            | Self::Unrouted(_)
            | Self::ModelProxyDown => ErrorCode::Internal,
        }
    }

    fn launch_problem(&self) -> Option<LaunchProfileRefusal> {
        if let Self::LaunchBusy(_) = self {
            Some(LaunchProfileRefusal::Busy)
        } else if let Self::LaunchChanged = self {
            Some(LaunchProfileRefusal::ConfigurationChanged)
        } else if let Self::LaunchStale(current) = self {
            Some(LaunchProfileRefusal::StaleRevision { current: *current })
        } else if let Self::LaunchExhausted = self {
            Some(LaunchProfileRefusal::RevisionExhausted)
        } else if let Self::LaunchAgentOrigin = self {
            Some(LaunchProfileRefusal::AgentOrigin)
        } else if let Self::LaunchStructuredUnavailable(_) = self {
            Some(LaunchProfileRefusal::StructuredUnavailable)
        } else {
            None
        }
    }
}
