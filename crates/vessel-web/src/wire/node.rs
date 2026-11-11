//! How the browser names nodes: [`NodeKey`] (any node the UI lists) and
//! [`BookName`] (a book entry, where only one will do). Both are parsed
//! when a frame is decoded, so a handler never sees a name the book would
//! not take; in TypeScript both stay `string`.
//!
//! [Decision Log]
//! - Purpose: the bridge passed node names on as bare strings, checked
//!   (if at all) by whichever source got them; the tests' fake and the
//!   CLI's book checked them differently.
//! - Choice: the book's own `NodeName` inside two wire types. A frame that
//!   holds a name the book would not take does not decode, and the bridge
//!   answers it by id as unreadable (`bridge`'s `Addressed`).
//! - `BookName` is the type of every field that must name a book entry
//!   (`forget`, `rename`, `pair`), so this machine's node cannot be
//!   forgotten or renamed: `_here` does not parse as one, and the error
//!   says why.
//! - Alternatives: one string type validated in each handler (what was
//!   there); the protocol's crate (these are the bridge's frames, not the
//!   protocol's).

use std::fmt;

use serde::{Deserialize, Serialize};
use vessel_client::book::{InvalidNodeName, NodeName};

/// How the browser names a node: a book entry, or this machine's node
/// ([`NodeKey::HERE`] on the wire). Book names start with a letter or a
/// digit, so the two never meet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub enum NodeKey {
    /// This machine's node: the CLI's default when nothing names one,
    /// reached without the book.
    Here,
    /// A node in the book.
    Book(NodeName),
}

impl NodeKey {
    /// This machine's node, on the wire.
    pub const HERE: &str = "_here";

    /// This machine's node.
    #[must_use]
    pub const fn here() -> Self {
        Self::Here
    }

    /// The key as the wire has it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Here => Self::HERE,
            Self::Book(name) => name.as_str(),
        }
    }

    /// Whether it is this machine's node.
    #[must_use]
    pub const fn is_here(&self) -> bool {
        matches!(self, Self::Here)
    }
}

impl From<NodeName> for NodeKey {
    fn from(name: NodeName) -> Self {
        Self::Book(name)
    }
}

impl From<BookName> for NodeKey {
    fn from(name: BookName) -> Self {
        Self::Book(name.0)
    }
}

/// Text that is neither this machine's key nor a book name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(transparent)]
pub struct InvalidNodeKey(#[from] InvalidNodeName);

impl std::str::FromStr for NodeKey {
    type Err = InvalidNodeKey;

    fn from_str(text: &str) -> Result<Self, InvalidNodeKey> {
        if text == Self::HERE {
            return Ok(Self::Here);
        }
        Ok(Self::Book(text.parse()?))
    }
}

impl TryFrom<String> for NodeKey {
    type Error = InvalidNodeKey;

    fn try_from(text: String) -> Result<Self, InvalidNodeKey> {
        text.parse()
    }
}

impl From<NodeKey> for String {
    fn from(key: NodeKey) -> Self {
        key.as_str().to_owned()
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A node's name in the book, where only a book entry will do.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct BookName(NodeName);

impl BookName {
    /// The book's name.
    #[must_use]
    pub const fn node_name(&self) -> &NodeName {
        &self.0
    }
}

impl From<NodeName> for BookName {
    fn from(name: NodeName) -> Self {
        Self(name)
    }
}

/// Text that does not name a book entry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBookName {
    /// This machine's key: its node is reached without the book, so it
    /// has no entry to forget and no name to change, and it is what the
    /// page falls back on with an empty book. (A book entry that points at
    /// this machine is an ordinary entry, and may be forgotten.)
    #[error("this machine's node is not in the book")]
    Here,
    /// Not a name the book takes.
    #[error(transparent)]
    Name(#[from] InvalidNodeName),
}

impl std::str::FromStr for BookName {
    type Err = InvalidBookName;

    fn from_str(text: &str) -> Result<Self, InvalidBookName> {
        if text == NodeKey::HERE {
            return Err(InvalidBookName::Here);
        }
        Ok(Self(text.parse()?))
    }
}

impl TryFrom<String> for BookName {
    type Error = InvalidBookName;

    fn try_from(text: String) -> Result<Self, InvalidBookName> {
        text.parse()
    }
}

impl From<BookName> for String {
    fn from(name: BookName) -> Self {
        name.0.as_str().to_owned()
    }
}

impl fmt::Display for BookName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_names_parse_on_the_way_in() {
        let key: NodeKey = serde_json::from_str("\"_here\"").unwrap();
        assert_eq!(key, NodeKey::Here);
        let key: NodeKey = serde_json::from_str("\"gpu-box\"").unwrap();
        assert_eq!(key.as_str(), "gpu-box");
        assert_eq!(serde_json::to_string(&key).unwrap(), "\"gpu-box\"");
        for bad in ["\"\"", "\"two words\"", "\"-x\"", "\"a/b\"", "3"] {
            assert!(serde_json::from_str::<NodeKey>(bad).is_err(), "{bad}");
        }
        let name: BookName = serde_json::from_str("\"lab\"").unwrap();
        assert_eq!(NodeKey::from(name), "lab".parse().unwrap());
        assert_eq!("_here".parse::<BookName>(), Err(InvalidBookName::Here));
        let error = serde_json::from_str::<BookName>("\"_here\"").unwrap_err();
        assert!(error.to_string().contains("not in the book"), "{error}");
        assert!(matches!(
            "two words".parse::<BookName>(),
            Err(InvalidBookName::Name(_))
        ));
    }
}
