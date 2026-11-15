//! Private CDP control identities never become browser/pane ids or diagnostics.

use std::fmt;
use std::num::NonZeroU64;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

const MAX_CDP_ID_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct CommandId(NonZeroU64);
impl CommandId {
    pub(super) fn from_sequence(sequence: u64) -> Option<Self> {
        NonZeroU64::new(sequence).map(Self)
    }
}

/// A malformed private id is refused without copying it into an error.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("CDP identity is empty, too long or contains whitespace")]
pub(crate) struct InvalidCdpIdentity;

macro_rules! opaque_id {
    ($name:ident, $doc:literal) => {
        #[doc=$doc]
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub(crate) struct $name(Arc<str>);
        impl TryFrom<String> for $name {
            type Error = InvalidCdpIdentity;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty()
                    || value.len() > MAX_CDP_ID_BYTES
                    || value.chars().any(|c| c.is_control() || c.is_whitespace())
                {
                    Err(InvalidCdpIdentity)
                } else {
                    Ok(Self(Arc::from(value)))
                }
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0.to_string()
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}
opaque_id!(
    TargetId,
    "A Chromium target id, never a node browser resource id."
);
opaque_id!(
    SessionId,
    "A flattened CDP session id, unrelated to a Vessel `SessionId`."
);

opaque_id!(
    FrameId,
    "A private frame id within the admitted flattened target session."
);
opaque_id!(
    LoaderId,
    "A private loader id; snapshots cannot cross a navigation."
);
/// A context is re-created/rechecked per traversal, never selected by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ContextId(NonZeroU64);
