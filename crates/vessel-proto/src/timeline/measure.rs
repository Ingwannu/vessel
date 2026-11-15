//! Units for canonical tool details; offsets, counts and exits cannot be swapped.

use serde::{Deserialize, Serialize};

macro_rules! measure {
    ($name:ident, $inner:ty, $why:literal) => {
        #[doc = $why]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            /// Keep an observed quantity in its own unit at the decoding edge.
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Expose the magnitude for formatting without changing its unit.
            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

measure!(
    DetailCount,
    u64,
    "A reported count of files, matches or lines in a tool detail."
);
measure!(
    DetailOffset,
    u64,
    "A provider's read offset, preserved without assuming its line convention."
);
measure!(
    ToolByteCount,
    u64,
    "Reported fetched bytes, kept distinct from characters or tokens."
);
measure!(
    ToolExitCode,
    i64,
    "A tool's reported exit code, including provider-specific signed values."
);

/// A positive ordinal in the provider's ordered setup commands or sub-agent actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct DetailIndex(std::num::NonZeroU32);

impl DetailIndex {
    /// Reject the absent ordinal instead of interpreting zero as a sentinel.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match std::num::NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Its one-based magnitude for presentation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_refuse_zero_at_both_construction_and_decode() {
        assert!(DetailIndex::new(0).is_none());
        assert!(serde_json::from_str::<DetailIndex>("0").is_err());
        assert_eq!(serde_json::from_str::<DetailIndex>("1").unwrap().get(), 1);
    }
}
