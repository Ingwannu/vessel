//! Node-only projection. Private Ready construction prevents a renderer from
//! installing arbitrary timestamps as a checked preview. No cron evaluator here.
//!
//! [Decision Log]
//! - Purpose: show three runs for exactly the rule and physical connection edited.
//! - Existing seam: schedule.preview returns a canonical interval anchor.
//! - Choice: immutable request, private outcome, strict envelope/count/order and
//!   cadence validation; expose canonical cadence only for matching current input.
//! - Alternative: another local cron evaluator or accepting a partial response.
//! - Why/cost: the node stays authoritative; malformed/older/refused previews show
//!   no guessed times. The form must invalidate on edit, host change and close.
use crate::connection::ConnectionContext;
use crate::localization::Translator;
use crate::schedules::load::{Failure, Ticket};
use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    Cadence, OutOfRange, PreviewCount, PreviewSchedule, SchedulePreview, SchedulePreviewError,
};

/// The editor's next-three-runs count, independent of the wire's default five.
pub const NEXT_RUNS: u32 = 3;
/// One immutable projection for an original physical connection/form edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    connection: ConnectionContext,
    ticket: Ticket,
    input: PreviewSchedule,
}
impl Request {
    /// Use one explicit instant so response after/anchor can be checked exactly.
    /// # Errors
    /// Retains a bad configured count instead of panicking or inventing a run.
    pub fn on(
        connection: ConnectionContext,
        ticket: Ticket,
        cadence: Cadence,
        after: DateTime<Utc>,
    ) -> Result<Self, OutOfRange> {
        Ok(Self {
            connection,
            ticket,
            input: PreviewSchedule {
                cadence,
                after: Some(after),
                count: PreviewCount::try_from(NEXT_RUNS)?,
            },
        })
    }
    /// The actor selects/fences this context; no later alias resolution.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }
    /// Correlation for this edit; a different visit must allocate another ticket.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
    /// Already parsed node request; no text parsing on dispatch or paint.
    #[must_use]
    pub const fn input(&self) -> &PreviewSchedule {
        &self.input
    }
}
/// Preview failures retain typed reasons; diagnostic Display is never UI copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Typed node reason from an InvalidRequest schedule-preview detail.
    Rule(SchedulePreviewError),
    /// Admission/transport/protocol/refusal from the closed shared mapper.
    Call(Failure),
    /// The node's response did not match its request or was partial/unordered.
    Mismatched,
}
impl Problem {
    fn of(failure: Failure) -> Self {
        if let Failure::Node(wire) = &failure
            && wire.code == vessel_proto::ErrorCode::InvalidRequest
            && let Some(vessel_proto::ErrorDetail::SchedulePreview { reason }) = &wire.detail
        {
            return Self::Rule(*reason);
        }
        Self::Call(failure)
    }
    /// One whole sentence per typed reason, including unknown future reasons.
    #[must_use]
    pub fn label<'a>(&self, catalog: &'a Translator) -> &'a str {
        let words = catalog.schedules();
        match self {
            Self::Call(failure) => crate::schedules::locale::failure(failure, catalog),
            Self::Mismatched => words.preview_mismatched(),
            Self::Rule(reason) => match reason {
                SchedulePreviewError::UnknownZone => words.preview_unknown_zone(),
                SchedulePreviewError::UnknownCadence => words.preview_unknown_cadence(),
                SchedulePreviewError::NoFutureRun => words.preview_no_future_run(),
                SchedulePreviewError::TimeOutOfRange => words.preview_out_of_range(),
                SchedulePreviewError::Unknown => words.preview_unknown(),
            },
        }
    }
}
#[derive(Debug, Default)]
enum Outcome {
    #[default]
    Idle,
    Waiting(Request),
    Ready {
        request: Request,
        value: SchedulePreview,
    },
    Failed(Problem),
}
/// One form edit's preview. Only settle may construct Ready.
#[derive(Debug, Default)]
pub struct State(Outcome);
impl State {
    /// An edit/host/visit change retires old times immediately.
    pub fn invalidate(&mut self) {
        self.0 = Outcome::Idle;
    }
    /// Sending a new valid rule removes all previously displayed times.
    pub fn begin(&mut self, request: Request) {
        self.0 = Outcome::Waiting(request);
    }
    /// Admit only the complete result for the expected immutable request.
    #[must_use]
    pub fn settle(&mut self, request: &Request, result: Result<SchedulePreview, Failure>) -> bool {
        if !matches!(&self.0, Outcome::Waiting(expected) if expected == request) {
            return false;
        }
        self.0 = match result {
            Ok(value) if valid(request, &value) => Outcome::Ready {
                request: request.clone(),
                value,
            },
            Ok(_) => Outcome::Failed(Problem::Mismatched),
            Err(failure) => Outcome::Failed(Problem::of(failure)),
        };
        true
    }
    /// No partial, older or failed times survive a changed input.
    #[must_use]
    pub fn times(&self) -> &[DateTime<Utc>] {
        match &self.0 {
            Outcome::Ready { value, .. } => &value.runs,
            Outcome::Idle | Outcome::Waiting(_) | Outcome::Failed(_) => &[],
        }
    }
    /// Save reuses only a matching node-returned anchor. Even if a renderer
    /// forgets invalidate, a different context or cadence cannot reuse it.
    #[must_use]
    pub fn canonical_for(
        &self,
        connection: &ConnectionContext,
        cadence: &Cadence,
    ) -> Option<&Cadence> {
        match &self.0 {
            Outcome::Ready { request, value }
                if request.connection() == connection && &request.input.cadence == cadence =>
            {
                Some(&value.cadence)
            }
            Outcome::Ready { .. } | Outcome::Idle | Outcome::Waiting(_) | Outcome::Failed(_) => {
                None
            }
        }
    }
    /// Loading and failure are separate from an empty successful result.
    #[must_use]
    pub const fn pending(&self) -> bool {
        matches!(self.0, Outcome::Waiting(_))
    }
    /// Structured error meaning, never a raw node message.
    #[must_use]
    pub const fn problem(&self) -> Option<&Problem> {
        match &self.0 {
            Outcome::Failed(problem) => Some(problem),
            Outcome::Idle | Outcome::Waiting(_) | Outcome::Ready { .. } => None,
        }
    }
}
fn valid(request: &Request, value: &SchedulePreview) -> bool {
    valid_input(request.input(), value)
}
pub(super) fn valid_input(input: &PreviewSchedule, value: &SchedulePreview) -> bool {
    if input.after != Some(value.after)
        || u32::try_from(value.runs.len()) != Ok(input.count.get())
        || value
            .runs
            .iter()
            .try_fold(value.after, |previous, next| {
                (next > &previous).then_some(*next)
            })
            .is_none()
    {
        return false;
    }
    match &input.cadence {
        Cadence::Every {
            seconds: wanted,
            anchor,
        } => match &value.cadence {
            Cadence::Every {
                seconds,
                anchor: Some(resolved),
            } => wanted == seconds && anchor.unwrap_or(value.after) == *resolved,
            Cadence::Every { anchor: None, .. } | Cadence::Cron { .. } | Cadence::Unknown => false,
        },
        Cadence::Cron {
            expr: wanted,
            zone: wanted_zone,
        } => match &value.cadence {
            Cadence::Cron { expr, zone } => wanted == expr && wanted_zone == zone,
            Cadence::Every { .. } | Cadence::Unknown => false,
        },
        Cadence::Unknown => false,
    }
}
#[cfg(test)]
mod tests;
