//! Protocol 11's agent calls (ADR 0032): their requests. The replies are
//! the types of `added` (and, later in the track, `status`, `catalogue`,
//! `install`).

use serde::{Deserialize, Serialize};

use super::added::AgentToAdd;
use super::name::AgentName;

/// `agent.list`: every agent the node can run, the four then the added ones
/// by name. Starts no process. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListAgents {}

/// `agent.add`: define an agent on the node by its command (`agents.toml`),
/// and answer with it as now in effect. Starts and fetches nothing. Needs
/// `admin`, and is refused to an agent and to a call from another node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AddAgent {
    /// Its name; agents' and profiles' names are one space, compared
    /// ignoring case.
    pub name: AgentName,
    /// What to add.
    pub agent: AgentToAdd,
    /// Replace an agent of that name (sessions running keep what they
    /// started with).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace: bool,
}

/// `agent.remove`: forget an added agent. Running sessions keep running;
/// an ended one cannot be started again. Refused while a profile extends
/// it. Needs `admin`, and is refused to an agent and to a call from
/// another node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RemoveAgent {
    /// Which agent.
    pub name: AgentName,
}
