//! Installer output is paged even after the process exits. A terminal
//! state on a nonempty page is not the end of the output (ADR 0032).
//!
//! [Decision Log]
//! - Purpose: keep all retained output, including a failure past page one.
//! - Constraints: the node keeps 1 MiB, pages 64 KiB; UTF-8 can straddle
//!   pages, and a completed process can still have pages to drain.
//! - Alternatives: stop on the first terminal state (loses later pages),
//!   decode each page (corrupts a split character), accumulate without a cap.
//! - Choice: continue to an empty terminal page; validate the cursor at
//!   the link edge; retain bounded bytes and decode the combined output.
//! - Cost: one final empty read after the process ends.

use std::borrow::Cow;
use vessel_proto::TerminalBytes;
use vessel_proto::agents::{InstallOutput, InstallState};

/// The node's output retention bound, mirrored defensively by the client.
pub const MAX_BYTES: usize = 1_048_576;

/// Where the next installer read begins, in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cursor(u64);

impl Cursor {
    /// First read of a new job.
    pub const START: Self = Self(0);
    /// Offset sent to the node.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Whether to wait for new output or drain what the completed job retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadNext {
    /// Still running; use the node's bounded long poll.
    Wait(Cursor),
    /// Process ended but this page had bytes; read the next page immediately.
    Drain(Cursor),
    /// Empty terminal page: no retained bytes remain.
    Finished(Completion),
}

/// A completed installer cannot still be running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// The process exited successfully.
    Succeeded,
    /// It failed, with an exit code when the node knows one.
    Failed(Option<i32>),
    /// The node stopped it at its time limit.
    TimedOut,
}

impl Completion {
    /// The completed job’s notice, chosen without a toolkit dependency.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::Succeeded => crate::alert::Variant::Success,
            Self::Failed(_) | Self::TimedOut => crate::alert::Variant::Error,
        }
    }

    /// The protocol's terminal state, for its localized sentence.
    #[must_use]
    pub const fn state(self) -> InstallState {
        match self {
            Self::Succeeded => InstallState::Succeeded,
            Self::Failed(code) => InstallState::Failed { code },
            Self::TimedOut => InstallState::TimedOut,
        }
    }
}

/// A malformed page must not cause a repeated read or duplicate output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OutputProblem {
    /// The node answered from a different offset.
    #[error("the install output starts at an unexpected byte")]
    Start,
    /// Its next cursor does not account for exactly these bytes.
    #[error("the install output has an inconsistent next byte")]
    Next,
}

/// Decide the next read, once at the link edge, before forwarding a page.
///
/// # Errors
/// [`OutputProblem`] if the node's cursors do not describe this page.
pub fn after_read(expected: Cursor, page: &InstallOutput) -> Result<ReadNext, OutputProblem> {
    if page.from != expected.get() {
        return Err(OutputProblem::Start);
    }
    let count = page
        .next
        .checked_sub(page.from)
        .ok_or(OutputProblem::Next)?;
    if usize::try_from(count).ok() != Some(page.bytes.bytes().len()) {
        return Err(OutputProblem::Next);
    }
    let next = Cursor(page.next);
    let completion = match page.state {
        InstallState::Running => return Ok(ReadNext::Wait(next)),
        InstallState::Succeeded => Completion::Succeeded,
        InstallState::Failed { code, .. } => Completion::Failed(code),
        InstallState::TimedOut => Completion::TimedOut,
    };
    if page.bytes.bytes().is_empty() {
        Ok(ReadNext::Finished(completion))
    } else {
        Ok(ReadNext::Drain(next))
    }
}

/// Whether any output was beyond the retained bound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Limit {
    /// All received output fits.
    #[default]
    Whole,
    /// The node or the client reached its retention limit.
    Clipped,
}

/// Bounded bytes, not separately decoded page strings.
#[derive(Debug, Default)]
pub struct Transcript {
    bytes: Vec<u8>,
    limit: Limit,
}

impl Transcript {
    /// Append a page that the link already validated.
    pub fn append(&mut self, bytes: &TerminalBytes) {
        let room = MAX_BYTES.saturating_sub(self.bytes.len());
        if bytes.bytes().len() > room {
            self.limit = Limit::Clipped;
        }
        self.bytes.extend(bytes.bytes().iter().take(room));
    }
    /// The node also stopped retaining later output.
    pub fn clipped_on_node(&mut self) {
        self.limit = Limit::Clipped;
    }
    /// Decode together so a character split across two pages remains intact.
    #[must_use]
    pub fn text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }
    /// Whether all output was retained.
    #[must_use]
    pub const fn limit(&self) -> Limit {
        self.limit
    }
    /// Whether an installer has printed anything yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(from: u64, text: &'static str, state: InstallState) -> InstallOutput {
        InstallOutput {
            from,
            next: from
                .checked_add(u64::try_from(text.len()).unwrap())
                .unwrap(),
            bytes: TerminalBytes::new(text.into()),
            truncated: false,
            state,
        }
    }
    #[test]
    fn drains_a_finished_job_and_a_failed_job_until_an_empty_page() {
        for completion in [
            Completion::Succeeded,
            Completion::Failed(Some(7)),
            Completion::Failed(None),
            Completion::TimedOut,
        ] {
            let expected = if completion == Completion::Succeeded {
                crate::alert::Variant::Success
            } else {
                crate::alert::Variant::Error
            };
            assert_eq!(completion.variant(), expected);
            let state = completion.state();
            let first = page(0, "first page", state);
            let ReadNext::Drain(next) = after_read(Cursor::START, &first).unwrap() else {
                panic!("drain")
            };
            let second = page(next.get(), "failure detail on the later page", state);
            let ReadNext::Drain(next) = after_read(next, &second).unwrap() else {
                panic!("drain")
            };
            assert_eq!(
                after_read(next, &page(next.get(), "", state)),
                Ok(ReadNext::Finished(completion))
            );
        }
        assert_eq!(
            after_read(Cursor::START, &page(0, "", InstallState::Running)),
            Ok(ReadNext::Wait(Cursor::START))
        );
    }
    #[test]
    fn rejects_replayed_and_nonadvancing_nonempty_pages() {
        let mut output = page(1, "duplicate", InstallState::Running);
        assert_eq!(
            after_read(Cursor::START, &output),
            Err(OutputProblem::Start)
        );
        output.from = 0;
        output.next = 0;
        assert_eq!(after_read(Cursor::START, &output), Err(OutputProblem::Next));
    }
    #[test]
    fn a_split_korean_character_is_decoded_after_combining_and_output_is_bounded() {
        let mut text = Transcript::default();
        text.append(&TerminalBytes::new(vec![0xea, 0xb0].into()));
        text.append(&TerminalBytes::new(vec![0x80].into()));
        assert_eq!(text.text(), "가");
        assert_eq!(text.limit(), Limit::Whole);
        text.append(&TerminalBytes::new(vec![b'x'; MAX_BYTES].into()));
        assert_eq!(text.bytes.len(), MAX_BYTES);
        assert_eq!(text.limit(), Limit::Clipped);
        text.append(&TerminalBytes::new(b"ignored".as_slice().into()));
        assert_eq!(text.bytes.len(), MAX_BYTES);
        let mut node_cut = Transcript::default();
        assert!(node_cut.is_empty());
        node_cut.clipped_on_node();
        assert_eq!(node_cut.limit(), Limit::Clipped);
    }
}
