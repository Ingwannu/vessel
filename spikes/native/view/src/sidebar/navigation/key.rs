//! Parse only UI preference keys here. Plugin addresses are supplied by
//! their producer and are not reconstructed from these persisted strings.

use super::{Section, Visibility};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

/// Longest UTF-8 navigation key kept from a newer or absent contribution.
pub const MAX_KEY_BYTES: usize = 512;

/// Builtin rows, in Paseo's default inventory order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    /// Start a workspace with supported active project context.
    NewWorkspace,
    /// Open Sessions/history.
    History,
    /// Open the shared command center.
    Search,
    /// Open Schedules.
    Schedules,
    /// Optional Usage summary above the fixed icon line.
    Usage,
}
impl Builtin {
    /// Header defaults followed by the footer default.
    pub const ALL: [Self; 5] = [
        Self::NewWorkspace,
        Self::History,
        Self::Search,
        Self::Schedules,
        Self::Usage,
    ];
    /// The independent band this builtin belongs to.
    #[must_use]
    pub const fn section(self) -> Section {
        match self {
            Self::NewWorkspace | Self::History | Self::Search | Self::Schedules => Section::Header,
            Self::Usage => Section::Footer,
        }
    }
    /// Usage is opt-in; every header row starts shown.
    #[must_use]
    pub const fn default_visibility(self) -> Visibility {
        match self {
            Self::Usage => Visibility::Hidden,
            Self::NewWorkspace | Self::History | Self::Search | Self::Schedules => {
                Visibility::Shown
            }
        }
    }
    fn spelling(self) -> &'static str {
        match self {
            Self::NewWorkspace => "new-workspace",
            Self::History => "history",
            Self::Search => "search",
            Self::Schedules => "schedules",
            Self::Usage => "usage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Builtin(Builtin),
    Opaque(Arc<str>),
}

/// Stable navigation preference identity, including absent/future items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Key(Kind);

/// An empty, oversized or control-bearing external preference key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("navigation keys must contain 1 to {MAX_KEY_BYTES} UTF-8 bytes and no controls")]
pub struct InvalidKey;

impl Key {
    /// A known builtin without allocating its static spelling.
    #[must_use]
    pub const fn of(builtin: Builtin) -> Self {
        Self(Kind::Builtin(builtin))
    }
    /// The exact persisted spelling, without interpreting plugin identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Kind::Builtin(builtin) => builtin.spelling(),
            Kind::Opaque(value) => value,
        }
    }
    /// A currently known builtin, or an opaque retained UI key.
    #[must_use]
    pub const fn builtin(&self) -> Option<Builtin> {
        match self.0 {
            Kind::Builtin(builtin) => Some(builtin),
            Kind::Opaque(_) => None,
        }
    }
}
impl FromStr for Key {
    type Err = InvalidKey;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.len() > MAX_KEY_BYTES || value.chars().any(char::is_control) {
            return Err(InvalidKey);
        }
        let kind = match value {
            "new-workspace" => Kind::Builtin(Builtin::NewWorkspace),
            "history" => Kind::Builtin(Builtin::History),
            "search" => Kind::Builtin(Builtin::Search),
            "schedules" => Kind::Builtin(Builtin::Schedules),
            "usage" => Kind::Builtin(Builtin::Usage),
            _ => Kind::Opaque(Arc::from(value)),
        };
        Ok(Self(kind))
    }
}
impl TryFrom<String> for Key {
    type Error = InvalidKey;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<Key> for String {
    fn from(value: Key) -> Self {
        value.as_str().to_owned()
    }
}
