//! [`Driver`]: which protocol drives an agent's structured session (ADR
//! 0057).
//!
//! [Decision Log]
//! - Purpose: Codex, Claude Code and `OpenCode` get drivers that speak their
//!   own protocols, beside the ACP driver every agent has today.
//! - Choice: one seam for all of them, the one ACP already has
//!   ([`AgentSetup`](crate::AgentSetup), the commands behind
//!   [`AgentInput`](crate::AgentInput), the outputs behind
//!   [`AgentOutputs`](crate::AgentOutputs)); a driver is chosen when the
//!   agent is spawned ([`AgentProcess::spawn_with`](crate::AgentProcess::spawn_with)),
//!   and nothing above the seam knows which one runs.
//! - These seam types are frozen while the direct drivers are built in
//!   parallel (the plan, `docs/plans/agents-own-protocols.md`): a change to
//!   them is agreed first, since every driver implements them.

/// The protocol that drives an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Driver {
    /// The Agent Client Protocol, natively or through an adapter.
    #[default]
    Acp,
    /// Codex's app-server (JSON-RPC over stdio).
    CodexAppServer,
    /// Claude Code's stream-json over stdio.
    ClaudeStream,
    /// `OpenCode`'s HTTP API and event stream.
    OpencodeHttp,
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Acp => "ACP",
            Self::CodexAppServer => "Codex app-server",
            Self::ClaudeStream => "Claude Code stream-json",
            Self::OpencodeHttp => "OpenCode HTTP",
        })
    }
}
