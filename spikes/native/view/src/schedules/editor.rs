//! Typed form boundary. Raw text becomes a protocol value once, when edited;
//! submission uses the validated value and the revision that was opened.

use thiserror::Error;
use vessel_proto::SessionMode;
use vessel_proto::automation::{
    Cadence, CronExpr, EverySeconds, InvalidAutomationText, InvalidCron, OutOfRange,
    ScheduleDefinition, ScheduleInfo, ScheduleState, TimeZoneName, UpdateSchedule,
};

/// Cadence form input, before parsing; no string encodes the selected kind.
#[derive(Debug, Clone, Copy)]
pub enum CadenceInput<'a> {
    /// Elapsed seconds (including intervals that do not divide a day).
    Every(&'a str),
    /// Wall-clock rule and its explicitly selected zone.
    Cron {
        /// Five-field expression or a supported shorthand.
        expression: &'a str,
        /// IANA name (existence and possible due times are checked by the node).
        zone: &'a str,
    },
}

/// Why a cadence cannot be put into the form's typed value.
#[derive(Debug, Error)]
pub enum CadenceError {
    /// Seconds were not a whole positive integer.
    #[error("{}", crate::Lang::En.schedules().interval_number)]
    Number(#[source] std::num::ParseIntError),
    /// Seconds were outside the node's supported interval.
    #[error(transparent)]
    Range(#[from] OutOfRange),
    /// Cron did not match the wire's grammar.
    #[error(transparent)]
    Cron(#[from] InvalidCron),
    /// Zone name was malformed.
    #[error(transparent)]
    Zone(#[from] InvalidAutomationText),
}

impl CadenceInput<'_> {
    /// Parse at the form edge; neither the renderer nor submission reparses it.
    ///
    /// # Errors
    /// Malformed seconds, an out-of-range interval, cron syntax or zone shape.
    pub fn parse(self) -> Result<Cadence, CadenceError> {
        match self {
            Self::Every(text) => {
                let seconds = text.trim().parse::<u32>().map_err(CadenceError::Number)?;
                Ok(Cadence::Every {
                    seconds: EverySeconds::try_from(seconds)?,
                    anchor: None,
                })
            }
            Self::Cron { expression, zone } => Ok(Cadence::Cron {
                expr: expression.parse::<CronExpr>()?,
                zone: zone.parse::<TimeZoneName>()?,
            }),
        }
    }
}

/// Why a whole definition cannot be safely replaced by this client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EditError {
    /// At least one field is a newer node's variant.
    #[error("{}", crate::Lang::En.schedules().unknown)]
    Unknown,
    /// The node has ended it; updates would conflict.
    #[error("{}", crate::Lang::En.schedules().edit_ended)]
    Ended,
    /// Automatic runs only support structured sessions without resume.
    #[error("{}", crate::Lang::En.schedules().structured_only)]
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
    if let vessel_proto::automation::ScheduleTarget::NewSession { spec, .. } = &definition.target {
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
