//! Only the PTY-owning actor may answer the reference server's query whitelist.
use crate::{FEED_BUDGET, ScreenError};
use std::fmt::Write as _;

/// A mirrored screen must never send another actor's terminal replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplyDuty {
    /// Native observers and retained snapshots only interpret output.
    #[default]
    Silent,
    /// The one node actor returns replies to its existing ordered PTY writer.
    NodeAuthority,
}

// The longest whitelisted response is an OSC color response. A feed may finish
// one query begun in an earlier feed, then complete at most one query per byte.
const MAX_QUERY_REPLY_BYTES: usize = 25;
/// Allocation ceiling derived from the bounded parser input, including a carried query.
pub const MAX_REPLY_BYTES: usize = (FEED_BUDGET + 1) * MAX_QUERY_REPLY_BYTES;

/// Ordered protocol bytes, with no terminal text or clipboard contents mixed in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Replies(Vec<u8>);
impl Replies {
    /// Borrow bytes for the actor's existing ordered writer.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) struct Queries {
    duty: ReplyDuty,
    bytes: String,
    failure: Option<ScreenError>,
    budget: usize,
}
impl Queries {
    pub(crate) fn new(duty: ReplyDuty) -> Self {
        Self {
            duty,
            bytes: String::new(),
            failure: None,
            budget: 0,
        }
    }
    pub(crate) fn begin(&mut self, bytes: usize) {
        self.budget = bytes
            .saturating_add(1)
            .saturating_mul(MAX_QUERY_REPLY_BYTES);
    }
    pub(crate) fn take(&mut self) -> Result<Replies, ScreenError> {
        let bytes = std::mem::take(&mut self.bytes).into_bytes();
        match self.failure.take() {
            Some(error) => Err(error),
            None => Ok(Replies(bytes)),
        }
    }
    fn cursor(&mut self, screen: &vt100::Screen, prefix: &str) {
        let (row, column) = screen.cursor_position();
        // Preserve the pending-wrap column, as the server's explicit handler does.
        let _bounded = write!(
            self,
            "\x1b[{prefix}{};{}R",
            u32::from(row).saturating_add(1),
            u32::from(column).saturating_add(1)
        );
    }
}
impl std::fmt::Write for Queries {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.failure.is_some() {
            return Err(std::fmt::Error);
        }
        let next = self.bytes.len().checked_add(text.len());
        if next.is_none_or(|next| next > self.budget || next > MAX_REPLY_BYTES)
            || self.bytes.try_reserve_exact(text.len()).is_err()
        {
            self.failure = Some(ScreenError::ReplyUnavailable);
            return Err(std::fmt::Error);
        }
        self.bytes.push_str(text);
        Ok(())
    }
}
impl vt100::Callbacks for Queries {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        first: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        final_byte: char,
    ) {
        if self.duty == ReplyDuty::Silent || second.is_some() {
            return;
        }
        let parameter = match params {
            [] => 0,
            [&[value]] => value,
            _ => return,
        };
        match (first, final_byte, parameter) {
            (None, 'c', 0) => {
                let _bounded = self.write_str("\x1b[?62;4;22c");
            }
            (None, 'n', 5) => {
                let _bounded = self.write_str("\x1b[0n");
            }
            (None, 'n', 6) => self.cursor(screen, ""),
            (Some(b'?'), 'n', 6) => self.cursor(screen, "?"),
            _ => {}
        }
    }
    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        if self.duty == ReplyDuty::Silent {
            return;
        }
        let [code, data] = params else {
            return;
        };
        let code = code.iter().try_fold(0_u16, |value, byte| {
            byte.is_ascii_digit().then_some(())?;
            value
                .checked_mul(10)?
                .checked_add(u16::from(byte.checked_sub(b'0')?))
        });
        if std::str::from_utf8(data)
            .ok()
            .map(|text| text.trim_matches(crate::projection::js_whitespace))
            != Some("?")
        {
            return;
        }
        let response = match code {
            Some(10) => "\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\",
            Some(11) => "\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\",
            Some(12) => "\x1b]12;rgb:e6e6/e6e6/e6e6\x1b\\",
            _ => return,
        };
        let _bounded = self.write_str(response);
    }
}

#[cfg(test)]
mod tests;
