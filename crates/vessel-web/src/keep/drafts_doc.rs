//! The drafts file (`ui-drafts.json`, ADR 0028): its format, merging this
//! process's saves into what another process wrote, and the bounds.
//!
//! ```json
//! { "version": 1,
//!   "drafts": [ { "key": { "kind": "new" }, "text": "…", "at": "2026-11-11T09:00:00Z" } ] }
//! ```
//!
//! The unit kept whole is one draft: an entry this build cannot read (a
//! key kind a newer page added, say) is written back as it was, and is
//! counted against the bounds like any other. Unknown top-level keys are
//! kept too. Written compactly: it is prompt text, not something a person
//! edits.
//!
//! [Decision Log] Bounds and merging
//! - Bounds: 64 drafts, 128 KiB each (the wire's `DraftText`), and the
//!   file at most 1 MiB, measured as the bytes written so that every file
//!   written can be read again under the same bound (seven drafts at the
//!   128 KiB limit fit, with their JSON around them). Past a bound the
//!   drafts saved longest ago go first; a draft not saved for 30 days goes
//!   too, since a session deleted elsewhere leaves its draft behind. A
//!   kept entry whose time cannot be read is never aged out but goes first
//!   when a bound is passed.
//! - Merging: per key, the later save wins, whichever process made it; an
//!   empty save removes the draft unless the file holds a later one. So
//!   two tabs or two processes typing into one draft end with the later
//!   save, and a removal never takes text saved after it.
//! - Alternatives: this process's saves always winning (an older save
//!   would overwrite a newer one another process wrote in between); a lock
//!   file (the book has none either, and a crashed holder would need
//!   breaking).

use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::RawJson;

use crate::wire::{Draft, DraftKey, DraftText, DraftsState, Kept};

/// The file's format.
pub(crate) const VERSION: u32 = 1;

/// The largest file read or written, in bytes.
pub(crate) const MAX_FILE: u64 = 1024 * 1024;

/// The most drafts kept.
pub(crate) const MAX_DRAFTS: usize = 64;

/// How long a draft is kept without being saved again.
pub(crate) const MAX_AGE: TimeDelta = TimeDelta::days(30);

/// A save this process has not written yet; an empty text removes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Save {
    /// The text.
    pub(crate) text: DraftText,
    /// When it was saved.
    pub(crate) at: DateTime<Utc>,
}

/// An entry this build cannot read, kept as it was, with its time if that
/// much reads.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeptDraft {
    raw: RawJson,
    at: Option<DateTime<Utc>>,
}

/// Every draft: those this build reads, and the rest of the file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DraftSet {
    read: BTreeMap<DraftKey, Draft>,
    kept: Vec<KeptDraft>,
    rest: BTreeMap<String, RawJson>,
}

/// A file read.
#[derive(Debug)]
pub(crate) enum Decoded {
    /// In this build's format.
    Current(DraftSet),
    /// In a newer format: what could be read, for this launch only.
    Newer {
        /// The file's version.
        version: u32,
        /// What could be read.
        set: DraftSet,
    },
}

/// Why a file is not a drafts file.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DecodeError {
    /// Not a JSON object with a list of drafts.
    #[error("not a drafts file: {0}")]
    Format(#[source] serde_json::Error),
    /// A version no `vessel` wrote.
    #[error("version {0} is not a version of this file")]
    Version(u64),
}

#[derive(Deserialize)]
struct Head {
    #[serde(default)]
    version: Option<u64>,
}

#[derive(Deserialize)]
struct Body {
    #[serde(default)]
    drafts: Vec<RawJson>,
    #[serde(flatten)]
    rest: BTreeMap<String, RawJson>,
}

/// One entry as this build writes it; anything else is kept.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    key: DraftKey,
    text: DraftText,
    at: DateTime<Utc>,
}

/// The time of an entry that did not read whole.
#[derive(Deserialize)]
struct Time {
    at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Entry<'a> {
    Read(&'a Draft),
    Kept(&'a RawJson),
}

#[derive(Serialize)]
struct Written<'a> {
    version: u32,
    drafts: Vec<Entry<'a>>,
    #[serde(flatten)]
    rest: &'a BTreeMap<String, RawJson>,
}

/// Which entry to drop next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Slot {
    Kept(usize),
    Read(DraftKey),
}

impl DraftSet {
    /// Read a file's bytes.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Decoded, DecodeError> {
        let head: Head = serde_json::from_slice(bytes).map_err(DecodeError::Format)?;
        let mut body: Body = serde_json::from_slice(bytes).map_err(DecodeError::Format)?;
        let _version = body.rest.remove("version");
        let mut set = Self {
            rest: body.rest,
            ..Self::default()
        };
        for raw in body.drafts {
            match serde_json::from_str::<Record>(raw.as_text()) {
                // An empty draft is no draft.
                Ok(record) if record.text.is_empty() => {}
                Ok(record) => {
                    let later = set
                        .read
                        .get(&record.key)
                        .is_none_or(|other| other.at <= record.at);
                    if later {
                        let draft = Draft {
                            key: record.key,
                            text: record.text,
                            at: record.at,
                        };
                        let _older = set.read.insert(record.key, draft);
                    }
                }
                Err(_unknown) => {
                    let at = serde_json::from_str::<Time>(raw.as_text())
                        .ok()
                        .map(|time| time.at);
                    set.kept.push(KeptDraft { raw, at });
                }
            }
        }
        match head.version {
            None | Some(1) => Ok(Decoded::Current(set)),
            Some(version @ 2..) => Ok(Decoded::Newer {
                version: u32::try_from(version).unwrap_or(u32::MAX),
                set,
            }),
            Some(version @ 0) => Err(DecodeError::Version(version)),
        }
    }

    /// The file's bytes.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.written(true))
    }

    fn written(&self, with_drafts: bool) -> Written<'_> {
        let drafts = if with_drafts {
            self.read
                .values()
                .map(Entry::Read)
                .chain(self.kept.iter().map(|kept| Entry::Kept(&kept.raw)))
                .collect()
        } else {
            Vec::new()
        };
        Written {
            version: VERSION,
            drafts,
            rest: &self.rest,
        }
    }

    /// Merge saves into the set: per key, the later save wins.
    pub(crate) fn merge(&mut self, saves: &BTreeMap<DraftKey, Save>) {
        for (key, save) in saves {
            if self.read.get(key).is_some_and(|theirs| theirs.at > save.at) {
                continue;
            }
            if save.text.is_empty() {
                let _removed = self.read.remove(key);
            } else {
                let draft = Draft {
                    key: *key,
                    text: save.text.clone(),
                    at: save.at,
                };
                let _old = self.read.insert(*key, draft);
            }
        }
    }

    /// Drop what is past the bounds (module notes), as of `now`.
    pub(crate) fn bound(&mut self, now: DateTime<Utc>) {
        let fresh = |at: DateTime<Utc>| now.signed_duration_since(at) <= MAX_AGE;
        self.read.retain(|_key, draft| fresh(draft.at));
        self.kept.retain(|kept| kept.at.is_none_or(fresh));

        let length = |entry: &Entry<'_>| serde_json::to_vec(entry).map_or(0, |bytes| bytes.len());
        let mut sizes: BTreeMap<Slot, usize> = self
            .read
            .values()
            .map(|draft| (Slot::Read(draft.key), length(&Entry::Read(draft))))
            .chain(
                self.kept
                    .iter()
                    .enumerate()
                    .map(|(index, kept)| (Slot::Kept(index), length(&Entry::Kept(&kept.raw)))),
            )
            .collect();
        let fixed = serde_json::to_vec(&self.written(false)).map_or(0, |bytes| bytes.len());
        let bound = usize::try_from(MAX_FILE).unwrap_or(usize::MAX);
        // The bytes written: the frame, each entry, and a comma between two.
        let total = |sizes: &BTreeMap<Slot, usize>| {
            sizes
                .values()
                .fold(fixed, |sum, size| sum.saturating_add(*size))
                .saturating_add(sizes.len().saturating_sub(1))
        };
        // Oldest first: a kept entry without a time, then by time.
        let mut order: Vec<(Option<DateTime<Utc>>, Slot)> = self
            .read
            .values()
            .map(|draft| (Some(draft.at), Slot::Read(draft.key)))
            .chain(
                self.kept
                    .iter()
                    .enumerate()
                    .map(|(index, kept)| (kept.at, Slot::Kept(index))),
            )
            .collect();
        order.sort_unstable();
        let mut dropped = Vec::new();
        for (_at, slot) in order {
            if sizes.len() <= MAX_DRAFTS && total(&sizes) <= bound {
                break;
            }
            let _size = sizes.remove(&slot);
            dropped.push(slot);
        }
        for slot in dropped {
            match slot {
                Slot::Read(key) => {
                    let _gone = self.read.remove(&key);
                }
                Slot::Kept(_) => {}
            }
        }
        let mut index = 0_usize;
        self.kept.retain(|_kept| {
            let keep = sizes.contains_key(&Slot::Kept(index));
            index = index.saturating_add(1);
            keep
        });
    }

    /// One draft's text.
    pub(crate) fn text(&self, key: &DraftKey) -> Option<DraftText> {
        self.read.get(key).map(|draft| draft.text.clone())
    }

    /// Every draft the page can read, ordered by key.
    pub(crate) fn state(&self, kept: Kept) -> DraftsState {
        DraftsState {
            kept,
            drafts: self.read.values().cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests;
