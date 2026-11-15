//! Metadata-only prompt navigation; no transcript bodies or store knowledge.
//! Pending admission keeps the new request off older or unassigned peers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{EventSeq, ProtocolVersion, SessionId};

/// Assigned only when client, node and outline consumers handle this contract.
pub const PROMPT_INDEX_PROTOCOL: Option<ProtocolVersion> = None;
/// UTF-16 units in a preview including its optional ellipsis (Paseo's limit).
pub const PREVIEW_UNITS: usize = 120;
/// Metadata entries per reply; paging keeps every frame bounded.
pub const PAGE_ENTRIES: u16 = 200;
/// Metadata retained during one snapshot projection; excess is a typed refusal.
pub const INDEX_ENTRIES: usize = 65_536;
/// Records decoded per snapshot; a retained log cannot monopolize readers.
pub const SCAN_RECORDS: usize = 1_000_000;

/// A bad preview or page is refused before it reaches a view or store operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPromptIndex {
    /// The peer sent text outside the canonical bounded preview form.
    #[error("a prompt preview must be normalized and at most {PREVIEW_UNITS} UTF-16 units")]
    Preview,
    /// A requested page is empty or exceeds the frame budget.
    #[error("a prompt index limit must be within 1..={PAGE_ENTRIES}")]
    Limit,
    /// A revision cannot name a rewind outside its retained window.
    #[error("a prompt-index rewind must not precede retained history")]
    Epoch,
    /// A response is too large, unsorted, or has an inconsistent continuation.
    #[error("prompt index entries must be ordered, bounded and match their continuation")]
    Page,
}

/// Canonical preview text, validated again when an untrusted peer sends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(transparent)]
pub struct PromptPreview(String);

impl PromptPreview {
    /// Collapse whitespace and truncate without splitting a Unicode scalar.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let mut preview = PreviewBuilder::default();
        preview.push(text);
        preview.finish()
    }

    /// Borrow the already normalized presentation text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PromptPreview {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let text = String::deserialize(decoder)?;
        let preview = Self::from_text(&text);
        if preview.0 != text {
            return Err(serde::de::Error::custom(InvalidPromptIndex::Preview));
        }
        Ok(preview)
    }
}

/// Incremental history chunks must not require buffering an entire prompt.
#[derive(Debug, Default)]
pub struct PreviewBuilder {
    text: String,
    units: usize,
    separator: Separator,
    extent: Extent,
}

#[derive(Debug, Default)]
enum Separator {
    #[default]
    Joined,
    Pending,
}
#[derive(Debug, Default)]
enum Extent {
    #[default]
    Exact,
    Truncated,
}

impl PreviewBuilder {
    /// Retain only a bounded prefix; metadata may separate history chunks.
    pub fn push(&mut self, text: &str) {
        for character in text.chars() {
            // ECMAScript whitespace adds BOM and excludes NEL from Rust's set.
            if (character.is_whitespace() && character != '\u{85}') || character == '\u{feff}' {
                if !self.text.is_empty() {
                    self.separator = Separator::Pending;
                }
                continue;
            }
            if matches!(self.separator, Separator::Pending) {
                self.append(' ');
                self.separator = Separator::Joined;
            }
            self.append(character);
        }
    }

    fn append(&mut self, character: char) {
        if matches!(self.extent, Extent::Truncated) {
            return;
        }
        // Saturation cannot undercount the bounded prefix on oversized input.
        let next = self.units.saturating_add(character.len_utf16());
        if next > PREVIEW_UNITS {
            self.extent = Extent::Truncated;
        } else {
            self.units = next;
            self.text.push(character);
        }
    }

    /// Finish a canonical value even when the last chunk ended with whitespace.
    #[must_use]
    pub fn finish(mut self) -> PromptPreview {
        if matches!(self.extent, Extent::Truncated) {
            while self.units >= PREVIEW_UNITS {
                let Some(character) = self.text.pop() else {
                    break;
                };
                self.units = self.units.saturating_sub(character.len_utf16());
            }
            self.text.push('…');
        }
        PromptPreview(self.text)
    }
}

/// A validated reply-size budget, independent of record-scan work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PromptIndexLimit(u16);

impl PromptIndexLimit {
    /// Refuse an invalid bound at the wire edge before opening a reader.
    /// # Errors
    /// The limit is outside the named page budget.
    pub fn new(limit: u16) -> Result<Self, InvalidPromptIndex> {
        if (1..=PAGE_ENTRIES).contains(&limit) {
            Ok(Self(limit))
        } else {
            Err(InvalidPromptIndex::Limit)
        }
    }
    /// Convert the checked budget for collection operations.
    #[must_use]
    pub fn count(self) -> usize {
        usize::from(self.0)
    }
}
impl Default for PromptIndexLimit {
    fn default() -> Self {
        Self(PAGE_ENTRIES)
    }
}
impl<'de> Deserialize<'de> for PromptIndexLimit {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::new(u16::deserialize(decoder)?).map_err(serde::de::Error::custom)
    }
}

/// Retention and rewinds change addressability; appends alone preserve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptIndexEpoch {
    retained_from: EventSeq,
    rewound_at: Option<EventSeq>,
}

impl PromptIndexEpoch {
    /// Reject inconsistent revisions at the same edge as a received reply.
    /// # Errors
    /// A rewind marker precedes the retained start.
    pub fn new(
        retained_from: EventSeq,
        rewound_at: Option<EventSeq>,
    ) -> Result<Self, InvalidPromptIndex> {
        if rewound_at.is_some_and(|seq| seq < retained_from) {
            return Err(InvalidPromptIndex::Epoch);
        }
        Ok(Self {
            retained_from,
            rewound_at,
        })
    }
    /// A retained-prefix change invalidates a previous page's addresses.
    #[must_use]
    pub const fn retained_from(self) -> EventSeq {
        self.retained_from
    }
    /// A rewind invalidates prompt addresses even when retention did not move.
    #[must_use]
    pub const fn rewound_at(self) -> Option<EventSeq> {
        self.rewound_at
    }
}

impl<'de> Deserialize<'de> for PromptIndexEpoch {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            retained_from: EventSeq,
            rewound_at: Option<EventSeq>,
        }
        let wire = Wire::deserialize(decoder)?;
        Self::new(wire.retained_from, wire.rewound_at).map_err(serde::de::Error::custom)
    }
}

/// Proposed Read request; registration waits for the shared pending table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListSessionPrompts {
    /// The session whose metadata the person may already read as a log.
    pub session: SessionId,
    /// Continue strictly after this returned prompt's sequence.
    #[serde(default)]
    pub after: Option<EventSeq>,
    /// Refuse a page from a changed retention/rewind revision.
    #[serde(default)]
    pub epoch: Option<PromptIndexEpoch>,
    /// Bounds only the metadata reply; scan work has its own ceiling.
    #[serde(default)]
    pub limit: PromptIndexLimit,
}

/// One addressable user prompt, including history chunks and steered prompts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptLocation {
    /// First record of the prompt's run.
    pub seq: EventSeq,
    /// Time that record was written; no client clock reconstruction.
    pub timestamp: DateTime<Utc>,
    /// Safe to render without loading the message body.
    pub preview: PromptPreview,
}

/// A consistent metadata page; peers cannot construct unchecked ordered rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptIndexPage {
    session: SessionId,
    epoch: PromptIndexEpoch,
    entries: Vec<PromptLocation>,
    next: Option<EventSeq>,
}

impl PromptIndexPage {
    /// Validate the server projection and the same invariants on decode.
    /// # Errors
    /// Entries exceed their budget, are not ordered or contradict the cursor.
    pub fn new(
        session: SessionId,
        epoch: PromptIndexEpoch,
        entries: Vec<PromptLocation>,
        next: Option<EventSeq>,
    ) -> Result<Self, InvalidPromptIndex> {
        let ordered = entries.windows(2).all(|pair| {
            pair.first()
                .zip(pair.last())
                .is_some_and(|(left, right)| left.seq < right.seq)
        });
        if entries.len() > usize::from(PAGE_ENTRIES)
            || entries
                .iter()
                .any(|entry| entry.seq < epoch.retained_from())
            || !ordered
            || next.is_some_and(|cursor| entries.last().is_none_or(|last| cursor != last.seq))
        {
            return Err(InvalidPromptIndex::Page);
        }
        Ok(Self {
            session,
            epoch,
            entries,
            next,
        })
    }
    /// Correlation includes the session so another panel cannot adopt a reply.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
    /// Reuse this revision when continuing the result set.
    #[must_use]
    pub const fn epoch(&self) -> PromptIndexEpoch {
        self.epoch
    }
    /// Borrow ordered metadata for the outline.
    #[must_use]
    pub fn entries(&self) -> &[PromptLocation] {
        &self.entries
    }
    /// A further page starts strictly after this prompt.
    #[must_use]
    pub const fn next(&self) -> Option<EventSeq> {
        self.next
    }
}

impl<'de> Deserialize<'de> for PromptIndexPage {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            session: SessionId,
            epoch: PromptIndexEpoch,
            entries: Vec<PromptLocation>,
            next: Option<EventSeq>,
        }
        let wire = Wire::deserialize(decoder)?;
        Self::new(wire.session, wire.epoch, wire.entries, wire.next)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
