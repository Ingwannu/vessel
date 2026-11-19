//! One installation's logs dialog (Paseo's `PluginLogsSheet`): read when it
//! opens and on Refresh; while a refresh is read the last answer stays.
//!
//! [Decision Log]
//! - Purpose: Paseo's sheet: "Loading logs…" before the first answer, the
//!   error with Refresh, "No plugin output yet", or each entry's head (its
//!   time and stream) over its message.
//! - Choice: Paseo's query states as one value ([`LogsView`]); the head as
//!   Paseo writes it (`{timestamp} {stream}`): the time as the node sends
//!   it (RFC 3339, UTC, as the page shows it), the stream's own name. The
//!   node's lifecycle lines are named "vessel" (the page's word; Paseo's
//!   first stream is the plugin's stdout, which here frames the helper's
//!   protocol).
//! - Cost: the head is not translated, as Paseo's is not: it is the
//!   entry's data.

use chrono::SecondsFormat;
use vessel_proto::plugins::{LogEntry, LogStream, PluginId, PluginLogs};

use super::{Problem, Reply};
use crate::devices::Ticket;

/// The logs of one installation, as the dialog holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logs {
    plugin: PluginId,
    asking: Option<Ticket>,
    read: Option<Result<PluginLogs, Problem>>,
}

/// What the dialog shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogsView<'a> {
    /// "Loading logs…": nothing read yet.
    Loading,
    /// "Unable to load plugin logs", why, and Refresh.
    Failed(&'a Problem),
    /// "No plugin output yet".
    Empty,
    /// The entries, oldest first.
    Entries(&'a [LogEntry]),
}

impl Logs {
    /// The logs of `plugin`, asked with `ticket`.
    pub(super) const fn asked(plugin: PluginId, ticket: Ticket) -> Self {
        Self {
            plugin,
            asking: Some(ticket),
            read: None,
        }
    }

    /// The installation they are of.
    #[must_use]
    pub const fn plugin(&self) -> &PluginId {
        &self.plugin
    }

    /// Whether they are being read (Refresh says "Refreshing…" and waits).
    #[must_use]
    pub const fn refreshing(&self) -> bool {
        self.asking.is_some()
    }

    /// Asked again with `ticket`.
    pub(super) fn ask(&mut self, ticket: Ticket) {
        self.asking = Some(ticket);
    }

    /// An answer: kept when it is for the latest ask.
    pub(super) fn answered(&mut self, ticket: Ticket, result: Result<Reply, Problem>) -> bool {
        if self.asking != Some(ticket) {
            return false;
        }
        self.asking = None;
        self.read = Some(result.and_then(|reply| {
            if let Reply::Logs(logs) = reply {
                Ok(logs)
            } else {
                Err(Problem::Refused {
                    code: vessel_proto::ErrorCode::Internal,
                    message: None,
                })
            }
        }));
        true
    }

    /// What the dialog shows now.
    #[must_use]
    pub fn view(&self) -> LogsView<'_> {
        match self.read.as_ref() {
            None => LogsView::Loading,
            Some(Err(problem)) => LogsView::Failed(problem),
            Some(Ok(logs)) if logs.entries.is_empty() => LogsView::Empty,
            Some(Ok(logs)) => LogsView::Entries(&logs.entries),
        }
    }
}

/// An entry's head: its time and its stream (Paseo's
/// `{entry.timestamp} {entry.stream}`).
#[must_use]
pub fn log_head(entry: &LogEntry) -> String {
    let stream = match entry.stream {
        LogStream::Host => "vessel",
        LogStream::Stderr => "stderr",
    };
    format!(
        "{} {stream}",
        entry.at.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    )
}
