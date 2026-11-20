//! The form's fields and the errors a save can carry.
//!
//! A save is a whole definition. It has no field that keeps the previous
//! `defined_by` (ADR 0072).

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    CreateSchedule, InvalidAutomationText, InvalidCron, OutOfRange, ScheduleInfo, TimeZoneName,
    UpdateSchedule,
};
use vessel_proto::{SessionId, SessionSpec};

use super::BlockedAct;

/// Which part of a stored definition must not be written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownPiece {
    /// `Cadence::Unknown`.
    Cadence,
    /// `ScheduleTarget::Unknown`.
    Target,
    /// `AfterRun::Unknown`.
    AfterRun,
    /// `CatchUp::Unknown`.
    CatchUp,
}

/// Why the form will not build a save. Several can be current at once.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleFormError {
    /// `ScheduleName` refused the text.
    #[error("the name is not a schedule name")]
    Name(#[source] InvalidAutomationText),
    /// `PromptText` refused the text.
    #[error("the prompt is not a prompt")]
    Prompt(#[source] InvalidAutomationText),
    /// `CronExpr` refused the text.
    #[error("the cron expression is not valid")]
    Cron(#[source] InvalidCron),
    /// `EverySeconds` refused the interval. It is not rounded.
    #[error("the interval is out of range")]
    Interval(#[source] OutOfRange),
    /// A cron has no zone, and neither the device nor the machine supplied one.
    #[error("a cron needs a time zone")]
    Zone,
    /// The cap was not a number from 1 to 1,000,000. Blank is not this error.
    #[error("the run cap is not a number of runs")]
    Cap,
    /// The cap was zero. That is not "unlimited".
    #[error("zero runs is not a cap")]
    ZeroCap,
    /// The expiry did not parse. A past instant is allowed and is not this.
    #[error("the expiry is not an instant")]
    Expiry,
    /// A new session needs a place.
    #[error("a new session needs a place")]
    Place,
    /// A terminal session, which the node refuses for a schedule.
    #[error("a schedule cannot start a terminal session")]
    Terminal,
    /// A spec that resumes a harness session. The node refuses it.
    #[error("a schedule cannot resume a harness session")]
    Resume,
    /// The edit would swap the target session.
    #[error("the target session is not the one this schedule has")]
    Target,
    /// The stored definition has a part this app must show and not rewrite.
    #[error("the schedule has a part this app cannot save")]
    UnknownDefinition(UnknownPiece),
    /// Saving would make a person the owner of an agent's schedule.
    #[error("saving would take an agent's schedule")]
    Owner,
    /// Create, update or run on a node reached only by forwarding.
    #[error("a forwarded call cannot create, update or run a schedule")]
    Reach(BlockedAct),
    /// The handshake is older than protocol 11.
    #[error("schedules are not on this machine")]
    OlderPeer,
    /// Ended, or a state this app does not know. The node refuses the change.
    #[error("the schedule is not running")]
    NotRunning,
    /// Archive was asked of a session target.
    #[error("archive on finish is only for a new session")]
    NotNewSession,
}

/// The five cron presets Paseo submits, still cron.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronPreset {
    /// `* * * * *`.
    EveryMinute,
    /// `0 * * * *`. A new schedule opens here.
    EveryHour,
    /// `0 9 * * *`.
    DailyNine,
    /// `0 9 * * 1-5`.
    WeekdaysNine,
    /// `0 9 * * 1`.
    MondaysNine,
}

impl CronPreset {
    /// The expression Paseo submits for this preset.
    #[must_use]
    pub const fn expression(self) -> &'static str {
        match self {
            Self::EveryMinute => "* * * * *",
            Self::EveryHour => "0 * * * *",
            Self::DailyNine => "0 9 * * *",
            Self::WeekdaysNine => "0 9 * * 1-5",
            Self::MondaysNine => "0 9 * * 1",
        }
    }
}

/// How the cadence field is filled. `Every` is not rewritten into cron.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CadenceField {
    /// One of the five presets.
    Preset(CronPreset),
    /// A cron the person typed.
    Cron(String),
    /// Elapsed seconds. Not rounded to a minute.
    Every(u32),
}

/// The run-cap field. Blank means no cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapField {
    /// No cap.
    Blank,
    /// What was typed, not yet parsed.
    Text(String),
}

/// The expiry field. A parsed instant may be in the past.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryField {
    /// No expiry.
    None,
    /// An absolute UTC instant.
    At(DateTime<Utc>),
    /// The text did not parse.
    Invalid,
}

/// Where the prompt goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetField {
    /// A new session. Place is required.
    NewSession(SessionSpec),
    /// An existing session.
    Session(SessionId),
}

/// Archive or keep, for a new session. Not a `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterChoice {
    /// `AfterRun::Archive`.
    Archive,
    /// `AfterRun::Keep`.
    Keep,
}

/// The fields the person has filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// Blank means no name.
    pub name: String,
    /// The prompt.
    pub prompt: String,
    /// Preset, cron text, or elapsed seconds.
    pub cadence: CadenceField,
    /// The zone the field shows. `None` is missing, not UTC.
    pub zone: Option<TimeZoneName>,
    /// Blank or text.
    pub max_runs: CapField,
    /// Absent, an instant, or unparsed.
    pub ends_at: ExpiryField,
    /// Archive or keep. Ignored for a session target.
    pub after: AfterChoice,
    /// New session or existing session.
    pub target: TargetField,
}

/// Create, or edit of one loaded schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormJob {
    /// `schedule.create`, not paused.
    Create,
    /// `schedule.update` of this record.
    Edit(ScheduleInfo),
}

/// A save the node can be sent. Neither carries `defined_by`: the node sets
/// that to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Save {
    /// Create, running from the start.
    Create(CreateSchedule),
    /// Replace the whole definition at this revision.
    Update(UpdateSchedule),
}

/// Every current error, and a save only when there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormReview {
    /// Field, owner and reach errors together.
    pub errors: Vec<ScheduleFormError>,
    /// Present only when `errors` is empty.
    pub save: Option<Save>,
}

impl FormReview {
    /// True only when a save was built.
    #[must_use]
    pub fn can_save(&self) -> bool {
        self.errors.is_empty() && self.save.is_some()
    }
}
