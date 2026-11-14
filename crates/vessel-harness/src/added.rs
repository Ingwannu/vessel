//! Agents a node's owner added by their command (protocol 11, ADR 0032):
//! `agents.toml`, read by [`parse_agents`], written by [`add_agent`] and
//! [`remove_agent`], and the command one runs as ([`command`]).
//!
//! ```toml
//! [agents.goose]
//! label = "Goose"
//! program = "goose"         # a name found on the search path, or an absolute path
//! args = ["acp"]
//! env = { GOOSE_MODE = "auto", ANTHROPIC_API_KEY = { secret = "anthropic" } }
//! inherit = ["GOOSE_PROVIDER"]  # passed on from the node's own environment
//! mcp = ["docs"]            # the fleet's MCP servers it gets (default none)
//! source = { registry = "goose", version = "1.9.0" }
//! ```
//!
//! [Decision Log]
//! - Purpose: run any agent that speaks ACP, by the command its owner
//!   wrote, without Vessel guessing anything about it.
//! - Constraint: a definition is code run as the node's user, and an
//!   agent from the registry is often a package fetched from the network.
//!   It must get what it is given and nothing else of what the node holds.
//! - Alternatives: the node's environment, as the four get it (Paseo,
//!   GOAT): every key and token of the node would reach whatever the
//!   command is; a section of `profiles.toml` (one file, but a profile and
//!   an agent are different things, and the file's rules are ADR 0011's).
//! - Choice: a file of its own, parsed purely here (the node reads it,
//!   checks who may write it, and reloads it with `profiles.toml`); the
//!   command starts from an empty environment ([`command`]); errors name
//!   the agent and the key, never a value, as `profiles.toml`'s do.
//! - Cost: an agent that needs something of the node's environment beyond
//!   the system's own variables says so (`inherit`, `env`).

mod command;
mod edit;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use vessel_proto::agents::{
    AddedAgent as AddedInfo, AgentLabel, AgentProgram, AgentSource, InvalidAgentText, RegistryId,
    RegistryVersion, SecretEnv,
};
use vessel_proto::{AgentName, AgentNameError, ConfigValue, EnvName, McpServerName, SecretName};

pub(crate) use command::clean;
pub use command::{PASSED_ON, command, installer, passed_on};
pub use edit::{EditError, add_agent, remove_agent};

use crate::profile::{Position, expand, position, without_values};

/// The largest `agents.toml` the node reads: a reply listing every agent
/// stays well under a frame.
pub const MAX_FILE: usize = 256 * 1024;
/// The most agents one node defines.
pub(crate) const MAX_AGENTS: usize = 256;
/// The most arguments, variables, inherited variables or MCP servers of
/// one agent.
pub(crate) const MAX_ITEMS: usize = 64;
/// The longest argument, in bytes.
pub(crate) const MAX_ARG: usize = 1024;

/// An agent added on a node, as `agents.toml` defines it. `Debug` names
/// the variables it sets, never their values.
#[derive(Clone, PartialEq, Eq)]
pub struct AddedAgent {
    name: AgentName,
    label: Option<AgentLabel>,
    program: AgentProgram,
    args: Vec<String>,
    env: BTreeMap<EnvName, ConfigValue>,
    inherit: Vec<EnvName>,
    mcp: Vec<McpServerName>,
    source: AgentSource,
}

impl AddedAgent {
    /// Its name.
    #[must_use]
    pub const fn name(&self) -> &AgentName {
        &self.name
    }

    /// Its program as written (`~` expanded).
    #[must_use]
    pub const fn program(&self) -> &AgentProgram {
        &self.program
    }

    /// The fleet's MCP servers it gets.
    #[must_use]
    pub fn mcp(&self) -> &[McpServerName] {
        &self.mcp
    }

    /// The secrets its environment names, each once.
    pub fn secrets(&self) -> impl Iterator<Item = &SecretName> {
        self.env
            .values()
            .filter_map(|value| match value {
                ConfigValue::Secret(name) => Some(name),
                ConfigValue::Text(_) => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    /// What a node reports of it: its command whole, its environment by
    /// name.
    #[must_use]
    pub fn info(&self) -> AddedInfo {
        AddedInfo {
            label: self.label.clone(),
            program: self.program.clone(),
            args: self.args.clone(),
            env: self
                .env
                .iter()
                .filter(|(_name, value)| matches!(value, ConfigValue::Text(_)))
                .map(|(name, _value)| name.clone())
                .collect(),
            secrets: self
                .env
                .iter()
                .filter_map(|(variable, value)| match value {
                    ConfigValue::Secret(secret) => Some(SecretEnv {
                        variable: variable.clone(),
                        secret: secret.clone(),
                    }),
                    ConfigValue::Text(_) => None,
                })
                .collect(),
            inherit: self.inherit.clone(),
            mcp: self.mcp.clone(),
            source: self.source.clone(),
        }
    }
}

impl fmt::Debug for AddedAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddedAgent")
            .field("name", &self.name)
            .field("program", &self.program)
            .field("args", &self.args.len())
            .field("env", &self.env.keys().collect::<Vec<_>>())
            .field("inherit", &self.inherit)
            .field("mcp", &self.mcp)
            .finish_non_exhaustive()
    }
}

/// The agents a node defines, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddedAgents {
    agents: BTreeMap<AgentName, Arc<AddedAgent>>,
}

impl AddedAgents {
    /// The agent named `name`.
    #[must_use]
    pub fn get(&self, name: &AgentName) -> Option<&Arc<AddedAgent>> {
        self.agents.get(name)
    }

    /// Every agent, by name.
    pub fn all(&self) -> impl Iterator<Item = &Arc<AddedAgent>> {
        self.agents.values()
    }

    /// Whether an agent has `name`, ignoring case (agents' and profiles'
    /// names are one space, ADR 0032).
    #[must_use]
    pub fn has_name(&self, name: &str) -> bool {
        self.agents
            .keys()
            .any(|defined| defined.as_str().eq_ignore_ascii_case(name))
    }
}

/// `agents.toml` as typed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default)]
    agents: BTreeMap<String, RawAgent>,
}

/// One `[agents.<name>]` table; checked after typing, so errors can name
/// the agent and the key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAgent {
    #[serde(default)]
    label: Option<String>,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, ConfigValue>,
    #[serde(default)]
    inherit: Vec<String>,
    #[serde(default)]
    mcp: Vec<String>,
    #[serde(default)]
    source: Option<RawSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    registry: String,
    version: String,
}

/// `agents.toml` could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentsError {
    /// Larger than [`MAX_FILE`].
    #[error("agents.toml is larger than {MAX_FILE} bytes")]
    TooLarge,
    /// Not TOML, or not this file's shape. The message names the key and
    /// the position, never a value.
    #[error("agents.toml, {}: {message}", position.map_or_else(|| "unknown position".to_owned(), |at| at.to_string()))]
    Toml {
        /// Where, if the TOML library said.
        position: Option<Position>,
        /// The TOML library's message, without values.
        message: String,
    },
    /// More than [`MAX_AGENTS`] agents.
    #[error("agents.toml defines more than {MAX_AGENTS} agents")]
    TooMany,
    /// One agent is wrong.
    #[error("agents.toml, agent `{agent}`: {problem}")]
    Agent {
        /// Its name as written.
        agent: String,
        /// What is wrong with it.
        problem: AgentProblem,
    },
}

/// What is wrong with one agent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentProblem {
    /// The name breaks the rule, or names one of the four.
    #[error("{0}")]
    Name(AgentNameError),
    /// Another agent has the same name ignoring case.
    #[error("agent `{other}` has the same name (names are compared ignoring case)")]
    Duplicate {
        /// The other, as written.
        other: String,
    },
    /// A key's text breaks its rule.
    #[error("`{key}`: {invalid}")]
    Text {
        /// The key.
        key: &'static str,
        /// What is wrong.
        invalid: InvalidAgentText,
    },
    /// The program has a path separator but is not absolute.
    #[error("`program`: a path must be absolute (or a bare name found on the search path)")]
    RelativeProgram,
    /// A name in `env`, `inherit` or `mcp` is not one.
    #[error("`{key}`: `{name}` is not a valid name")]
    InvalidName {
        /// `env`, `inherit` or `mcp`.
        key: &'static str,
        /// The name given.
        name: String,
    },
    /// A variable is named twice, or both set and inherited (ignoring
    /// case, as Windows does), or a server twice.
    #[error("`{key}`: `{name}` is named twice (compared ignoring case)")]
    Repeated {
        /// `env`, `inherit` or `mcp`.
        key: &'static str,
        /// The second spelling.
        name: String,
    },
    /// More than [`MAX_ITEMS`] of one key.
    #[error("`{key}`: more than {MAX_ITEMS}")]
    TooMany {
        /// The key.
        key: &'static str,
    },
    /// An argument longer than [`MAX_ARG`] bytes.
    #[error("`args`: an argument is longer than {MAX_ARG} bytes")]
    LongArgument,
    /// A value starts with `~` and the node user's home is unknown.
    #[error(
        "`{key}`: `~` needs the node user's home directory, which is unknown or not valid text"
    )]
    NoHome {
        /// `program` or `args`.
        key: &'static str,
    },
}

/// Read `agents.toml` from its text. `user_home` is what `~/` expands to in
/// `program` and `args`.
///
/// Pure: the caller reads the file and checks who may write it.
///
/// # Errors
/// The first problem found, naming the agent and key; see [`AgentsError`].
pub fn parse_agents(text: &str, user_home: Option<&Path>) -> Result<AddedAgents, AgentsError> {
    if text.len() > MAX_FILE {
        return Err(AgentsError::TooLarge);
    }
    let raw: RawFile = toml_edit::de::from_str(text).map_err(|error| AgentsError::Toml {
        position: position(text, error.span()),
        message: without_values(error.message()),
    })?;
    if raw.agents.len() > MAX_AGENTS {
        return Err(AgentsError::TooMany);
    }
    let home = user_home.and_then(Path::to_str);
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut agents = BTreeMap::new();
    for (written, raw_agent) in raw.agents {
        let fail = |problem| AgentsError::Agent {
            agent: written.clone(),
            problem,
        };
        let name: AgentName = written
            .parse()
            .map_err(|error| fail(AgentProblem::Name(error)))?;
        if let Some(other) = seen.insert(written.to_ascii_lowercase(), written.clone()) {
            return Err(fail(AgentProblem::Duplicate { other }));
        }
        let agent = agent(name, raw_agent, home).map_err(fail)?;
        let _new = agents.insert(agent.name.clone(), Arc::new(agent));
    }
    Ok(AddedAgents { agents })
}

/// One agent's checks, after its name.
fn agent(name: AgentName, raw: RawAgent, home: Option<&str>) -> Result<AddedAgent, AgentProblem> {
    let text = |key: &'static str| move |invalid| AgentProblem::Text { key, invalid };
    let label = raw
        .label
        .as_deref()
        .map(str::parse::<AgentLabel>)
        .transpose()
        .map_err(text("label"))?;
    let expanded = expand(&raw.program, home).ok_or(AgentProblem::NoHome { key: "program" })?;
    let program: AgentProgram = expanded.parse().map_err(text("program"))?;
    if has_separator(program.as_str()) && !Path::new(program.as_str()).is_absolute() {
        return Err(AgentProblem::RelativeProgram);
    }
    if raw.args.len() > MAX_ITEMS {
        return Err(AgentProblem::TooMany { key: "args" });
    }
    let args = raw
        .args
        .iter()
        .map(|arg| {
            if arg.len() > MAX_ARG {
                return Err(AgentProblem::LongArgument);
            }
            expand(arg, home).ok_or(AgentProblem::NoHome { key: "args" })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut variables = Folded::default();
    if raw.env.len() > MAX_ITEMS {
        return Err(AgentProblem::TooMany { key: "env" });
    }
    let mut env = BTreeMap::new();
    for (written, value) in raw.env {
        let variable = variables.add("env", &written)?;
        let _new = env.insert(variable, value);
    }
    if raw.inherit.len() > MAX_ITEMS {
        return Err(AgentProblem::TooMany { key: "inherit" });
    }
    let inherit = raw
        .inherit
        .iter()
        .map(|written| variables.add("inherit", written))
        .collect::<Result<Vec<_>, _>>()?;
    if raw.mcp.len() > MAX_ITEMS {
        return Err(AgentProblem::TooMany { key: "mcp" });
    }
    let mut servers = BTreeSet::new();
    let mut mcp = Vec::new();
    for written in raw.mcp {
        let server: McpServerName =
            written
                .parse()
                .map_err(|_invalid| AgentProblem::InvalidName {
                    key: "mcp",
                    name: written.clone(),
                })?;
        if !servers.insert(written.to_ascii_lowercase()) {
            return Err(AgentProblem::Repeated {
                key: "mcp",
                name: written,
            });
        }
        mcp.push(server);
    }
    let source = match raw.source {
        None => AgentSource::Command,
        Some(RawSource { registry, version }) => AgentSource::Registry {
            id: registry
                .parse::<RegistryId>()
                .map_err(text("source.registry"))?,
            version: version
                .parse::<RegistryVersion>()
                .map_err(text("source.version"))?,
        },
    };
    Ok(AddedAgent {
        name,
        label,
        program,
        args,
        env,
        inherit,
        mcp,
        source,
    })
}

/// Whether `program` names a path rather than a program to look up. Both
/// separators count on every system, so a definition means the same
/// everywhere.
fn has_separator(program: &str) -> bool {
    program.contains(['/', '\\'])
}

/// The variable names seen across `env` and `inherit`, folded to one case
/// (Windows treats `Path` and `PATH` as one variable).
#[derive(Default)]
struct Folded(BTreeSet<String>);

impl Folded {
    fn add(&mut self, key: &'static str, written: &str) -> Result<EnvName, AgentProblem> {
        let variable: EnvName = written
            .parse()
            .map_err(|_invalid| AgentProblem::InvalidName {
                key,
                name: written.to_owned(),
            })?;
        if !self.0.insert(written.to_ascii_uppercase()) {
            return Err(AgentProblem::Repeated {
                key,
                name: written.to_owned(),
            });
        }
        Ok(variable)
    }
}

#[cfg(test)]
mod tests;
