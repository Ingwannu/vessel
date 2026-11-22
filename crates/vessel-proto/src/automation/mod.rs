//! Automation (protocol 11, milestone 11's track D): schedules, a session's
//! recurring prompt to itself, saved prompts, telling an agent when a
//! session it started finishes, and webhooks.
//!
//! Each part is a module of its own. The plan's "Track D: contract" is the
//! reader's summary, ADRs 0033 (schedules, time, who a run acts as, the
//! starter told) and 0034 (webhooks) the reasons, and `docs/PROTOCOL.md`
//! the changelog. Nothing here does I/O: the node computes due times in a
//! zone (`vessel-node`'s `automation::time`), from the shapes parsed here.

mod cron;
mod hook;
mod preview;
mod prompt;
mod run;
mod schedule;
mod text;

pub use cron::{CronExpr, CronField, CronFields, InvalidCron};
pub use hook::{
    CreateHook, DeleteHook, DeliveryList, DeliveryOutcome, HookCreated, HookDefinition,
    HookDelivery, HookInfo, HookList, HookListener, HookSecret, InvalidHookSecret, ListDeliveries,
    ListHooks, Rejection, RotateHook, UpdateHook,
};
pub use preview::{PreviewCount, PreviewSchedule, SchedulePreview, SchedulePreviewError};
pub use prompt::{CreatePrompt, DeletePrompt, ListPrompts, PromptList, SavedPrompt, UpdatePrompt};
pub use run::{
    ListRuns, RunArchive, RunEnd, RunError, RunErrorCode, RunList, RunNumber, RunOutcome, RunTrigger,
    ScheduleRun, SkipReason,
};
pub use schedule::{
    AfterRun, Cadence, CatchUp, CreateSchedule, DeleteSchedule, EndReason, EverySeconds,
    GetSchedule, HEARTBEAT_CADENCE_PROTOCOL, ListSchedules, MaxRuns, OutOfRange, PauseSchedule,
    ResumeSchedule, RunSchedule, SCHEDULE_ISOLATION_PROTOCOL, ScheduleDefinition, ScheduleInfo, ScheduleList, ScheduleState,
    ScheduleTarget, UpdateSchedule, UpdateScheduleCadence, serves_schedule_isolation,
};
pub use text::{
    AutomationText, DeliveryKey, InvalidAutomationText, PromptName, PromptText, ScheduleName,
    TimeZoneName,
};
