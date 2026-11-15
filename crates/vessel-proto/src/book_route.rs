//! Client-book route identity shared without a client/view dependency edge.
//! This is local persisted data, not an RPC connection or a protocol field.

use std::fmt;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// A client-minted route id scoped to one machine entry in the client book.
///
/// Not on the wire: this value lives in proto so `vessel-client` and the native
/// view share one type without either depending on the other's I/O/UI layer.
/// It is not the node's connection lifetime identity. All values are exact
/// signed TOML integers and JavaScript numbers; zero is never an allocated id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BookRouteId(NonZeroU32);

impl BookRouteId {
    /// The first allocated route in an entry.
    pub const FIRST: Self = Self(NonZeroU32::MIN);

    /// The last representable route; allocating after it is exhausted.
    pub const LAST: Self = Self(NonZeroU32::MAX);

    /// Decode a persisted id; zero is invalid rather than an empty sentinel.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Its exact numeric representation for persistence and projection.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Allocate the successor, refusing exhaustion rather than wrapping/reusing.
    /// The book retains its allocation counter when a route is removed.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        // Checked addition is the allocation boundary, including LAST.
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl fmt::Display for BookRouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests;
