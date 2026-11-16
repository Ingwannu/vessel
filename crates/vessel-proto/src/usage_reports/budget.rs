//! The complete report payload is refused before it can exhaust the RPC frame.
use serde::Serialize;
use std::{
    io::{self},
    sync::Arc,
};

/// Report JSON leaves room for the RPC envelope within the transport's frame cap.
pub const MAX_USAGE_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// A checked encoded size is a byte count, never an account/report count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodedReportBytes(usize);
impl EncodedReportBytes {
    /// The validated encoded byte count for cache/admission accounting.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Serialization causes are retained; a byte refusal never includes report contents.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UsageReportBudgetError {
    /// No report is silently removed merely to fit the wire.
    #[error("the complete usage report exceeds {limit} encoded bytes", limit = MAX_USAGE_RESPONSE_BYTES)]
    TooLarge,
    /// Unexpected serializer failures keep their cause for internal diagnostics.
    #[error("the usage report could not be encoded")]
    Encoding(#[source] Arc<serde_json::Error>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BudgetState {
    Open,
    Exceeded,
}
struct Counter {
    written: usize,
    state: BudgetState,
}
impl io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Saturation must refuse rather than wrap the count and admit an oversized body.
        let next = self.written.saturating_add(bytes.len());
        if next > MAX_USAGE_RESPONSE_BYTES {
            self.state = BudgetState::Exceeded;
            return Err(io::Error::other("usage report byte budget exceeded"));
        }
        self.written = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encoded<T: Serialize>(value: &T) -> Result<EncodedReportBytes, UsageReportBudgetError> {
    let mut counter = Counter {
        written: 0,
        state: BudgetState::Open,
    };
    let result = serde_json::to_writer(&mut counter, value);
    if counter.state == BudgetState::Exceeded {
        return Err(UsageReportBudgetError::TooLarge);
    }
    result.map_err(|source| UsageReportBudgetError::Encoding(Arc::new(source)))?;
    Ok(EncodedReportBytes(counter.written))
}
impl super::UsageReports {
    /// Validate the whole collection before publication, without allocating encoded JSON.
    /// # Errors
    /// An oversized response or serializer failure refuses the complete observation.
    pub fn validate_budget(&self) -> Result<EncodedReportBytes, UsageReportBudgetError> {
        encoded(self)
    }
}
impl super::UsageReportLookup {
    /// Inspect has the same complete-response budget as a list.
    /// # Errors
    /// An oversized report or serializer failure refuses the complete observation.
    pub fn validate_budget(&self) -> Result<EncodedReportBytes, UsageReportBudgetError> {
        encoded(self)
    }
}
