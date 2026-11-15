//! Compatibility presentation uses original decoder boundaries, never a raw scanner.
use crate::queries::EscapeEffect;
use crate::{
    BytesTaken, FEED_BUDGET, Replies, ReplyDuty, Screen, ScreenError, ScrollbackLines, Size,
};

/// Security floor for an unfinished control token; refuses rather than leaking a prefix.
pub const MAX_CONTROL_TOKEN_BYTES: usize = 64 * 1024;
const MAX_PRESENTATION_BYTES: usize = MAX_CONTROL_TOKEN_BYTES + FEED_BUDGET;
const MAX_PRESENTATION_EVENTS: usize = FEED_BUDGET;

/// Complete presentation tokens; node-answered non-rendering requests are omitted.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Presentation(Vec<u8>);
impl Presentation {
    /// Borrow bytes for the existing output log, including an empty sequence record.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Byte position in this feed's presentation, independent of raw event sequence numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationOffset(usize);
impl PresentationOffset {
    /// Position in the returned presentation only.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// Withheld protocol candidates, never normal node replies or replay instructions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UnansweredReplies(Vec<u8>);
impl UnansweredReplies {
    /// Preserve for diagnostics/future delegation; this grants no responder authority.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
/// An unchanged legacy span; multiple raw clients may answer it as before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationEvent {
    /// A shared ESC ends an earlier prefix and starts a query; the node did not answer.
    AmbiguousOverlap {
        /// Inclusive beginning of the unchanged span.
        start: PresentationOffset,
        /// Exclusive end of the unchanged span.
        end: PresentationOffset,
        /// Bounded ordered candidates, withheld from the node writer even for frame-only views.
        candidates: UnansweredReplies,
    },
}
/// Progress and admitted output survive a later refusal; never re-feed the prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "publish admitted output and replies, then handle any refusal"]
pub struct PresentationFeed {
    /// Already interpreted prefix, including a byte whose reply allocation failed.
    pub consumed: BytesTaken,
    /// Existing raw clients receive only this presentation, never original query bytes.
    pub presentation: Presentation,
    /// Ordered node replies, independent of the presentation log.
    pub replies: Replies,
    /// Typed exceptions carry offsets and whitelisted candidates, never query text.
    pub events: Vec<PresentationEvent>,
    /// A refusal stops interpretation at the reported prefix; the owner must fence it.
    pub outcome: Result<(), ScreenError>,
}

pub(crate) struct DecodedByte {
    pub(crate) step: vt100::DecoderStep,
    pub(crate) executed: Option<u8>,
    pub(crate) escape: Option<EscapeEffect>,
    pub(crate) replies: Result<Replies, ScreenError>,
}
#[derive(Debug, Default)]
enum Overlap {
    #[default]
    None,
    QueryUnanswered,
}
#[derive(Clone, Copy)]
enum Token {
    Preserve,
    Answered,
}

/// One node authority; bulk raw feed cannot bypass its presentation boundary.
#[derive(Debug)]
pub struct Multiplexer {
    screen: Screen,
    pending: Vec<u8>,
    // Canceled prefixes share the next ESC; retain them until its token settles.
    deferred: Vec<u8>,
    controls: Vec<usize>,
    overlap: Overlap,
    candidates: UnansweredReplies,
    string_end: Option<EscapeEffect>,
    failure: Option<ScreenError>,
}
impl Multiplexer {
    /// Allocate an authority separately from silent capture and display observers.
    ///
    /// # Errors
    /// Refuses the same cell admission failures as [`Screen::new`].
    pub fn new(size: Size, scrollback: ScrollbackLines) -> Result<Self, ScreenError> {
        Ok(Self {
            screen: Screen::with_replies(size, scrollback, ReplyDuty::NodeAuthority)?,
            pending: Vec::new(),
            deferred: Vec::new(),
            controls: Vec::new(),
            overlap: Overlap::None,
            candidates: UnansweredReplies::default(),
            string_end: None,
            failure: None,
        })
    }
    /// Read-only cells and capture share the authority; no raw parser escape hatch.
    #[must_use]
    pub const fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Resize the same authority without interpreting synthetic terminal bytes.
    ///
    /// # Errors
    /// Refuses retained-cell geometry or a previously fenced presentation.
    pub fn resize(&mut self, size: Size) -> Result<(), ScreenError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.screen.resize(size)
    }
    /// Apply the validated capacity to the same retained grid.
    ///
    /// # Errors
    /// Refuses retained-cell geometry or a previously fenced presentation.
    pub fn set_scrollback(&mut self, limit: ScrollbackLines) -> Result<(), ScreenError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.screen.set_scrollback(limit)
    }
    /// Pin the same grid without exposing a mutable bulk-feed parser.
    ///
    /// # Errors
    /// Refuses a fenced authority or the screen's validated marker limits.
    pub fn create_marker(
        &mut self,
        row: crate::LineIndex,
    ) -> Result<crate::ViewportAnchor, ScreenError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.screen.create_marker(row)
    }
    /// Replace an owned active marker without allocating another marker slot.
    ///
    /// # Errors
    /// Refuses a fenced authority, stale/inactive handle or unavailable row.
    pub fn replace_marker(
        &mut self,
        old: &crate::ViewportAnchor,
        row: crate::LineIndex,
    ) -> Result<crate::ViewportAnchor, ScreenError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.screen.replace_marker(old, row)
    }
    /// Cleanup remains available even after output admission has been fenced.
    pub fn release_marker(&mut self, anchor: &crate::ViewportAnchor) -> crate::AnchorRelease {
        self.screen.release_marker(anchor)
    }
    /// Interpret a bounded prefix, withholding unfinished controls across calls.
    pub fn feed(&mut self, bytes: &[u8]) -> PresentationFeed {
        let mut result = PresentationFeed {
            consumed: BytesTaken(0),
            presentation: Presentation::default(),
            replies: Replies::default(),
            events: Vec::new(),
            outcome: Ok(()),
        };
        if let Some(error) = self.failure {
            result.outcome = Err(error);
            return result;
        }
        for &byte in bytes.iter().take(FEED_BUDGET) {
            if let Err(error) = self.admit_byte() {
                result.outcome = Err(error);
                break;
            }
            let decoded = self.screen.feed_byte(byte);
            result.consumed.0 = result.consumed.0.saturating_add(1);
            if let Err(error) = self.present(byte, decoded, &mut result) {
                result.outcome = Err(error);
                break;
            }
        }
        self.failure = result.outcome.err();
        result
    }
    fn admit_byte(&mut self) -> Result<(), ScreenError> {
        if self.pending.len().saturating_add(self.deferred.len()) >= MAX_CONTROL_TOKEN_BYTES
            || self.pending.try_reserve(1).is_err()
            || self.controls.try_reserve(1).is_err()
        {
            return Err(ScreenError::PresentationUnavailable);
        }
        Ok(())
    }
    fn present(
        &mut self,
        byte: u8,
        decoded: DecodedByte,
        result: &mut PresentationFeed,
    ) -> Result<(), ScreenError> {
        let replies = decoded.replies?;
        let token = if replies.as_bytes().is_empty() {
            Token::Preserve
        } else {
            Token::Answered
        };
        if decoded.step.prefix == vt100::PrefixEnd::Ended {
            self.end_prefix(token, &replies, result)?;
        } else if matches!(token, Token::Answered) && !self.deferred.is_empty() {
            self.defer(&replies)?;
        }
        if decoded.executed == Some(byte) {
            self.controls.push(self.pending.len());
        }
        self.pending.push(byte);
        if decoded.step.boundary == vt100::DecoderBoundary::Complete {
            let terminator = self.string_end == Some(EscapeEffect::StringTerminator)
                && decoded.escape == Some(EscapeEffect::StringTerminator);
            let final_token = if terminator { Token::Answered } else { token };
            self.finish(final_token, result)?;
            if decoded.step.prefix == vt100::PrefixEnd::Retained
                && matches!(self.overlap, Overlap::None)
            {
                append(
                    &mut result.replies.0,
                    replies.as_bytes(),
                    crate::MAX_REPLY_BYTES,
                )?;
            }
            self.overlap = Overlap::None;
        }
        Ok(())
    }
    fn end_prefix(
        &mut self,
        token: Token,
        replies: &Replies,
        result: &mut PresentationFeed,
    ) -> Result<(), ScreenError> {
        self.string_end = None;
        match token {
            Token::Answered if self.deferred.is_empty() => {
                self.finish(Token::Answered, result)?;
                append(
                    &mut result.replies.0,
                    replies.as_bytes(),
                    crate::MAX_REPLY_BYTES,
                )?;
                self.string_end = Some(EscapeEffect::StringTerminator);
            }
            Token::Preserve | Token::Answered => {
                if matches!(token, Token::Answered) {
                    self.defer(replies)?;
                }
                append(&mut self.deferred, &self.pending, MAX_CONTROL_TOKEN_BYTES)?;
                self.pending.clear();
                self.controls.clear();
            }
        }
        Ok(())
    }
    fn defer(&mut self, replies: &Replies) -> Result<(), ScreenError> {
        append(
            &mut self.candidates.0,
            replies.as_bytes(),
            crate::MAX_REPLY_BYTES,
        )?;
        self.overlap = Overlap::QueryUnanswered;
        Ok(())
    }
    fn finish(&mut self, token: Token, result: &mut PresentationFeed) -> Result<(), ScreenError> {
        if matches!(token, Token::Answered) && !self.deferred.is_empty() {
            self.overlap = Overlap::QueryUnanswered;
        }
        if matches!(self.overlap, Overlap::QueryUnanswered)
            && (result.events.len() >= MAX_PRESENTATION_EVENTS
                || result.events.try_reserve(1).is_err())
        {
            return Err(ScreenError::PresentationUnavailable);
        }
        let start = PresentationOffset(result.presentation.0.len());
        append(
            &mut result.presentation.0,
            &self.deferred,
            MAX_PRESENTATION_BYTES,
        )?;
        match token {
            Token::Preserve => append(
                &mut result.presentation.0,
                &self.pending,
                MAX_PRESENTATION_BYTES,
            )?,
            Token::Answered if matches!(self.overlap, Overlap::QueryUnanswered) => {
                append(
                    &mut result.presentation.0,
                    &self.pending,
                    MAX_PRESENTATION_BYTES,
                )?;
            }
            Token::Answered => {
                // Executed controls survive even when their enclosing request is omitted.
                for &index in &self.controls {
                    if let Some(&control) = self.pending.get(index) {
                        append(
                            &mut result.presentation.0,
                            &[control],
                            MAX_PRESENTATION_BYTES,
                        )?;
                    }
                }
            }
        }
        if matches!(self.overlap, Overlap::QueryUnanswered) {
            result.events.push(PresentationEvent::AmbiguousOverlap {
                start,
                end: PresentationOffset(result.presentation.0.len()),
                candidates: std::mem::take(&mut self.candidates),
            });
        }
        self.pending.clear();
        self.deferred.clear();
        self.controls.clear();
        self.string_end = None;
        Ok(())
    }
}
fn append(target: &mut Vec<u8>, bytes: &[u8], maximum: usize) -> Result<(), ScreenError> {
    if target
        .len()
        .checked_add(bytes.len())
        .is_none_or(|next| next > maximum)
        || target.try_reserve(bytes.len()).is_err()
    {
        return Err(ScreenError::PresentationUnavailable);
    }
    target.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests;
