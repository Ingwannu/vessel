//! `vessel agents …` (protocol 11, ADR 0032): the agents a node can run,
//! those its owner added by their command among them.

use clap::Subcommand;
use vessel_proto::agents::{AgentLabel, RegistryId};
use vessel_proto::{
    AgentName, EnvName, HarnessKind, HarnessRef, McpServerName, ProfileName, SecretName,
};

/// What to do with the node's agents; listing them when none is given.
#[derive(Debug, Subcommand)]
pub(crate) enum AgentsCommand {
    /// List the agents the node can run: the four Vessel knows, then those
    /// added on it, with each one's command.
    List,
    /// Add an agent that speaks ACP, by the command that starts it. The
    /// command runs on the node as the node's user, with an empty
    /// environment and only what is given here (`--env`, `--secret`,
    /// `--inherit`). Arguments are shown to every client and in the
    /// process table: put a key in a secret (`vessel fleet secret set`)
    /// and name it with `--secret`.
    Add(AddAgentArgs),
    /// Forget an added agent. Its running sessions keep running.
    Remove {
        /// The agent's name.
        name: AgentName,
    },
    /// Whether agents are installed and signed in: every agent's default
    /// profile, or one agent's profile. Nothing runs without `--refresh`.
    Status {
        /// Only this agent.
        agent: Option<HarnessRef>,
        /// Its profile (its default without one).
        #[arg(long, requires = "agent")]
        profile: Option<ProfileName>,
        /// Check now: the agent's status command, or its handshake (never
        /// a prompt).
        #[arg(long, requires = "agent")]
        refresh: bool,
    },
    /// Install one of the four (or its ACP adapter) on the node with its
    /// vendor's documented command: shown first, run after you say yes,
    /// its output printed as it runs.
    Install {
        /// claude-code, codex, gemini or opencode.
        agent: HarnessKind,
        /// Its ACP adapter instead of the agent.
        #[arg(long)]
        adapter: bool,
        /// Run it without asking.
        #[arg(long)]
        yes: bool,
    },
    /// The ACP registry, as the node's copy: what each entry would run.
    Catalogue {
        /// Fetch it first.
        #[arg(long)]
        refresh: bool,
    },
}

/// `vessel agents add`.
#[derive(Debug, clap::Args)]
pub(crate) struct AddAgentArgs {
    /// Its name (letters, digits, `.`, `_`, `-`; not one of the four's).
    pub(crate) name: AgentName,
    /// What to call it on the page.
    #[arg(long)]
    pub(crate) label: Option<AgentLabel>,
    /// A variable it is given, `NAME=VALUE` (repeatable). Not for secrets:
    /// use `--secret`.
    #[arg(long = "env", value_name = "NAME=VALUE", value_parser = variable)]
    pub(crate) env: Vec<(EnvName, String)>,
    /// A variable it is given from one of the node's secrets,
    /// `NAME=SECRET` (repeatable).
    #[arg(long = "secret", value_name = "NAME=SECRET", value_parser = secret)]
    pub(crate) secrets: Vec<(EnvName, SecretName)>,
    /// A variable passed on from the node's own environment (repeatable).
    #[arg(long)]
    pub(crate) inherit: Vec<EnvName>,
    /// One of the fleet's MCP servers it gets (repeatable); it gets none
    /// but Vessel's own otherwise.
    #[arg(long)]
    pub(crate) mcp: Vec<McpServerName>,
    /// Replace an agent of that name.
    #[arg(long)]
    pub(crate) replace: bool,
    /// Add an entry of the node's copy of the ACP registry (`vessel agents
    /// catalogue`) instead of a command: the node writes the command it
    /// showed, after you say yes.
    #[arg(long, value_name = "ID", conflicts_with_all = ["command", "env", "secrets", "inherit", "label"])]
    pub(crate) from_registry: Option<RegistryId>,
    /// Add the registry entry without asking (with `--from-registry`).
    #[arg(long)]
    pub(crate) yes: bool,
    /// The program (a name on the node's search path, or an absolute path)
    /// and its arguments, after `--`.
    #[arg(last = true, required_unless_present = "from_registry", num_args = 1..)]
    pub(crate) command: Vec<String>,
}

/// `NAME=VALUE`, split at the first `=`.
fn variable(text: &str) -> Result<(EnvName, String), String> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not NAME=VALUE"))?;
    let name = name.parse::<EnvName>().map_err(|error| error.to_string())?;
    Ok((name, value.to_owned()))
}

/// `NAME=SECRET`: a variable from one of the node's secrets.
fn secret(text: &str) -> Result<(EnvName, SecretName), String> {
    let (name, secret) = variable(text)?;
    let secret = secret
        .parse::<SecretName>()
        .map_err(|error| error.to_string())?;
    Ok((name, secret))
}
