//! Actual thread/list metadata, with no inferred cwd or synthetic epoch timestamp.
use super::super::read_probe::{ProbeError, ReadRequest};
use crate::history::{HistoryEntry, HistoryFrom, HistoryTimestamp, MAX_ENTRIES};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vessel_proto::HarnessSessionId;

const MAX_NATIVE_ID_BYTES: usize = 512;
const MAX_CWD_BYTES: usize = 4096;
const MAX_TITLE_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListThreads<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<&'a str>,
    pub limit: usize,
    pub sort_key: SortKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<&'a Path>,
}
#[derive(Serialize)]
pub(super) enum SortKey {
    #[serde(rename = "updated_at")]
    Updated,
}
impl ReadRequest for ListThreads<'_> {
    type Response = ThreadPage;
    const METHOD: &'static str = "thread/list";
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadPage {
    pub data: Vec<Thread>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Thread {
    id: String,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preview: Option<String>,
    #[serde(default)]
    updated_at: Option<i64>,
    #[serde(default)]
    created_at: Option<i64>,
}
impl ThreadPage {
    pub(super) fn next(self_cursor: Option<String>) -> Result<Option<HistoryFrom>, ProbeError> {
        self_cursor
            .map(|cursor| {
                let next = HistoryFrom {
                    cursor: Some(cursor),
                    skip: 0,
                };
                if next.cursor.as_ref().is_none_or(String::is_empty)
                    || next.to_string().len()
                        > vessel_proto::history::GetAgentHistory::MAX_CURSOR_BYTES
                {
                    return Err(ProbeError::Invalid);
                }
                Ok(next)
            })
            .transpose()
    }
    pub(super) fn entries(
        self,
        cwd: Option<&Path>,
        skip: usize,
    ) -> Result<Vec<HistoryEntry>, ProbeError> {
        if self.data.len() > MAX_ENTRIES {
            return Err(ProbeError::Invalid);
        }
        let mut entries = Vec::new();
        for row in self.data.into_iter().skip(skip) {
            if row.id.is_empty()
                || row.id.len() > MAX_NATIVE_ID_BYTES
                || row.id.chars().any(char::is_control)
            {
                return Err(ProbeError::Invalid);
            }
            let Some(directory) = row.cwd.filter(|path| path.is_absolute()) else {
                continue;
            };
            if directory.as_os_str().len() > MAX_CWD_BYTES {
                return Err(ProbeError::Invalid);
            }
            if cwd.is_some_and(|wanted| wanted != directory.as_path()) {
                continue;
            }
            let title = row
                .name
                .filter(|name| !name.trim().is_empty())
                .or(row.preview);
            if title
                .as_ref()
                .is_some_and(|title| title.len() > MAX_TITLE_BYTES)
            {
                return Err(ProbeError::Invalid);
            }
            let updated_at = row
                .updated_at
                .or(row.created_at)
                .map(HistoryTimestamp::UnixSeconds);
            entries.push(HistoryEntry {
                session: HarnessSessionId::new(row.id),
                cwd: directory,
                title,
                updated_at,
            });
        }
        Ok(entries)
    }
}
