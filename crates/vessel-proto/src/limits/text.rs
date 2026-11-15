//! Vendor quota identities and presentation names, decoded and bounded once.
use std::{fmt, str::FromStr, sync::Arc};

/// A quota string failed its bounded, single-line rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a quota {kind} must be nonempty, bounded, trimmed and without control characters")]
pub struct InvalidLimitText {
    kind: &'static str,
}

macro_rules! limit_text {
    ($(#[$doc:meta])* $name:ident, $kind:literal, $bytes:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);
        impl $name {
            /// The validated vendor text.
            #[must_use]
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl FromStr for $name {
            type Err = InvalidLimitText;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if text.is_empty() || text.len() > $bytes || text.trim() != text || text.chars().any(char::is_control) {
                    Err(InvalidLimitText { kind: $kind })
                } else { Ok(Self(Arc::from(text))) }
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
        crate::key::serde_via_text!($name);
    };
}
limit_text! {
    /// Provider's stable quota identity, bounded to 512 bytes. Punctuation
    /// remains significant: model ids `fable-pro` and `fable_pro` differ.
    LimitKey, "identity", 512
}
limit_text! {
    /// Provider's human label, bounded to 256 bytes; UI text may wrap it.
    LimitName, "name", 256
}
