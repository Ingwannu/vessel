//! Collection bounds apply before decoding an excess body and before publication.
use super::{
    SourceReportEntry, UsageBalance, UsageDetail, UsageLoginFailure, UsageSourceDescriptor,
    UsageSourceFailure, UsageSourceGrant, UsageWindow,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeSeed, Error as _, SeqAccess, Visitor},
};
use std::{collections::HashSet, fmt, hash::Hash, marker::PhantomData};

/// Maximum accounts retained in one complete report response.
pub const MAX_USAGE_REPORTS: usize = 256;
/// Maximum independent allowances in one account report.
pub const MAX_USAGE_WINDOWS: usize = 128;
/// Maximum independent balances in one account report.
pub const MAX_USAGE_BALANCES: usize = 32;
/// Maximum metadata rows in one account report.
pub const MAX_USAGE_DETAILS: usize = 32;
/// Maximum safe login failures presented for one account.
pub const MAX_USAGE_LOGIN_FAILURES: usize = 16;
/// Maximum independently failed source discoveries in one response.
pub const MAX_USAGE_SOURCE_FAILURES: usize = 64;

/// A collection's semantic identity also owns its one numeric bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageCollection {
    /// Account reports.
    Reports,
    /// Independent account allowances.
    Windows,
    /// Independent balances.
    Balances,
    /// Source metadata rows.
    Details,
    /// Safe login failures.
    Logins,
    /// Independent source failures.
    Sources,
}
impl UsageCollection {
    /// Shared by constructors, decoders and errors; bounds cannot drift.
    #[must_use]
    pub const fn maximum(self) -> usize {
        match self {
            Self::Reports => MAX_USAGE_REPORTS,
            Self::Windows => MAX_USAGE_WINDOWS,
            Self::Balances => MAX_USAGE_BALANCES,
            Self::Details => MAX_USAGE_DETAILS,
            Self::Logins => MAX_USAGE_LOGIN_FAILURES,
            Self::Sources => MAX_USAGE_SOURCE_FAILURES,
        }
    }
}
impl fmt::Display for UsageCollection {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Reports => "reports",
            Self::Windows => "windows",
            Self::Balances => "balances",
            Self::Details => "details",
            Self::Logins => "login failures",
            Self::Sources => "source failures",
        })
    }
}

/// No input value is retained by a structural report error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidUsageCollection {
    /// A complete response must refuse, rather than omit, excess records.
    #[error("usage {kind} permits at most {maximum} entries", maximum = .kind.maximum())]
    TooMany {
        /// Collection whose bound was exceeded.
        kind: UsageCollection,
    },
    /// Duplicate identities cannot overwrite a different observed value.
    #[error("usage {kind} contains duplicate identities")]
    Duplicate {
        /// Collection with an ambiguous identity.
        kind: UsageCollection,
    },
}

struct Excess(UsageCollection);
impl<'de> DeserializeSeed<'de> for Excess {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(D::Error::custom(InvalidUsageCollection::TooMany {
            kind: self.0,
        }))
    }
}
struct Records<T> {
    kind: UsageCollection,
    item: PhantomData<T>,
}
impl<'de, T: Deserialize<'de>> Visitor<'de> for Records<T> {
    type Value = Vec<T>;
    fn expecting(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            output,
            "a usage {} list of at most {} entries",
            self.kind,
            self.kind.maximum()
        )
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        if sequence
            .size_hint()
            .is_some_and(|count| count > self.kind.maximum())
        {
            return Err(A::Error::custom(InvalidUsageCollection::TooMany {
                kind: self.kind,
            }));
        }
        let mut result = Vec::new();
        loop {
            if result.len() == self.kind.maximum() {
                return match sequence.next_element_seed(Excess(self.kind))? {
                    None => Ok(result),
                    Some(()) => Err(A::Error::custom(InvalidUsageCollection::TooMany {
                        kind: self.kind,
                    })),
                };
            }
            match sequence.next_element()? {
                Some(item) => result.push(item),
                None => return Ok(result),
            }
        }
    }
}
fn decode<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    input: D,
    kind: UsageCollection,
) -> Result<Vec<T>, D::Error> {
    input.deserialize_seq(Records {
        kind,
        item: PhantomData,
    })
}
fn unique<'a, T: Eq + Hash + 'a>(
    keys: impl Iterator<Item = &'a T>,
    kind: UsageCollection,
) -> Result<(), InvalidUsageCollection> {
    let mut seen = HashSet::new();
    for key in keys {
        if !seen.insert(key) {
            return Err(InvalidUsageCollection::Duplicate { kind });
        }
    }
    Ok(())
}

macro_rules! bounded {
    ($name:ident, $item:ty, $kind:ident, $doc:literal $(, $key:ident)?) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name(Vec<$item>);
        impl $name {
            /// Borrow already bounded and identity-checked records.
            #[must_use]
            pub fn as_slice(&self) -> &[$item] { &self.0 }
            /// Empty means no records, never a fabricated zero-valued report.
            #[must_use]
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
        }
        impl TryFrom<Vec<$item>> for $name {
            type Error = InvalidUsageCollection;
            fn try_from(values: Vec<$item>) -> Result<Self, Self::Error> {
                let kind = UsageCollection::$kind;
                if values.len() > kind.maximum() { return Err(InvalidUsageCollection::TooMany { kind }); }
                $(unique(values.iter().map(|item| &item.$key), kind)?;)?
                Ok(Self(values))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
                let records = decode(input, UsageCollection::$kind)?;
                Self::try_from(records).map_err(D::Error::custom)
            }
        }
    };
}
bounded!(
    ReportWindows,
    UsageWindow,
    Windows,
    "Validated independent allowance identities and bounds.",
    id
);
bounded!(
    ReportBalances,
    UsageBalance,
    Balances,
    "Validated independent balance identities and bounds.",
    id
);
bounded!(
    ReportDetails,
    UsageDetail,
    Details,
    "Validated source metadata identities and bounds.",
    id
);
bounded!(
    ReportLoginFailures,
    UsageLoginFailure,
    Logins,
    "Bounded safe login outcomes; their presentation labels may coincide."
);
bounded!(
    ReportEntries,
    SourceReportEntry,
    Reports,
    "A complete bounded report inventory; duplicate account ids are refused.",
    id
);
bounded!(
    ReportSourceFailures,
    UsageSourceFailure,
    Sources,
    "Independent safe discovery failures; a source occurs at most once.",
    source
);
bounded!(
    ReportSourceDescriptors,
    UsageSourceDescriptor,
    Sources,
    "Cache-only source metadata is bounded and refuses duplicate identities.",
    id
);
bounded!(
    ReportSourceGrants,
    UsageSourceGrant,
    Sources,
    "Only bounded unique source consent revisions are persisted.",
    id
);

impl ReportBalances {
    pub(super) fn redact_amounts(&mut self) {
        for balance in &mut self.0 {
            balance.used = None;
            balance.remaining = None;
            balance.limit = None;
        }
    }
}
impl ReportDetails {
    pub(super) fn read_only(&mut self) {
        self.0
            .retain(|detail| detail.access == super::UsageDetailAccess::Read);
    }
}

impl ReportEntries {
    pub(super) fn redact(&mut self, visibility: crate::usage::CostVisibility) {
        for report in &mut self.0 {
            report.redact(visibility);
        }
    }
}
