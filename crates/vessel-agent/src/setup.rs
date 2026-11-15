//! Typed launch-only setup, separate from the ACP handshake state machine.
//! Keeping the input contract here lets new launch policy avoid growing protocol logic.

use crate::handshake::ForkFrom;
use std::path::PathBuf;
use std::time::Duration;
use vessel_proto::{HarnessSessionId, ModelId, OptionChoice, ResolvedMcpServers};

/// What the session should start with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSetup {
    /// Complete transient replacement of Vessel's supplemental instruction layer;
    /// the vendor-native base stays intact. None uses the captured launch context.
    /// A supported transport must prove replacement and restoration on resume.
    /// This field neither captures nor guesses that context, and is never kept
    /// in persistent specs or user logs.
    pub system_prompt: Option<crate::SystemPromptOverride>,
    /// Working directory of the ACP session (absolute).
    pub cwd: PathBuf,
    /// Model to select, if not the agent's default.
    pub model: Option<ModelId>,
    /// Resume this harness session (`session/load`) instead of starting a
    /// new one.
    pub resume: Option<HarnessSessionId>,
    /// Fork this conversation (`session/fork`) instead of starting a new
    /// one or resuming one (protocol 14, ADR 0052); wins over `resume`.
    /// Boxed: rare, and a setup is carried by every start's future.
    pub fork: Option<Box<ForkFrom>>,
    /// The agent's options to set once the session exists, in order, after
    /// the model (protocol 9). One the agent does not offer, or refuses,
    /// fails the handshake.
    pub options: Vec<OptionChoice>,
    /// MCP servers the session gets, in `session/new` or `session/load`.
    /// Values reach the agent over its stdin, never its command line; an
    /// HTTP server is left out (and logged) if the agent cannot take HTTP.
    pub mcp_servers: ResolvedMcpServers,
    /// Longest the whole handshake may take (normally
    /// [`crate::DEFAULT_HANDSHAKE_TIMEOUT`]); past it the connection ends with
    /// `Disconnect::Handshake`.
    pub handshake_timeout: Duration,
    /// Whether the agent may ask the person questions (protocol 10): ACP's
    /// form elicitation is advertised in `initialize` only then, and Claude's
    /// adapter offers its question tool only to a client that advertises it.
    /// False unless someone watches the session (the node's choice: a
    /// session the owner or a device created); a probe never asks.
    pub questions: bool,
    /// The node's own owner-only directory for a launch's private files
    /// (`DataDir::prepare_mcp_dir`: created owner-only and emptied of a
    /// previous run's files at every node start, ADR 0012). A direct
    /// driver whose agent takes something only from a file at launch
    /// (Claude Code's `--mcp-config`) writes it there, through the
    /// harness's own writer (`vessel_harness::McpFile`), and nowhere else;
    /// `None` means nothing may be written, and such a launch that needs a
    /// file is refused before its agent starts, never started without it.
    /// A session's launches carry it; a probe and a listing do not. The
    /// ACP driver never reads it (its servers go over ACP).
    pub private_dir: Option<PathBuf>,
}
