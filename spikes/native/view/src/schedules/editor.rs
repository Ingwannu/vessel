//! Typed form boundary. Raw text becomes a protocol value once, when edited;
//! submission uses the validated value and the revision that was opened.

pub mod cadence;
pub mod definition;
pub mod fields;
pub mod io;
pub mod preview;

use thiserror::Error;
use vessel_proto::SessionMode;
use vessel_proto::automation::{
    ScheduleDefinition, ScheduleInfo, ScheduleState, ScheduleTarget, UpdateSchedule,
};

pub use super::cadence::{CadenceError, CadenceInput, IntervalParts, IntervalUnit};

/// Why a whole definition cannot be safely replaced by this client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EditError {
    /// At least one field is a newer node's variant.
    #[error("unknown schedule state or definition")]
    Unknown,
    /// The node has ended it; updates would conflict.
    #[error("an ended schedule cannot be edited")]
    Ended,
    /// Automatic runs only support structured sessions without resume.
    #[error("a scheduled session must be structured without resume")]
    SessionMode,
}

/// Validate constraints visible in the wire; never pretend to check the node's
/// zone database, project access, target existence or next due time here.
///
/// # Errors
/// An unknown variant, terminal session or a resume spec.
pub fn check_definition(definition: &ScheduleDefinition) -> Result<(), EditError> {
    if !super::editable(definition) {
        return Err(EditError::Unknown);
    }
    if let ScheduleTarget::NewSession { spec, .. } = &definition.target {
        match &spec.mode {
            SessionMode::Structured { resume: None, .. } => {}
            SessionMode::Structured {
                resume: Some(_), ..
            }
            | SessionMode::Terminal { .. } => {
                return Err(EditError::SessionMode);
            }
        }
    }
    Ok(())
}

/// Make a replacement carrying the opened revision, never the latest revision
/// from a background refresh: a concurrent edit must produce a visible conflict.
///
/// # Errors
/// Ended or unknown state, or an unsupported definition.
pub fn replacement(
    opened: &ScheduleInfo,
    definition: ScheduleDefinition,
) -> Result<UpdateSchedule, EditError> {
    match opened.state {
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => {}
        ScheduleState::Ended { .. } => return Err(EditError::Ended),
        ScheduleState::Unknown => return Err(EditError::Unknown),
    }
    check_definition(&opened.definition)?;
    check_definition(&definition)?;
    Ok(UpdateSchedule {
        schedule: opened.id,
        revision: opened.revision,
        definition,
    })
}
