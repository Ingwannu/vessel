//! Reject unassigned session metadata at creation and dynamic import edges.
//! This temporary node-side guard precedes writes/spawns; the shared
//! shared optional call accessor rejects explicit fields before client sends.

use crate::error::ServiceError;
use vessel_proto::automation::ScheduleTarget;
use vessel_proto::{SessionInfo, SessionLabels, SessionSpec, WireError};

fn labels(value: &SessionLabels) -> Result<(), WireError> {
    if !value.is_empty() && vessel_proto::lifecycle_version::SINCE.is_none() {
        return Err(ServiceError::LabelsUnassigned.to_wire());
    }
    Ok(())
}

/// Existing create calls must not silently accept an unassigned field.
pub(crate) fn spec(value: &SessionSpec) -> Result<(), WireError> {
    labels(&value.labels)?;
    if value.listing.is_some() && vessel_proto::lifecycle_version::SINCE.is_none() {
        return Err(ServiceError::ListingUnassigned.to_wire());
    }
    Ok(())
}

/// Saved templates follow the same boundary before any lookup or write.
pub(crate) fn template(value: Option<&SessionSpec>) -> Result<(), WireError> {
    value.map_or(Ok(()), spec)
}

/// Only a new-session schedule carries a creation specification.
pub(crate) fn target(value: &ScheduleTarget) -> Result<(), WireError> {
    match value {
        ScheduleTarget::NewSession { spec: value, .. } => spec(value),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => Ok(()),
    }
}

/// An import may carry different creation and current maps; neither is accepted yet.
pub(crate) fn imported(value: &SessionInfo) -> Result<(), WireError> {
    labels(&value.labels)?;
    if !value.listing.is_shown() && vessel_proto::lifecycle_version::SINCE.is_none() {
        return Err(ServiceError::ListingUnassigned.to_wire());
    }
    spec(&value.spec)
}

/// Validate a manifest before any files are fetched when it will start a session.
pub(crate) fn starting_import(
    value: &SessionInfo,
    request: &vessel_proto::ImportSession,
) -> Result<(), WireError> {
    if request.start {
        imported(value)
    } else {
        Ok(())
    }
}
