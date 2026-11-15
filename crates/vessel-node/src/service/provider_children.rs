//! Provider projections reuse the parent log, with non-queueing node admission.

use super::{Connection, NodeService, wire};
use vessel_proto::subagents::{
    ChildCursor, ChildScanBudget, ListProviderChildren, ProviderChildPage, ProviderChildren,
    ReadProviderChild, bounds,
};
use vessel_proto::{ErrorCode, EventSeq, SessionId, WireError};

mod projection;

/// Node-wide non-queueing admission: a slow disk cannot grow a per-session queue.
pub(super) const AT_ONCE: usize = 4;
/// Each durable read also retains the store's configured byte/page caps.
const READ_BATCH: u32 = 32;

impl NodeService {
    pub(crate) async fn list_provider_children(
        &self,
        _: &Connection,
        request: ListProviderChildren,
    ) -> Result<ProviderChildren, WireError> {
        self.provider_projection(request.session, None)
            .await?
            .list()
            .map_err(|_error| invalid_projection())
    }
    pub(crate) async fn read_provider_child(
        &self,
        _: &Connection,
        request: ReadProviderChild,
    ) -> Result<ProviderChildPage, WireError> {
        self.provider_projection(request.session, Some(request))
            .await?
            .page()
            .map_err(|_error| invalid_projection())
    }
    async fn provider_projection(
        &self,
        session: SessionId,
        request: Option<ReadProviderChild>,
    ) -> Result<projection::Projection, WireError> {
        let _place = self
            .child_reads
            .try_acquire()
            .map_err(|_error| unavailable())?;
        let deadline = tokio::time::Instant::now()
            .checked_add(bounds::READ_DEADLINE)
            .ok_or_else(unavailable)?;
        let info = tokio::time::timeout_at(deadline, self.sessions.get(session))
            .await
            .map_err(|_error| unavailable())?
            .map_err(|error| wire(&error))?;
        let end = info.next_seq;
        let (start, upper) = window(request.as_ref().and_then(|r| r.cursor), end)?;
        let projection = projection::Projection::new(request, start, end);
        self.scan_children(session, projection, upper, end, deadline)
            .await
    }

    async fn scan_children(
        &self,
        session: SessionId,
        mut projection: projection::Projection,
        upper: EventSeq,
        end: EventSeq,
        deadline: tokio::time::Instant,
    ) -> Result<projection::Projection, WireError> {
        let mut budget = ChildScanBudget::default();
        loop {
            let from = projection.next();
            if from >= upper {
                projection.finish(upper == end);
                break;
            }
            let page = match tokio::time::timeout_at(
                deadline,
                self.sessions.read(session, from, READ_BATCH),
            )
            .await
            {
                Ok(page) => page.map_err(|error| wire(&error))?,
                Err(_) if projection.scanned() => {
                    projection.truncated();
                    break;
                }
                Err(_) => return Err(unavailable()),
            };
            if !observe_page(&mut projection, &mut budget, page.records, upper, deadline)? {
                break;
            }
            if page.at_end {
                projection.finish(upper == end);
                break;
            }
            if projection.next() <= from {
                projection.truncated();
                break;
            }
        }
        Ok(projection)
    }
}
fn unavailable() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "provider child history is busy or its read deadline elapsed",
    )
}
fn invalid_projection() -> WireError {
    WireError::new(
        ErrorCode::Internal,
        "provider child history could not be projected within its bounds",
    )
}

fn window(cursor: Option<ChildCursor>, end: EventSeq) -> Result<(EventSeq, EventSeq), WireError> {
    let upper = match cursor {
        Some(ChildCursor::Before(seq)) => seq.min(end),
        Some(ChildCursor::After(_)) | None => end,
    };
    let records = u64::try_from(bounds::MAX_SCAN_RECORDS).map_err(|_error| invalid_projection())?;
    let start = match cursor {
        Some(ChildCursor::After(seq)) if seq > end => {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                "provider cursor is ahead of the parent log",
            ));
        }
        // A continuation still needs retained descriptors/support before the
        // cursor. An empty new range must not claim the child disappeared.
        Some(ChildCursor::After(seq)) => {
            seq.min(EventSeq::new(upper.get().saturating_sub(records)))
        }
        Some(ChildCursor::Before(_)) | None => EventSeq::new(upper.get().saturating_sub(records)),
    };
    Ok((start, upper))
}
fn observe_page(
    projection: &mut projection::Projection,
    budget: &mut ChildScanBudget,
    records: Vec<vessel_proto::EventRecord>,
    upper: EventSeq,
    deadline: tokio::time::Instant,
) -> Result<bool, WireError> {
    for record in records {
        if tokio::time::Instant::now() >= deadline {
            if !projection.scanned() {
                return Err(unavailable());
            }
            projection.truncated();
            return Ok(false);
        }
        if record.seq >= upper {
            return Ok(false);
        }
        if budget.admit(&record).is_err() {
            projection.refused_record(record.seq);
            return Ok(false);
        }
        if !projection.observe(record) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
