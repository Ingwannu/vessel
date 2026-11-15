//! Retained log ownership is independent of one process generation.

use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::sync::watch;
use vessel_proto::plugins::{HelperToNode, LogEntry, LogSequence, LogStream, LogTruncation};

use crate::{
    log_tail::{LogTail, StderrLines},
    runtime_error::RuntimeError,
};

/// Retain across reload/disable/failure; only explicit remove discards this
/// installation's store. The node must check human management authorization
/// before returning snapshot text; neither catalogs nor callback errors use it.
#[derive(Debug, Clone)]
pub struct SharedLogs {
    tail: Arc<Mutex<LogTail>>,
    revision: watch::Sender<Option<LogSequence>>,
}

impl Default for SharedLogs {
    fn default() -> Self {
        let (revision, _initial) = watch::channel(None);
        Self {
            tail: Arc::new(Mutex::new(LogTail::default())),
            revision,
        }
    }
}

impl SharedLogs {
    /// Observe retained-tail changes without copying diagnostic text into an
    /// event channel. The node still authorizes every management snapshot.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Option<LogSequence>> {
        self.revision.subscribe()
    }

    /// Management-only retained text, oldest first.
    ///
    /// # Errors
    /// Refuses an unavailable store; never logs the guard's private content.
    pub fn snapshot(&self) -> Result<Vec<LogEntry>, RuntimeError> {
        self.tail
            .lock()
            .map(|tail| tail.snapshot())
            .map_err(|_poisoned| RuntimeError::Logs)
    }

    pub(crate) fn helper_diagnostic(&self, message: &HelperToNode) -> Result<(), RuntimeError> {
        let diagnostic = match message {
            HelperToNode::Error { diagnostic, .. } | HelperToNode::Fatal { diagnostic } => {
                diagnostic
            }
            HelperToNode::Ready { .. }
            | HelperToNode::Result { .. }
            | HelperToNode::ClientChanged { .. }
            | HelperToNode::HooksChanged { .. } => return Ok(()),
        };
        let mut tail = self.tail.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        let _sequence = tail
            .push(
                Utc::now(),
                LogStream::Stderr,
                diagnostic.clone(),
                LogTruncation::Complete,
            )
            .map_err(RuntimeError::LogIdentity)?;
        self.publish(&tail);
        Ok(())
    }

    pub(crate) fn stderr(
        &self,
        parser: &mut StderrLines,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        let mut tail = self.tail.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        parser
            .feed(bytes, Utc::now(), &mut tail)
            .map_err(RuntimeError::LogIdentity)?;
        self.publish(&tail);
        Ok(())
    }

    pub(crate) fn finish_stderr(&self, parser: &mut StderrLines) -> Result<(), RuntimeError> {
        let mut tail = self.tail.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        parser
            .finish(Utc::now(), &mut tail)
            .map_err(RuntimeError::LogIdentity)?;
        self.publish(&tail);
        Ok(())
    }

    fn publish(&self, tail: &LogTail) {
        let latest = tail.latest();
        let _changed = self.revision.send_if_modified(|current| {
            if *current == latest {
                return false;
            }
            *current = latest;
            true
        });
    }
}
