//! Bounded latest plans and newest-first transcript reads (ADR 0045).
//! Plans are projected in the same commit as their events: an output-heavy
//! turn or retention must not make the current plan disappear from context.

use rusqlite::{OptionalExtension as _, params};
use serde::{Deserialize, Serialize};
use vessel_proto::side_questions::{PLAN_STEPS, TEXT_CHARS, TRANSCRIPT_RECORDS};
use vessel_proto::{EventRecord, EventSeq, PlanEntry, SessionId};

use crate::codec::{decode, record_from_row, seq_to_sql};
use crate::reader::Reader;
use crate::{Column, Store, StoreError};

/// A plan projected independently of the retained conversation tail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestPlan {
    /// At most twenty steps, each at most 1,200 characters.
    pub entries: Vec<PlanEntry>,
    /// The original plan had more steps or longer text.
    pub truncated: bool,
}

impl LatestPlan {
    pub(crate) fn bounded(entries: &[PlanEntry]) -> Self {
        let mut truncated = entries.len() > PLAN_STEPS;
        let entries = entries
            .iter()
            .take(PLAN_STEPS)
            .map(|entry| {
                let content: String = entry.content.chars().take(TEXT_CHARS).collect();
                truncated |= content.len() < entry.content.len();
                PlanEntry {
                    content,
                    priority: entry.priority,
                    status: entry.status,

                    id: None,
                    active_form: None,
                }
            })
            .collect();
        Self { entries, truncated }
    }
}

/// Parent context read on a blocking reader, with finite record and byte
/// budgets. Records are newest first, so a large older event cannot hide
/// a newer message. A single event uses the existing store page allowance.
#[derive(Debug)]
pub struct SideContext {
    /// None until recorded on this schema version, or after a rewind.
    pub plan: Option<LatestPlan>,
    /// At most 128 events, bounded also by the store's page byte budget.
    pub records: Vec<EventRecord>,
}

impl Store {
    /// A bounded context of `session`, through the observed log cursor.
    /// The node waits for that cursor to be durable before calling.
    ///
    /// # Errors
    /// Unknown session, corrupt data, or a reader failure.
    pub async fn side_context(
        &self,
        session: SessionId,
        through: EventSeq,
    ) -> Result<SideContext, StoreError> {
        self.read(move |reader| reader.side_context(session, through))
            .await
    }
}

impl Reader {
    fn side_context(
        &self,
        session: SessionId,
        through: EventSeq,
    ) -> Result<SideContext, StoreError> {
        self.with(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let id = session.to_string();
            let text: Option<Option<String>> = transaction
                .prepare_cached("SELECT latest_plan FROM sessions WHERE id = ?1")?
                .query_row([&id], |row| row.get(0))
                .optional()?;
            let plan = text
                .ok_or(StoreError::UnknownSession(session))?
                .map(|text| decode(session, Column::LatestPlan, &text))
                .transpose()?;
            let mut statement = transaction.prepare_cached(
                "SELECT seq, at, event FROM events WHERE session_id = ?1 AND seq < ?2
                 ORDER BY seq DESC LIMIT ?3",
            )?;
            let mut rows =
                statement.query(params![id, seq_to_sql(through)?, TRANSCRIPT_RECORDS])?;
            let mut records = Vec::new();
            let mut bytes = 0_usize;
            // Same allowance as normal paged reads. Decode at least one
            // existing event; an event already admitted by the log is the
            // largest individual allocation this operation may make.
            while bytes < self.page_bytes() {
                let Some(row) = rows.next()? else { break };
                let (record, size) = record_from_row(session, row)?;
                // A rewind leaves old records as audit history, including
                // turns no longer in the provider's active conversation.
                // A bounded tail cannot rebuild an arbitrary rewind chain;
                // use only newer messages and disclose the partial scope.
                if matches!(record.event, vessel_proto::SessionEvent::Rewound { .. }) {
                    break;
                }
                bytes = bytes.saturating_add(size);
                records.push(record);
            }
            Ok(SideContext { plan, records })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{PlanPriority, PlanStatus};

    #[test]
    fn a_plan_is_bounded_without_splitting_unicode() {
        let entry = PlanEntry {
            content: "한".repeat(1_201),
            priority: PlanPriority::High,
            status: PlanStatus::InProgress,

            id: None,
            active_form: None,
        };
        let plan = LatestPlan::bounded(&vec![entry; 21]);
        assert!(plan.truncated);
        assert_eq!(plan.entries.len(), 20);
        assert_eq!(plan.entries.first().unwrap().content, "한".repeat(1_200));
    }
}
