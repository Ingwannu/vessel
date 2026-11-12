//! The ports the node's sessions and terminals listen on (protocol 11, ADR
//! 0031), reported as facts.
//!
//! [Decision Log]
//! - Purpose: a dev server a session or a service starts opens a port a
//!   person wants to look at; GOAT finds every listener every 3 s, Paseo
//!   knows its services' ports because it chose them.
//! - Choice: the node finds the TCP listeners of the process trees it
//!   started (its sessions' and terminals'), with no privileged call, at
//!   most every 3 s and only while someone asks, and reports them with the
//!   address (so loopback is told from every interface) and the owner.
//!   Nothing is opened or forwarded: how a page reaches a port is the
//!   preview's decision, through the bridge.

use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{SessionId, TerminalId};

/// What the system calls a process (`comm` on Linux, `lsof`'s command on
/// macOS, the image name on Windows): at most 64 characters, no control
/// character. Content a person reads, kept plain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ProcessName(Arc<str>);

/// A name refused as a [`ProcessName`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidProcessName;

impl fmt::Display for InvalidProcessName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a process name is 1 to 64 characters, without control characters")
    }
}

impl std::error::Error for InvalidProcessName {}

impl ProcessName {
    /// The longest name, in characters.
    pub const MAX_CHARS: usize = 64;

    /// The name the system gave, made to fit: control characters dropped,
    /// cut at [`MAX_CHARS`](Self::MAX_CHARS); `None` when nothing is left.
    #[must_use]
    pub fn from_system(text: &str) -> Option<Self> {
        let kept: String = text
            .trim()
            .chars()
            .filter(|character| !character.is_control())
            .take(Self::MAX_CHARS)
            .collect();
        (!kept.is_empty()).then(|| Self(Arc::from(kept)))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ProcessName {
    type Err = InvalidProcessName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let fits = !text.is_empty()
            && text.chars().count() <= Self::MAX_CHARS
            && !text.chars().any(char::is_control);
        if fits {
            Ok(Self(Arc::from(text)))
        } else {
            Err(InvalidProcessName)
        }
    }
}

impl fmt::Display for ProcessName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(ProcessName);

/// Whose process tree a listener is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PortOwner {
    /// A session's (its terminal program or its agent, and their children).
    Session {
        /// Which.
        session: SessionId,
    },
    /// A terminal's (a shell, a script or a service, and their children).
    Terminal {
        /// Which.
        terminal: TerminalId,
    },
}

/// A TCP socket in the listening state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListeningPort {
    /// The port.
    pub port: u16,
    /// The local address it listens on (`127.0.0.1`, `::1`, `0.0.0.0`,
    /// `::`): loopback only, or every interface. Text on the wire.
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub address: IpAddr,
    /// The listening process.
    pub pid: u32,
    /// Its name, where the system said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessName>,
    /// Whose tree it is in.
    pub owner: PortOwner,
}

/// How the last scan went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PortScan {
    /// No scan yet (they run only while someone asks).
    Pending,
    /// Scanned at `at`.
    Scanned {
        /// When.
        at: DateTime<Utc>,
    },
    /// The system's tool is missing or failed.
    Unavailable {
        /// Why.
        reason: String,
    },
    /// A state a newer node knows.
    #[serde(other)]
    Unknown,
}

/// `port.list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PortList {
    /// The listeners the caller may see, by owner, then port.
    pub ports: Vec<ListeningPort>,
    /// How the last scan went.
    pub scan: PortScan,
    /// Changes when the list or the scan's state does.
    pub revision: u64,
    /// Whether there were more than 256.
    pub more: bool,
}

/// `port.list`: the listeners found, at once or after the next change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListPorts {
    /// Wait (at most 30 s) until the `revision` differs from this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_from: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_systems_name_is_made_to_fit() {
        assert_eq!(
            ProcessName::from_system(" node\n").unwrap().as_str(),
            "node"
        );
        assert!(ProcessName::from_system("\u{7}\n").is_none());
        let long = "x".repeat(100);
        assert_eq!(
            ProcessName::from_system(&long).unwrap().as_str().len(),
            ProcessName::MAX_CHARS
        );
        assert!(serde_json::from_str::<ProcessName>("\"a\\u0007b\"").is_err());
    }

    #[test]
    fn an_owner_reads_as_written() {
        let owner: PortOwner = serde_json::from_str(
            r#"{"kind":"terminal","terminal":"7b0c2f4e-62a8-4f4c-9a51-8f6f4a3e2d10"}"#,
        )
        .unwrap();
        assert!(matches!(owner, PortOwner::Terminal { .. }));
    }
}
