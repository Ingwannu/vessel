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
//! - Each direct driver is a module of its own (`codex`, `claude`,
//!   `opencode`) with the same two entries as ACP's: `run`, the task that
//!   drives a session over the spawned process's stdio, and `history`. A
//!   driver whose module says it is not `BUILT` is refused before its
//!   agent starts ([`AgentError::NoDriver`](crate::AgentError::NoDriver)).
//! - These seam types are frozen while the direct drivers are built in
//!   parallel (the plan, `docs/plans/agents-own-protocols.md`): a change to
//!   them is agreed first, since every driver implements them.

use tokio::sync::mpsc;

use crate::error::Disconnect;
use crate::output::AgentOutput;

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

impl Driver {
    /// Whether the corresponding harness transport can replace and restore a
    /// transient system instruction without a user prompt or environment shim.
    #[must_use]
    pub const fn system_prompt_support(self) -> vessel_harness::SystemPromptSupport {
        use vessel_harness::DirectProtocol;
        let protocol = match self {
            Self::Acp => None,
            Self::CodexAppServer => Some(DirectProtocol::CodexAppServer),
            Self::ClaudeStream => Some(DirectProtocol::ClaudeStream),
            Self::OpencodeHttp => Some(DirectProtocol::OpencodeHttp),
        };
        vessel_harness::system_prompt_support(protocol)
    }

    /// Whether this build runs sessions with it.
    #[must_use]
    pub const fn built(self) -> bool {
        match self {
            Self::Acp => true,
            Self::CodexAppServer => crate::codex::BUILT,
            Self::ClaudeStream => crate::claude::BUILT,
            Self::OpencodeHttp => crate::opencode::BUILT,
        }
    }
}

/// What a driver that is not built answers, were it ever started.
pub(crate) async fn not_built(outputs: &mpsc::Sender<AgentOutput>, driver: Driver) {
    let _gone = outputs
        .send(AgentOutput::Disconnected(Disconnect::Protocol {
            message: format!("the {driver} driver is not built yet"),
        }))
        .await;
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use vessel_proc::CommandSpec;
    use vessel_proto::ResolvedMcpServers;

    use super::*;
    use crate::error::AgentError;
    use crate::handshake::AgentSetup;
    use crate::history::{HistoryError, HistoryFrom, history_with};
    use crate::probe::{ProbeError, probe_with};
    use crate::process::AgentProcess;

    fn setup() -> AgentSetup {
        AgentSetup {
            cwd: PathBuf::from("."),
            model: None,
            resume: None,
            fork: None,
            options: Vec::new(),
            mcp_servers: ResolvedMcpServers::new(),
            handshake_timeout: Duration::from_secs(1),
            questions: false,
            system_prompt: None,
            private_dir: None,
        }
    }

    /// A driver not built yet (Claude Code's, `OpenCode`'s) is refused
    /// before its agent starts (the program here does not exist, and is never looked for),
    /// for a session, a probe and a listing alike.
    #[tokio::test]
    async fn a_driver_not_built_is_refused_before_anything_starts() {
        let missing = CommandSpec::new("vessel-no-such-agent");
        for driver in [Driver::ClaudeStream, Driver::OpencodeHttp] {
            assert!(!driver.built(), "{driver}");
            let Err(AgentError::NoDriver(refused)) =
                AgentProcess::spawn_with(driver, &missing, setup())
            else {
                panic!("{driver} must be refused");
            };
            assert_eq!(refused, driver);
            assert!(matches!(
                probe_with(driver, &missing, setup()).await,
                Err(ProbeError::Spawn(AgentError::NoDriver(_)))
            ));
            assert!(matches!(
                history_with(
                    driver,
                    &missing,
                    None,
                    HistoryFrom::default(),
                    Duration::from_secs(1)
                )
                .await,
                Err(HistoryError::Spawn(AgentError::NoDriver(_)))
            ));
        }
        assert!(Driver::Acp.built());
        assert!(Driver::CodexAppServer.built(), "since Codex's slice 1");
    }
}
