//! A consistent prompt projection from retained records; no writer/schema work.
//! A read snapshot includes rewind markers so superseded prompts never reappear.

mod fold;

use rusqlite::{OptionalExtension as _, params};
use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexPage};

use crate::StoreError;
use crate::codec::{record_from_row, seq_from_sql};
use crate::reader::Reader;

/// Blocking scans retain admission even if their async caller disconnects.
pub(crate) const MAX_READS: usize = 2;

impl Reader {
    pub(crate) fn prompt_index(
        &self,
        request: &ListSessionPrompts,
    ) -> Result<PromptIndexPage, StoreError> {
        self.with(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let session = request.session;
            let first = transaction
                .prepare_cached("SELECT first_seq FROM sessions WHERE id = ?1")?
                .query_row([session.to_string()], |row| row.get::<_, i64>(0))
                .optional()?
                .ok_or(StoreError::UnknownSession(session))?;
            let mut statement = transaction.prepare_cached(
                "SELECT seq, at, event FROM events WHERE session_id = ?1 AND seq >= ?2 ORDER BY seq"
            )?;
            let mut rows = statement.query(params![session.to_string(), first])?;
            let mut index = fold::Index::default();
            while let Some(row) = rows.next()? {
                index.take(&record_from_row(session, row)?.0)?;
            }
            index.page(seq_from_sql(first)?, request)
        })
    }
}
