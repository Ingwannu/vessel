//! Typed calls stay on the original selected connection, including every error.
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use vessel_app_view::workspace::registry_projection::{RegistryData, RegistryReadFailure};
use vessel_client::CallError;
use vessel_proto::{
    registry::{GetRegistry, RegistryRevision},
    workspace::{ListProjects, ListWorktrees},
};
const READ_DEADLINE: Duration = Duration::from_secs(60);

pub(super) async fn bounded(
    selected: &SelectedConnection,
    changed_from: Option<RegistryRevision>,
) -> Result<RegistryData, RegistryReadFailure> {
    tokio::time::timeout(READ_DEADLINE, read(selected, changed_from))
        .await
        .map_or(Err(RegistryReadFailure::Refused), |result| result)
}

async fn read(
    selected: &SelectedConnection,
    changed_from: Option<RegistryRevision>,
) -> Result<RegistryData, RegistryReadFailure> {
    // Read catalogs after the long poll resolves, avoiding arbitrarily old
    // catalogs while still making no cross-call atomicity promise.
    let registry = selected
        .call(GetRegistry { changed_from })
        .await
        .map_err(failure)?;
    let projects = selected.call(ListProjects {}).await.map_err(failure)?;
    let worktrees = selected
        .call(ListWorktrees { project: None })
        .await
        .map_err(failure)?;
    let data = RegistryData {
        registry,
        projects,
        worktrees,
    };
    data.check_size()
        .map_err(|_over_budget| RegistryReadFailure::Refused)?;
    Ok(data)
}

fn failure(error: RequestFailure) -> RegistryReadFailure {
    match error {
        RequestFailure::Refused(_) | RequestFailure::Call(CallError::Disconnected) => {
            RegistryReadFailure::Offline
        }
        RequestFailure::Call(
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. },
        ) => RegistryReadFailure::Unsupported,
        RequestFailure::Call(CallError::Remote(wire)) => {
            if wire.code == vessel_proto::ErrorCode::Unsupported {
                RegistryReadFailure::Unsupported
            } else {
                RegistryReadFailure::Refused
            }
        }
        RequestFailure::Call(CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted) => {
            RegistryReadFailure::Refused
        }
    }
}
