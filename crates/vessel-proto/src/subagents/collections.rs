//! Finite responses are validated both for local construction and wire decoding.

use super::{ChildDescriptor, ChildHistory, ChildLimit, bounds};
use crate::{EventRecord, EventSeq, SessionEvent};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashSet, fmt, io, marker::PhantomData};

/// A malformed or excessive provider projection is never silently shortened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidChildCollection {
    /// The entry count exceeds the declared response bound.
    #[error("provider projection contains too many entries")]
    Count,
    /// Escaped serialized bytes exceed the aggregate response bound.
    #[error("provider projection exceeds its serialized byte budget")]
    Bytes,
    /// A descriptor identity is repeated or nesting is cyclic/too deep.
    #[error("provider projection has ambiguous identities or nesting")]
    Identity,
    /// Child history must be canonical read-only activity in parent order.
    #[error("provider timeline has invalid content or parent sequence order")]
    Timeline,
}

/// Descriptor retention is bounded by count and actual serialized JSON bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChildDescriptors(Vec<ChildDescriptor>);

impl TryFrom<Vec<ChildDescriptor>> for ChildDescriptors {
    type Error = InvalidChildCollection;
    fn try_from(rows: Vec<ChildDescriptor>) -> Result<Self, Self::Error> {
        check_size(&rows, bounds::MAX_CHILDREN, bounds::MAX_DESCRIPTOR_BYTES)?;
        let ids: HashSet<_> = rows.iter().map(|row| &row.id).collect();
        if ids.len() != rows.len() {
            return Err(InvalidChildCollection::Identity);
        }
        for row in &rows {
            let mut seen = HashSet::from([&row.id]);
            let mut parent = row.parent.as_ref();
            while let Some(id) = parent {
                if !seen.insert(id) || seen.len() > bounds::MAX_CHILD_DEPTH {
                    return Err(InvalidChildCollection::Identity);
                }
                parent = rows
                    .iter()
                    .find(|other| &other.id == id)
                    .and_then(|p| p.parent.as_ref());
            }
        }
        Ok(Self(rows))
    }
}
impl ChildDescriptors {
    /// Stable provider order, never a managed-session authority source.
    #[must_use]
    pub fn as_slice(&self) -> &[ChildDescriptor] {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ChildDescriptors {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        bounded_rows(d, bounds::MAX_CHILDREN, bounds::MAX_DESCRIPTOR_BYTES)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

/// A child page reuses parent event sequence/time; it has no separate log epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChildRecords(Vec<EventRecord>);
impl TryFrom<Vec<EventRecord>> for ChildRecords {
    type Error = InvalidChildCollection;
    fn try_from(rows: Vec<EventRecord>) -> Result<Self, Self::Error> {
        let max = usize::try_from(u32::from(ChildLimit::default()))
            .map_err(|_error| InvalidChildCollection::Count)?;
        check_size(&rows, max, bounds::MAX_SCAN_BYTES)?;
        let mut previous = None;
        for row in &rows {
            if previous.is_some_and(|seq| seq >= row.seq) {
                return Err(InvalidChildCollection::Timeline);
            }
            let SessionEvent::Agent { event } = &row.event else {
                return Err(InvalidChildCollection::Timeline);
            };
            if super::ChildActivity::try_from(event.clone()).is_err() {
                return Err(InvalidChildCollection::Timeline);
            }
            previous = Some(row.seq);
        }
        Ok(Self(rows))
    }
}
impl ChildRecords {
    /// Canonical read-only child records in sparse parent order.
    #[must_use]
    pub fn as_slice(&self) -> &[EventRecord] {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ChildRecords {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let max = usize::try_from(bounds::MAX_TIMELINE_ITEMS).map_err(serde::de::Error::custom)?;
        bounded_rows(d, max, bounds::MAX_SCAN_BYTES)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

/// Provider support and coverage accompany every list, including an empty one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderChildren {
    /// Validated finite descriptors.
    pub children: ChildDescriptors,
    /// Whether this is complete, partial, unsupported or unavailable.
    pub history: ChildHistory,
}

/// One bounded child timeline page; continuation is a parent-log position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProviderChildPage {
    /// Canonical child activity retaining original parent sequence and time.
    pub records: ChildRecords,
    /// First parent position for a subsequent forward page.
    pub next: EventSeq,
    /// The scan reached the present parent-log end.
    pub at_end: bool,
    /// Retention/support gaps never turn into an empty successful history.
    pub history: ChildHistory,
}

impl ProviderChildPage {
    pub(crate) fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        if !self
            .records
            .as_slice()
            .iter()
            .any(|record| record.for_protocol(protocol).is_some())
        {
            return None;
        }
        let projected = self
            .records
            .as_slice()
            .iter()
            .map(|record| {
                record
                    .for_protocol(protocol)
                    .unwrap_or_else(|| record.clone())
            })
            .collect::<Vec<_>>();
        let (records, history) = match ChildRecords::try_from(projected) {
            Ok(records) => (records, self.history),
            Err(_) => {
                // An attachment stand-in may grow. Withhold that page rather
                // than leak an unmasked field or silently claim completeness.
                let first = self.records.as_slice().first()?;
                (
                    ChildRecords(Vec::new()),
                    ChildHistory::Partial {
                        reason: super::ChildGap::ScanBudget,
                        from: first.seq,
                    },
                )
            }
        };
        Some(Self {
            records,
            history,
            next: self.next,
            at_end: self.at_end,
        })
    }
}

/// A projection accounts for every examined parent record, including unrelated
/// main events. Byte counting uses actual serialized bytes, without a copy.
#[derive(Debug, Default)]
pub struct ChildScanBudget {
    records: usize,
    bytes: usize,
}
impl ChildScanBudget {
    /// Admit before observing/retaining a record; failed admission changes no state.
    /// # Errors
    /// The record count or actual serialized byte budget would be exceeded.
    pub fn admit(&mut self, record: &EventRecord) -> Result<(), InvalidChildCollection> {
        if self.records == bounds::MAX_SCAN_RECORDS {
            return Err(InvalidChildCollection::Count);
        }
        let remaining = bounds::MAX_SCAN_BYTES.saturating_sub(self.bytes);
        let size = serialized_bytes(record, remaining)?;
        self.records = self.records.saturating_add(1);
        self.bytes = self.bytes.saturating_add(size);
        Ok(())
    }
}

/// Count escaping and field overhead without allocating a second serialized copy.
pub(crate) fn serialized_bytes<T: Serialize>(
    value: &T,
    max: usize,
) -> Result<usize, InvalidChildCollection> {
    let mut sink = ByteBudget { used: 0, max };
    serde_json::to_writer(&mut sink, value).map_err(|_error| InvalidChildCollection::Bytes)?;
    Ok(sink.used)
}
fn check_size<T: Serialize>(
    rows: &[T],
    count: usize,
    bytes: usize,
) -> Result<(), InvalidChildCollection> {
    if rows.len() > count {
        return Err(InvalidChildCollection::Count);
    }
    serialized_bytes(&rows, bytes).map(|_| ())
}
struct ByteBudget {
    used: usize,
    max: usize,
}
impl io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .used
            .checked_add(bytes.len())
            .filter(|next| *next <= self.max)
            .ok_or_else(|| io::Error::other("provider byte budget exceeded"))?;
        self.used = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Stop before retaining an extra element; do not trust a sequence's size hint.
fn bounded_rows<'de, D, T>(d: D, max: usize, bytes: usize) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Serialize,
{
    struct Rows<T> {
        max: usize,
        bytes: usize,
        value: PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de> + Serialize> serde::de::Visitor<'de> for Rows<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "a provider collection of at most {} entries", self.max)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            let mut remaining = self.bytes.saturating_sub(b"[]".len());
            while let Some(row) = seq.next_element()? {
                if rows.len() == self.max {
                    return Err(serde::de::Error::custom(InvalidChildCollection::Count));
                }
                remaining = remaining
                    .checked_sub(usize::from(!rows.is_empty()))
                    .ok_or_else(|| serde::de::Error::custom(InvalidChildCollection::Bytes))?;
                let used = serialized_bytes(&row, remaining).map_err(serde::de::Error::custom)?;
                remaining = remaining.saturating_sub(used);
                rows.push(row);
            }
            Ok(rows)
        }
    }
    d.deserialize_seq(Rows {
        max,
        bytes,
        value: PhantomData,
    })
}

#[cfg(test)]
mod tests;
