//! Read native history through the shared prompt-free probe, never provider files.
use super::read_probe::{self, ProbeError, Transaction};
use crate::history::{HistoryError, HistoryFrom, HistoryPage, MAX_ENTRIES, MAX_PAGES};
use std::{collections::HashSet, path::Path, time::Duration};
use vessel_proc::CommandSpec;
mod data;

pub(super) async fn list(
    command: &CommandSpec,
    cwd: Option<&Path>,
    from: HistoryFrom,
    limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    let observed =
        read_probe::run(command, limit, |transaction| read(transaction, cwd, from)).await;
    match observed {
        Ok(Some(page)) => Ok(page),
        Ok(None) => Err(HistoryError::CannotList),
        Err(ProbeError::TimedOut) => Err(HistoryError::TimedOut {
            limit,
            log: Vec::new(),
        }),
        Err(ProbeError::Spawn(error)) => Err(HistoryError::Spawn(error.into())),
        Err(ProbeError::Protocol | ProbeError::Invalid) => Err(HistoryError::Failed {
            message: "Codex history protocol or bounded metadata was invalid".into(),
            log: Vec::new(),
        }),
    }
}
async fn read(
    mut transaction: Transaction,
    cwd: Option<&Path>,
    from: HistoryFrom,
) -> Result<Option<HistoryPage>, ProbeError> {
    let _initialized = transaction.initialize().await?;
    let HistoryFrom {
        mut cursor,
        mut skip,
    } = from;
    let mut seen = HashSet::new();
    for _page in 0..MAX_PAGES {
        if !seen.insert(cursor.clone()) {
            return Err(ProbeError::Invalid);
        }
        let request = data::ListThreads {
            cursor: cursor.as_deref(),
            limit: MAX_ENTRIES,
            sort_key: data::SortKey::Updated,
            cwd,
        };
        let Some(page) = transaction.request(&request).await? else {
            return Ok(None);
        };
        let next = data::ThreadPage::next(page.next_cursor.clone())?;
        if page.next_cursor.is_some() && page.next_cursor == cursor {
            return Err(ProbeError::Invalid);
        }
        let entries = page.entries(cwd, skip)?;
        if !entries.is_empty() || next.is_none() {
            return Ok(Some(HistoryPage { entries, next }));
        }
        let Some(next) = next else {
            return Err(ProbeError::Invalid);
        };
        cursor = next.cursor;
        skip = next.skip;
    }
    Ok(Some(HistoryPage {
        entries: Vec::new(),
        next: Some(HistoryFrom { cursor, skip }),
    }))
}

#[cfg(test)]
mod tests;
