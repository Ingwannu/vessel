//! One bounded tail survives process reload; management alone may expose text.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use vessel_proto::plugins::{
    LogEntry, LogSequence, LogStream, LogText, LogTruncation, MAX_LOG_BYTES, MAX_LOG_ENTRIES,
    MAX_LOG_LINE_BYTES,
};

/// Host-owned tail identity must never wrap and alias an earlier log row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("plugin log sequence exhausted")]
pub struct LogExhausted;

/// Pure tail data; this does not decide who may read private diagnostics.
#[derive(Debug, Default)]
pub struct LogTail {
    entries: VecDeque<LogEntry>,
    bytes: usize,
    sequence: u64,
}

impl LogTail {
    /// Cursor publication contains no private text and never uses zero as a row.
    pub(crate) fn latest(&self) -> Option<LogSequence> {
        (self.sequence != 0).then(|| LogSequence::new(self.sequence))
    }

    /// Bounded current tail, oldest first. Only authorized management code may
    /// serialize the returned entries or call their `LogText::as_str` accessors.
    #[must_use]
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.iter().cloned().collect()
    }

    /// Add one bounded line using host receipt time and metadata. Old entries
    /// leave only to enforce the reference's count and total-byte retention.
    ///
    /// # Errors
    /// Refuses counter exhaustion; callers retire the log producer rather than
    /// reusing a sequence or pretending it was successfully retained.
    pub fn push(
        &mut self,
        at: DateTime<Utc>,
        stream: LogStream,
        message: LogText,
        truncation: LogTruncation,
    ) -> Result<LogSequence, LogExhausted> {
        let sequence = self.sequence.checked_add(1).ok_or(LogExhausted)?;
        self.sequence = sequence;
        self.bytes = self.bytes.saturating_add(message.as_str().len());
        self.entries.push_back(LogEntry {
            sequence: LogSequence::new(sequence),
            at,
            stream,
            message,
            truncation,
        });
        while self.entries.len() > MAX_LOG_ENTRIES || self.bytes > MAX_LOG_BYTES {
            if let Some(old) = self.entries.pop_front() {
                self.bytes = self.bytes.saturating_sub(old.message.as_str().len());
            }
        }
        Ok(LogSequence::new(sequence))
    }
}

/// A helper may write an endless line; retain only the prefix while continuing
/// to drain bytes, so the child cannot turn stderr into an unbounded allocation.
#[derive(Default)]
pub(crate) struct StderrLines {
    prefix: Vec<u8>,
    truncation: Option<LogTruncation>,
}

impl std::fmt::Debug for StderrLines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StderrLines(<private diagnostic prefix>)")
    }
}

impl StderrLines {
    pub(crate) fn feed(
        &mut self,
        bytes: &[u8],
        at: DateTime<Utc>,
        tail: &mut LogTail,
    ) -> Result<(), LogExhausted> {
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            let content = part.strip_suffix(b"\n").unwrap_or(part);
            let available = MAX_LOG_LINE_BYTES.saturating_sub(self.prefix.len());
            let prefix = content.get(..available).unwrap_or(content);
            self.prefix.extend_from_slice(prefix);
            if prefix.len() < content.len() {
                self.truncation = Some(LogTruncation::PrefixRetained);
            }
            if part.ends_with(b"\n") {
                self.emit(at, tail)?;
            }
        }
        Ok(())
    }

    pub(crate) fn finish(
        &mut self,
        at: DateTime<Utc>,
        tail: &mut LogTail,
    ) -> Result<(), LogExhausted> {
        if !self.prefix.is_empty() || self.truncation.is_some() {
            self.emit(at, tail)?;
        }
        Ok(())
    }

    /// A cancelled reader has no EOF proof. Retain captured bytes but never
    /// label its unterminated prefix as a complete diagnostic line.
    pub(crate) fn interrupted(
        &mut self,
        at: DateTime<Utc>,
        tail: &mut LogTail,
    ) -> Result<(), LogExhausted> {
        if !self.prefix.is_empty() {
            self.truncation = Some(LogTruncation::PrefixRetained);
        }
        self.finish(at, tail)
    }

    fn emit(&mut self, at: DateTime<Utc>, tail: &mut LogTail) -> Result<(), LogExhausted> {
        let (message, captured) = LogText::capture(&self.prefix);
        let truncation = self.truncation.take().unwrap_or(captured);
        self.prefix.clear();
        let _sequence = tail.push(at, LogStream::Stderr, message, truncation)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endless_split_lines_remain_bounded_and_keep_the_final_partial_line() {
        let mut parser = StderrLines::default();
        let mut tail = LogTail::default();
        let at = Utc::now();
        for _ in 0..100 {
            parser
                .feed(&vec![b'x'; MAX_LOG_LINE_BYTES], at, &mut tail)
                .expect("feed");
        }
        assert_eq!(parser.prefix.len(), MAX_LOG_LINE_BYTES);
        assert!(tail.entries.is_empty());
        parser.feed(b"\nlast", at, &mut tail).expect("newline");
        parser.finish(at, &mut tail).expect("finish");
        assert_eq!(tail.entries.len(), 2);
        assert_eq!(
            tail.entries.front().expect("first").truncation,
            LogTruncation::PrefixRetained
        );
        assert_eq!(tail.entries.back().expect("last").message.as_str(), "last");
    }

    #[test]
    fn both_retention_budgets_evict_oldest_without_resetting_identity() {
        let mut tail = LogTail::default();
        let (text, truncated) = LogText::capture(&vec![b'x'; MAX_LOG_LINE_BYTES]);
        for _ in 0..MAX_LOG_ENTRIES.saturating_add(1) {
            let _seq = tail
                .push(Utc::now(), LogStream::Host, text.clone(), truncated)
                .expect("push");
        }
        assert!(tail.entries.len() < MAX_LOG_ENTRIES);
        assert!(tail.bytes <= MAX_LOG_BYTES);
        assert_eq!(
            tail.entries.back().expect("tail").sequence.get(),
            u64::try_from(MAX_LOG_ENTRIES.saturating_add(1)).expect("count")
        );
        tail.sequence = u64::MAX;
        assert!(
            tail.push(Utc::now(), LogStream::Host, text, truncated)
                .is_err()
        );
    }
}
