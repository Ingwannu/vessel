//! Schedules (protocol 11, ADR 0033): a prompt sent on a cadence, to a new
//! session or through an existing session's queue, as the person or agent
//! that made it.
//!
//! [Decision Log]
//! - Purpose: agent-orchestrator's automations, GOAT's scheduled tasks and
//!   Paseo's schedules and heartbeats in one shape: when (a [`Cadence`]),
//!   where ([`ScheduleTarget`]), what ([`PromptText`]), and when it stops
//!   (`max_runs`, `ends_at`: Paseo's two; the other two have neither).
//! - The definition is what a person writes ([`ScheduleDefinition`]), apart
//!   from what the node keeps about it ([`ScheduleInfo`]): `create` takes a
//!   definition and `update` replaces one whole under a `revision`, so two
//!   people editing one schedule cannot overwrite each other unseen (the
//!   queue's rule, protocol 10) and no field needs a "leave as is" form.
//! - A new session's target is a `SessionSpec`, exactly what
//!   `session.create` takes, so a run goes through `session.create`'s own
//!   checks (profile, place, an agent's limits) and nothing here restates
//!   them.
//! - Every enum a newer node may grow has an `unknown` variant a client
//!   shows as such (protocol 9's rule for options).

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::cron::CronExpr;
use super::run::ScheduleRun;
use super::text::{PromptText, ScheduleName, TimeZoneName};
use crate::ProtocolVersion;
use crate::id::{ScheduleId, SessionId};
use crate::profile::Creator;
use crate::session::SessionSpec;

/// Seconds between an `every` schedule's due times: at least a minute
/// (all three references' floor), at most 366 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u32", into = "u32")]
pub struct EverySeconds(u32);

impl EverySeconds {
    /// The shortest: one minute.
    pub const MIN: u32 = 60;
    /// The longest: 366 days.
    pub const MAX: u32 = 366 * 24 * 60 * 60;

    /// The seconds.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A number outside the range its type takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange {
    /// What it was meant to be.
    pub what: &'static str,
    /// The smallest taken.
    pub min: u32,
    /// The largest taken.
    pub max: u32,
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is {} to {}", self.what, self.min, self.max)
    }
}

impl std::error::Error for OutOfRange {}

impl TryFrom<u32> for EverySeconds {
    type Error = OutOfRange;

    fn try_from(seconds: u32) -> Result<Self, Self::Error> {
        if (Self::MIN..=Self::MAX).contains(&seconds) {
            Ok(Self(seconds))
        } else {
            Err(OutOfRange {
                what: "an interval in seconds",
                min: Self::MIN,
                max: Self::MAX,
            })
        }
    }
}

impl From<EverySeconds> for u32 {
    fn from(seconds: EverySeconds) -> Self {
        seconds.0
    }
}

/// How many runs a schedule makes before it ends: 1 to 1,000,000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u32", into = "u32")]
pub struct MaxRuns(u32);

impl MaxRuns {
    /// The largest taken.
    pub const MAX: u32 = 1_000_000;

    /// The count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for MaxRuns {
    type Error = OutOfRange;

    fn try_from(runs: u32) -> Result<Self, Self::Error> {
        if (1..=Self::MAX).contains(&runs) {
            Ok(Self(runs))
        } else {
            Err(OutOfRange {
                what: "a number of runs",
                min: 1,
                max: Self::MAX,
            })
        }
    }
}

impl From<MaxRuns> for u32 {
    fn from(runs: MaxRuns) -> Self {
        runs.0
    }
}

/// When a schedule is due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cadence {
    /// Every so often, in elapsed time: due at `anchor + k × seconds` for
    /// `k ≥ 0`, always after the schedule was made (or last changed its
    /// cadence). A day is 86,400 seconds whatever the wall clock does; use
    /// `cron` for wall-clock times.
    Every {
        /// The interval.
        seconds: EverySeconds,
        /// Where the count starts; the moment the cadence was set when
        /// left out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<DateTime<Utc>>,
    },
    /// At the wall-clock times a cron expression names, in a time zone.
    Cron {
        /// The times.
        expr: CronExpr,
        /// The zone they are read in.
        zone: TimeZoneName,
    },
    /// A cadence a newer node has; a client shows it, it cannot write one.
    #[serde(other)]
    Unknown,
}

/// Where a schedule's prompt goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleTarget {
    /// A new session for each run, started as `session.create` would with
    /// this spec (structured, without `resume`), the prompt sent once it is
    /// ready.
    NewSession {
        /// What to start.
        spec: Box<SessionSpec>,
        /// What becomes of the session after its first turn.
        #[serde(default)]
        after: AfterRun,
        /// Explicit per-run isolation. Absent on a definition made for legacy placement.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        isolation: Option<crate::registry::WorkspaceIsolation>,
    },
    /// An existing structured session, through its queue: a turn starts
    /// when it is idle and nothing waits; otherwise the prompt waits its
    /// turn. Never steered into a turn, never interrupting one.
    Session {
        /// Which session.
        session: SessionId,
    },
    /// A target a newer node has.
    #[serde(other)]
    Unknown,
}

/// What becomes of a run's new session once its first turn has ended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AfterRun {
    /// Archived (ADR 0026) when nothing waits in its queue: it stops, its
    /// log stays, it can start again (Paseo's `archiveOnFinish`). Without
    /// this every run would leave one more agent running.
    #[default]
    Archive,
    /// Left live; the schedule's next run waits for it to end
    /// (agent-orchestrator's one live session per automation).
    Keep,
    /// A choice a newer node has.
    #[serde(other)]
    Unknown,
}

/// What a schedule does about due times it missed (the node stopped, the
/// machine slept, the clock jumped).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CatchUp {
    /// One run for all of them, if the latest was missed by less than 24
    /// hours.
    #[default]
    Once,
    /// None: a record says how many were missed.
    Skip,
    /// A choice a newer node has.
    #[serde(other)]
    Unknown,
}

/// A schedule as a person writes it: `schedule.create` takes it,
/// `schedule.update` replaces it whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleDefinition {
    /// What it is called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<ScheduleName>,
    /// When it is due.
    pub cadence: Cadence,
    /// Where its prompt goes.
    pub target: ScheduleTarget,
    /// What it sends.
    pub prompt: PromptText,
    /// What it does about missed due times.
    #[serde(default)]
    pub catch_up: CatchUp,
    /// It ends after this many runs that did something.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_runs: Option<MaxRuns>,
    /// It ends when its next due time would be after this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<DateTime<Utc>>,
}

/// Why a schedule ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// It made `max_runs` runs.
    MaxRuns,
    /// Its next due time was after `ends_at`.
    Expired,
    /// The device that defined it was revoked.
    CreatorGone,
    /// What it names is gone: the session was deleted, the project or the
    /// worktree removed.
    TargetGone,
    /// A reason a newer node has.
    #[serde(other)]
    Unknown,
}

/// Whether a schedule runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ScheduleState {
    /// It runs; next at `next_due`.
    Active {
        /// Its next due time.
        next_due: DateTime<Utc>,
    },
    /// It does not run until resumed.
    Paused {
        /// Since when.
        since: DateTime<Utc>,
    },
    /// It will not run again.
    Ended {
        /// When.
        at: DateTime<Utc>,
        /// Why.
        reason: EndReason,
    },
    /// A state a newer node has.
    #[serde(other)]
    Unknown,
}

/// A schedule as the node keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleInfo {
    /// Its id.
    pub id: ScheduleId,
    /// What it does.
    pub definition: ScheduleDefinition,
    /// Whether it runs.
    pub state: ScheduleState,
    /// The runs that sent their prompt: what `max_runs` bounds.
    pub runs: u32,
    /// The due time its last run was for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_due: Option<DateTime<Utc>>,
    /// Its newest run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<ScheduleRun>,
    /// The creator or last full-definition writer; a run acts as them
    /// (ADR 0033). A cadence-only edit keeps the existing owner.
    pub defined_by: Creator,
    /// When it was made.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// Changes when its definition changes or a person pauses or resumes
    /// it, not at a run; `schedule.update` names the one it was made from.
    pub revision: u32,
}

/// `schedule.list`: every schedule on the node (at most 256). Needs
/// `read`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListSchedules {}

/// Reply to `schedule.list`, by creation, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleList {
    /// The schedules.
    pub schedules: Vec<ScheduleInfo>,
}

/// `schedule.get`: one schedule. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetSchedule {
    /// Which.
    pub schedule: ScheduleId,
}

/// `schedule.create`: make a schedule. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateSchedule {
    /// What it does.
    pub definition: ScheduleDefinition,
    /// Paused from the start.
    #[serde(default)]
    pub paused: bool,
}

/// `schedule.update`: replace a schedule's definition. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdateSchedule {
    /// Which.
    pub schedule: ScheduleId,
    /// The revision the change was made from; another is `conflict`.
    pub revision: u32,
    /// The new definition.
    pub definition: ScheduleDefinition,
}

/// Protocol for editing a heartbeat's cadence without changing its owner (ADR 0033).
pub const HEARTBEAT_CADENCE_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(48));

/// Per-run workspace isolation. Integration assigns the protocol when consumers land.
pub const SCHEDULE_ISOLATION_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(50));

/// Whether all hops on a schedule route can carry the isolation policy.
#[must_use]
pub fn serves_schedule_isolation(protocol: ProtocolVersion) -> bool {
    SCHEDULE_ISOLATION_PROTOCOL.is_some_and(|required| protocol >= required)
}

impl ScheduleTarget {
    /// The explicitly saved policy, as distinct from legacy placement.
    #[must_use]
    pub const fn isolation(&self) -> Option<&crate::registry::WorkspaceIsolation> {
        match self {
            Self::NewSession { isolation, .. } => isolation.as_ref(),
            Self::Session { .. } | Self::Unknown => None,
        }
    }
}

/// `schedule.update_cadence`: change only an existing-session schedule's
/// cadence. Needs a direct connection and `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdateScheduleCadence {
    /// Which schedule.
    pub schedule: ScheduleId,
    /// The revision the change was made from; another is `conflict`.
    pub revision: u32,
    /// The new cadence.
    pub cadence: Cadence,
}

/// `schedule.pause`: stop a schedule's runs. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PauseSchedule {
    /// Which.
    pub schedule: ScheduleId,
}

/// `schedule.resume`: let a paused schedule run again, from now. Needs
/// `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResumeSchedule {
    /// Which.
    pub schedule: ScheduleId,
}

/// `schedule.run`: run a schedule once now, outside its cadence. Needs
/// `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunSchedule {
    /// Which.
    pub schedule: ScheduleId,
}

/// `schedule.delete`: remove a schedule and its runs. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeleteSchedule {
    /// Which.
    pub schedule: ScheduleId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_isolation_is_the_newest_protocol() {
        // Keep the newest equality pin here until the next activation.
        assert_eq!(SCHEDULE_ISOLATION_PROTOCOL, Some(crate::PROTOCOL_VERSION));
    }

    #[test]
    fn heartbeat_cadence_keeps_its_assigned_protocol() {
        assert_eq!(HEARTBEAT_CADENCE_PROTOCOL, Some(ProtocolVersion::new(48)));
    }

    #[test]
    fn cadence_update_uses_the_feature_protocol_and_direct_operate_scope() {
        use crate::Scope;
        use crate::rpc::Request;

        let method = UpdateScheduleCadence::METHOD;
        let request = UpdateScheduleCadence {
            schedule: ScheduleId::random(),
            revision: 2,
            cadence: Cadence::Cron {
                expr: "0 9 * * *".parse().unwrap(),
                zone: "Asia/Seoul".parse().unwrap(),
            },
        };
        let encoded = serde_json::to_value(request.clone().into_call()).unwrap();
        assert_eq!(encoded["method"], "schedule.update_cadence");
        assert_eq!(encoded["params"].as_object().unwrap().len(), 3);
        assert_eq!(method.scope(), Some(Scope::Operate));
        assert!(!method.forwardable());
        assert_eq!(
            request.into_call().required_protocol(),
            HEARTBEAT_CADENCE_PROTOCOL
        );
    }

    #[test]
    fn numbers_are_held_to_their_ranges() {
        assert!(serde_json::from_str::<EverySeconds>("59").is_err());
        assert_eq!(
            serde_json::from_str::<EverySeconds>("60").unwrap().get(),
            60
        );
        assert!(serde_json::from_str::<EverySeconds>("31622401").is_err());
        assert!(serde_json::from_str::<MaxRuns>("0").is_err());
        assert_eq!(serde_json::from_str::<MaxRuns>("3").unwrap().get(), 3);
    }

    #[test]
    fn cadences_have_their_wire_names() {
        let cadence: Cadence =
            serde_json::from_str(r#"{"kind":"cron","expr":"0 9 * * 1-5","zone":"Asia/Seoul"}"#)
                .unwrap();
        assert!(matches!(cadence, Cadence::Cron { .. }));
        let every: Cadence = serde_json::from_str(r#"{"kind":"every","seconds":300}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&every).unwrap(),
            r#"{"kind":"every","seconds":300}"#
        );
        let newer: Cadence = serde_json::from_str(r#"{"kind":"rrule"}"#).unwrap();
        assert_eq!(newer, Cadence::Unknown);
        let ended: ScheduleState = serde_json::from_str(r#"{"state":"retired"}"#).unwrap();
        assert_eq!(ended, ScheduleState::Unknown);
    }

    #[test]
    fn a_definition_leaves_out_what_is_not_set() {
        let definition = ScheduleDefinition {
            name: None,
            cadence: Cadence::Every {
                seconds: EverySeconds::try_from(3600).unwrap(),
                anchor: None,
            },
            target: ScheduleTarget::Session {
                session: SessionId::random(),
            },
            prompt: "Check the build.".parse().unwrap(),
            catch_up: CatchUp::Once,
            max_runs: None,
            ends_at: None,
        };
        let json = serde_json::to_string(&definition).unwrap();
        assert!(
            !json.contains("name") && !json.contains("max_runs"),
            "{json}"
        );
        assert!(json.contains(r#""catch_up":"once""#), "{json}");
        assert_eq!(
            serde_json::from_str::<ScheduleDefinition>(&json).unwrap(),
            definition
        );
    }
}
