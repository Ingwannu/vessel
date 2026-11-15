//! Local identities keep workspace selection independent of filesystem paths.

use super::LayoutError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use vessel_proto::WorkspaceId;
use vessel_proto::{NodeId, SessionId};

/// A defensive storage bound, independent of the server's identifier grammar.
pub(super) const MAX_ID_BYTES: usize = 512;
macro_rules! text_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Arc<str>);
        impl $name {
            /// Validate once before storing or crossing the panel boundary.
            ///
            /// # Errors
            /// Refuses empty, overlong or control-containing identity text.
            pub fn parse(text: &str) -> Result<Self, LayoutError> {
                let text = text.trim();
                if text.is_empty()
                    || text.len() > MAX_ID_BYTES
                    || text.chars().any(char::is_control)
                {
                    return Err(LayoutError::InvalidIdentity);
                }
                Ok(Self(Arc::from(text)))
            }
            /// The validated spelling, only for presentation or encoding.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = LayoutError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(&value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0.to_string()
            }
        }
    };
}
text_id!(
    HostAlias,
    "Compatibility identity while an old app route lacks a node id."
);
text_id!(
    ResourceId,
    "Opaque panel-owned identity for browser and provider child resources."
);

/// Registry identity is preferred; the old session route has an explicit bridge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LayoutKey {
    /// Two logical workspaces in the same folder keep separate layouts.
    Workspace {
        /// Stable server identity survives display-name changes.
        node: NodeId,
        /// Logical registry identity.
        workspace: WorkspaceId,
    },
    /// Old routes cannot infer workspace ownership from cwd.
    Session {
        /// Existing machine-book alias, migrated on book rename.
        machine: HostAlias,
        /// Globally unique session from that route.
        session: SessionId,
    },
}

/// Stable local tab identity survives moves and compact-sheet promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TabId(pub(super) u64);
impl TabId {
    /// Presentation key; callers cannot manufacture stale counters.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
/// Pane identity survives content selection and reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PaneId(pub(super) u64);
impl PaneId {
    /// The initial ordinary pane.
    pub const MAIN: Self = Self(0);
    /// Explorer is docked outside the ordinary split tree.
    pub const EXPLORER: Self = Self(1);
    /// Presentation key for toolkit element identity.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Split-group identity is distinct from either child pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupId(pub(super) u64);
impl GroupId {
    /// Toolkit key for a surviving split group.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
