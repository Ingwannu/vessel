//! Retained log ownership is independent of one process generation.

use std::sync::{Arc, Mutex};

use chrono::Utc;
use vessel_proto::plugins::{HelperToNode, LogEntry, LogStream, LogTruncation};

use crate::{
    log_tail::{LogTail, StderrLines},
    runtime_error::RuntimeError,
};

/// Retain across reload/disable/failure; only explicit remove discards this
/// installation's store. The node must check human management authorization
/// before returning snapshot text; neither catalogs nor callback errors use it.
#[derive(Debug, Clone, Default)]
pub struct SharedLogs(Arc<Mutex<LogTail>>);

impl SharedLogs {
    /// Management-only retained text, oldest first.
    ///
    /// # Errors
    /// Refuses an unavailable store; never logs the guard's private content.
    pub fn snapshot(&self) -> Result<Vec<LogEntry>, RuntimeError> {
        self.0
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
        let mut tail = self.0.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        let _sequence = tail
            .push(
                Utc::now(),
                LogStream::Stderr,
                diagnostic.clone(),
                LogTruncation::Complete,
            )
            .map_err(RuntimeError::LogIdentity)?;
        Ok(())
    }

    pub(crate) fn stderr(
        &self,
        parser: &mut StderrLines,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        let mut tail = self.0.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        parser
            .feed(bytes, Utc::now(), &mut tail)
            .map_err(RuntimeError::LogIdentity)
    }

    pub(crate) fn finish_stderr(&self, parser: &mut StderrLines) -> Result<(), RuntimeError> {
        let mut tail = self.0.lock().map_err(|_poisoned| RuntimeError::Logs)?;
        parser
            .finish(Utc::now(), &mut tail)
            .map_err(RuntimeError::LogIdentity)
    }
}
