//! Provider cursor text, carried opaquely and bound to a repository by
//! the caller. It is never an address or authority to select a host.
//!
//! The existing forge text macro provides Display and a text-bearing
//! Debug for human-readable names. A provider cursor is private metadata,
//! so this wrapper uses explicit serde and a redacted Debug instead.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::forge::RepoRef;

/// A provider's opaque continuation, not displayed or logged. `Debug`
/// deliberately omits it because providers may embed private metadata.
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PullCursor(Arc<str>);

/// A cursor retained with its source repository, so a changed project
/// remote cannot silently continue a different repository's listing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectPullCursor {
    /// Consistency association only; the node must still resolve the
    /// registered project's remote and compare it before any forge call.
    pub repo: RepoRef,
    /// Passed to the provider only after the repository association fits.
    pub after: PullCursor,
}

/// Why provider cursor text cannot be carried safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPullCursor {
    /// Omission represents the first page; empty text is not a cursor.
    #[error("a pull-request cursor must not be empty")]
    Empty,
    /// A fixed request budget, independent of provider encoding.
    #[error("a pull-request cursor exceeds {max} bytes", max = PullCursor::MAX_BYTES)]
    TooLong,
    /// One opaque printable ASCII token, with no control characters.
    #[error("a pull-request cursor must contain only visible ASCII characters")]
    Characters,
}

impl PullCursor {
    /// Maximum bytes admitted before storing or sending a cursor.
    pub const MAX_BYTES: usize = 2048;

    /// The opaque text, for the provider request or serialization only.
    /// It must never be interpreted as a path, address or query fragment.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PullCursor {
    type Err = InvalidPullCursor;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Err(InvalidPullCursor::Empty);
        }
        if text.len() > Self::MAX_BYTES {
            return Err(InvalidPullCursor::TooLong);
        }
        if !text.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(InvalidPullCursor::Characters);
        }
        Ok(Self(Arc::from(text)))
    }
}

impl fmt::Debug for PullCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PullCursor(..)")
    }
}

impl Serialize for PullCursor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PullCursor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Cursor;
        impl serde::de::Visitor<'_> for Cursor {
            type Value = PullCursor;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "a visible ASCII cursor of at most {} bytes",
                    PullCursor::MAX_BYTES
                )
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
                text.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Cursor)
    }
}
