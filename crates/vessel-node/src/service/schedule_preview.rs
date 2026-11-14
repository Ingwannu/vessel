//! A preview uses the scheduler's calculation without entering its actor.
//! Four permits are acquired before spawning and held inside the closure:
//! canceling an RPC cannot free a slot while its blocking work still runs.

use chrono::{DateTime, Utc};
use vessel_proto::automation::{Cadence, PreviewSchedule, SchedulePreview, SchedulePreviewError};
use vessel_proto::{ErrorCode, ErrorDetail, WireError};

use super::{Connection, NodeService};
use crate::automation::time::{Due, NotComputable, Zones};

impl NodeService {
    pub(crate) async fn preview_schedule(
        &self,
        _: &Connection,
        request: PreviewSchedule,
    ) -> Result<SchedulePreview, WireError> {
        execute(std::sync::Arc::clone(&self.preview_slots), move || {
            compute(request, Utc::now()).map_err(refusal)
        })
        .await
    }
}

async fn execute<T: Send + 'static>(
    slots: std::sync::Arc<tokio::sync::Semaphore>,
    work: impl FnOnce() -> Result<T, WireError> + Send + 'static,
) -> Result<T, WireError> {
    let permit = slots.try_acquire_owned().map_err(|_busy| {
        WireError::new(
            ErrorCode::Unavailable,
            "schedule previews are busy; try again shortly",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "schedule preview worker failed");
        WireError::new(ErrorCode::Internal, "schedule preview failed; try again")
    })?
}

fn refusal(reason: SchedulePreviewError) -> WireError {
    let mut error = WireError::new(ErrorCode::InvalidRequest, reason.to_string());
    error.detail = Some(ErrorDetail::SchedulePreview { reason });
    error
}

fn supported_time(at: DateTime<Utc>) -> bool {
    i32::try_from(at.timestamp_subsec_nanos())
        .ok()
        .is_some_and(|nanos| jiff::Timestamp::new(at.timestamp(), nanos).is_ok())
}

fn compute(
    request: PreviewSchedule,
    now: DateTime<Utc>,
) -> Result<SchedulePreview, SchedulePreviewError> {
    let after = request.after.unwrap_or(now);
    if !supported_time(after) {
        return Err(SchedulePreviewError::TimeOutOfRange);
    }
    let mut cadence = request.cadence;
    if let Cadence::Every { anchor, .. } = &mut cadence {
        if anchor.is_none() {
            *anchor = Some(after);
        }
        if anchor.is_some_and(|at| !supported_time(at)) {
            return Err(SchedulePreviewError::TimeOutOfRange);
        }
    }
    let due = Due::of(&cadence, after, &Zones::default()).map_err(|error| match error {
        NotComputable::Zone(_) => SchedulePreviewError::UnknownZone,
        NotComputable::Unknown => SchedulePreviewError::UnknownCadence,
    })?;
    let mut cursor = after;
    let mut runs = Vec::new();
    for _ in 0..request.count.get() {
        cursor = due
            .next_after(cursor)
            .ok_or(SchedulePreviewError::NoFutureRun)?;
        if !supported_time(cursor) {
            return Err(SchedulePreviewError::TimeOutOfRange);
        }
        runs.push(cursor);
    }
    Ok(SchedulePreview {
        after,
        cadence,
        runs,
    })
}

#[cfg(test)]
#[path = "schedule_preview_tests.rs"]
mod tests;
