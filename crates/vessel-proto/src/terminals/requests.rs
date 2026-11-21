//! The terminal calls' requests (protocol 11, ADR 0031).

use serde::{Deserialize, Serialize};

use super::{TerminalInfo, TerminalPlace};
use crate::id::{EventSeq, SubscriptionId, TerminalId};
use crate::terminal::{TerminalInput, TerminalSize};
use crate::terminal_profiles::ProfileLaunch;
use crate::title::SessionTitle;

/// `terminal.create`: open the user's shell in a place, or one of the
/// node's terminal profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateTerminal {
    /// Where it runs.
    pub place: TerminalPlace,
    /// Its window; 80×24 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TerminalSize>,
    /// What to call it; a profile's name when absent and `profile` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SessionTitle>,
    /// Run this profile's program instead of the shell (ADR 0117,
    /// [`crate::terminal_profiles::TERMINAL_PROFILES_PROTOCOL`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileLaunch>,
}

/// `terminal.list`: the node's terminals, or one place's.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListTerminals {
    /// Only this session's or project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<TerminalPlace>,
    /// Wait (at most 30 s) until the node's terminal list `revision`
    /// differs from this one (ADR 0116,
    /// [`super::SHARED_TERMINALS_PROTOCOL`]; an older node answers at once).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_from: Option<u64>,
}

/// `terminal.get`: one terminal, at once or after its next change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetTerminal {
    /// Which.
    pub terminal: TerminalId,
    /// Wait (at most 30 s) until its `revision` differs from this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_from: Option<u64>,
}

/// `terminal.read`: a page of a terminal's log. Answered with an
/// `EventPage`, as `session.read` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadTerminal {
    /// Which.
    pub terminal: TerminalId,
    /// The first record wanted; a page starts at the oldest kept one when
    /// this is older.
    pub from: EventSeq,
    /// The most records; the node caps it at [`ReadTerminal::MAX_LIMIT`].
    pub limit: u32,
    /// With nothing at `from` yet, wait this many seconds for something
    /// (at most 30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

impl ReadTerminal {
    /// The most records a page holds.
    pub const MAX_LIMIT: u32 = 512;
}

/// `terminal.subscribe`: a terminal's records from `from`, then live ones,
/// as `ServerFrame::Event` frames; ended with `session.unsubscribe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubscribeTerminal {
    /// Which.
    pub terminal: TerminalId,
    /// The first record wanted (the oldest kept one when this is older).
    pub from: EventSeq,
}

/// `terminal.subscribe`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TerminalSubscribed {
    /// Tags the subscription's events.
    pub subscription: SubscriptionId,
    /// The terminal when the subscription opened.
    pub terminal: TerminalInfo,
}

/// `terminal.send`: keystrokes or a resize, for a shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SendTerminal {
    /// Which.
    pub terminal: TerminalId,
    /// What.
    pub input: TerminalInput,
}

impl SendTerminal {
    /// The most bytes one call writes.
    pub const MAX_BYTES: usize = 64 * 1024;
}

/// `terminal.rename`: give a terminal a title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenameTerminal {
    /// Which.
    pub terminal: TerminalId,
    /// The title.
    pub title: SessionTitle,
}

/// `terminal.kill`: end its process (a service is not started again); the
/// terminal is kept, ended, with its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KillTerminal {
    /// Which.
    pub terminal: TerminalId,
}

/// `terminal.close`: end it if it runs, and forget it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CloseTerminal {
    /// Which.
    pub terminal: TerminalId,
}
