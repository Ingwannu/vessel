//! Agents beyond the four Vessel knows (protocol 11, ADR 0032): how one is
//! named on the wire, what a node keeps of the ones its owner added, and
//! what it knows of each agent's installation and sign-in.
//!
//! [Decision Log]
//! - Purpose: a node runs any agent that speaks ACP, not only Claude Code,
//!   Codex, Gemini CLI and `OpenCode`, without breaking a peer that knows only
//!   those four (protocols 2 to 10).
//! - Constraint: [`HarnessKind`](crate::HarnessKind) is matched
//!   exhaustively wherever Vessel knows something about an agent (its home,
//!   its login, its session files, fleet sync, routing). That knowledge
//!   does not exist for an agent a person added by its command, and must
//!   not be guessed.
//! - Alternatives: a fifth `HarnessKind` variant carrying a name (every
//!   `match` on the four would gain an arm that pretends to know, and the
//!   type would lose `Copy`); a free string for every agent (the four would
//!   lose their exhaustive matches); added names in the same space of
//!   strings as the four (a stored session would change meaning the day
//!   Vessel knows a fifth agent by a name someone had used).
//! - Choice: [`HarnessRef`], the four as they are or an added agent by its
//!   [`AgentName`]; on the wire the four stay the strings they were and an
//!   added one is `{"added": "<name>"}`. Code that needs knowledge matches
//!   it and says, for `Added`, that the feature is not offered.
//! - Cost: every place that took a `HarnessKind` from a spec decides what an
//!   added agent gets there; the compiler lists them.

mod name;
mod older;

pub use name::{AgentName, AgentNameError, HarnessRef, TerminalSupport, UnknownAgent};
pub use older::{READS_ADDED_AGENTS, readable, unreadable};
pub(crate) use older::{harnesses as harnesses_for, profiles as profiles_for};
