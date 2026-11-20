//! What a schedule means on the screen: the list, the detail, the form and
//! the preview. No drawing and no I/O. The node decides when a run happens
//! (ADR 0072); these types only decide which control is honest.

mod detail;
mod fields;
mod form;
mod list;
mod peer;
mod preview;
mod row;

pub use detail::{
    ArchiveOutcome, MissedRuns, RunRow, ShownOutcome, ShownTrigger, archive, run_row,
};
pub use fields::{
    AfterChoice, CadenceField, CapField, CronPreset, Draft, ExpiryField, FormJob, FormReview, Save,
    ScheduleFormError, TargetField, UnknownPiece,
};
pub use form::{new_cron_zone, opened_zone, review, run_request};
pub use list::{LoadFault, NodeNotice, NodeReport, SchedulePage, page};
pub use peer::{
    BlockedAct, BookName, PreviewLane, ReadyPeer, SchedulePeer, ScheduleReach, SessionDirectory,
    Ticket,
};
pub use preview::{PreviewPlan, plan, reusable};
pub use row::{
    MenuAction, MetaPart, PromptLine, RowOpen, RunHold, ScheduleBadge, ScheduleBucket,
    ScheduleKind, ScheduleRow, TitleSource, actions, badge, bucket, hold_of, kind, meta, open_of,
    title,
};

#[cfg(test)]
mod tests;
