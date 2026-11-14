//! Added agents as the wire carries them (protocol 11, ADR 0032): what a
//! node's `agents.toml` holds, as clients see it and as `agent.add` writes
//! it, and what `agent.list` says of each agent.

use serde::{Deserialize, Serialize};

use super::name::HarnessRef;
use super::text::{AgentLabel, AgentProgram, AuthMethodId, RegistryId, RegistryVersion};
use crate::fleet::ConfigValue;
use crate::name::{EnvName, McpServerName, SecretName};

/// One agent as `agent.list` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentInfo {
    /// Which agent.
    pub agent: HarnessRef,
    /// What the node knows of it.
    pub detail: AgentDetail,
}

/// What a node knows of one agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentDetail {
    /// One of the four: how it is installed (each command shown before it
    /// runs, `agent.install`) and how a person signs in to it.
    Known {
        /// Its vendor's documented installs: the agent, and its ACP adapter
        /// when it needs one.
        installs: Vec<InstallOffer>,
        /// How a person signs in.
        sign_in: SignInHelp,
    },
    /// Added on this node by its command.
    Added {
        /// Its definition, values left out.
        added: AddedAgent,
    },
}

/// An added agent as clients see it: its command whole, its environment by
/// name. Values never leave the node (as for profiles, ADR 0011): a text
/// value may be a credential someone wrote there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AddedAgent {
    /// What a person calls it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<AgentLabel>,
    /// Its program as written.
    pub program: AgentProgram,
    /// Its arguments as written (`~` expanded).
    #[serde(default)]
    pub args: Vec<String>,
    /// The variables it sets to text, by name.
    #[serde(default)]
    pub env: Vec<EnvName>,
    /// The variables it takes from the node's secrets.
    #[serde(default)]
    pub secrets: Vec<SecretEnv>,
    /// The variables passed on from the node's own environment.
    #[serde(default)]
    pub inherit: Vec<EnvName>,
    /// The fleet's MCP servers it gets, by name (ADR 0012); it gets no
    /// other but the `vessel` server.
    #[serde(default)]
    pub mcp: Vec<McpServerName>,
    /// Where its definition came from.
    pub source: AgentSource,
}

/// A variable an added agent gets from one of the node's secrets
/// (`secrets.toml`, ADR 0012).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SecretEnv {
    /// The variable.
    pub variable: EnvName,
    /// The secret it is set from.
    pub secret: SecretName,
}

/// Where an added agent's definition came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "from", rename_all = "snake_case")]
pub enum AgentSource {
    /// A person wrote the command.
    Command,
    /// An entry of the ACP registry, as the node's copy had it.
    Registry {
        /// The entry's id.
        id: RegistryId,
        /// The entry's version.
        version: RegistryVersion,
    },
}

/// What `agent.add` writes when a person gives the command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentDefinition {
    /// What a person calls it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<AgentLabel>,
    /// Its program: a name found on the node's search path, or an absolute
    /// path.
    pub program: AgentProgram,
    /// Its arguments. Every client sees them, and the process table shows
    /// them: a secret belongs in `env` as a secret's name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// The variables it sets: text, or a secret of the node by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<AgentVariable>,
    /// The variables passed on from the node's own environment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherit: Vec<EnvName>,
    /// The fleet's MCP servers it gets, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<McpServerName>,
}

/// One variable an added agent sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentVariable {
    /// The variable.
    pub name: EnvName,
    /// Its value: text, or `{ secret = "NAME" }`.
    pub value: ConfigValue,
}

/// What `agent.add` adds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "from", rename_all = "snake_case")]
pub enum AgentToAdd {
    /// The command a person wrote.
    Command {
        /// The definition.
        definition: AgentDefinition,
    },
    /// An entry of the node's copy of the ACP registry, at this version:
    /// the node builds the definition from its own copy, so what runs is
    /// what the catalogue showed.
    Registry {
        /// The entry.
        id: RegistryId,
        /// Its version, as the catalogue showed it.
        version: RegistryVersion,
        /// The fleet's MCP servers it gets.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        mcp: Vec<McpServerName>,
    },
}

/// One install a known agent's vendor documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InstallOffer {
    /// What it installs.
    pub part: InstallPart,
    /// The whole command, as a person would type it, or the step a person
    /// takes (`by_hand`).
    pub command: String,
    /// A command or step a person runs themselves (a script piped to a
    /// shell, a download): the node shows it and refuses to run it
    /// ([`READS_TEN_AGENTS`](crate::agents::READS_TEN_AGENTS), ADR 0058;
    /// left out when false).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub by_hand: bool,
}

/// What an install puts in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum InstallPart {
    /// The agent itself.
    Harness,
    /// Its ACP adapter, for structured sessions.
    Adapter,
}

/// How a person signs in to an agent. The node only says how (ADR 0032):
/// a login acts as the account, and runs where its account is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SignInHelp {
    /// The agent's own command, to run on that machine as the profile's
    /// account; absent when it signs in inside its own interface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The variable that chooses the account (`CLAUDE_CONFIG_DIR`), by
    /// name: its value is the profile's, in `profiles.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<EnvName>,
    /// The sign-in methods its last handshake offered (ACP `authMethods`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<AuthMethod>,
}

/// A sign-in method an agent offers in its handshake (ACP `authMethods`, of
/// the `agent` type: Vessel advertises no other, so it is offered no
/// other). The node keeps at most [`AuthMethod::MAX`], each cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AuthMethod {
    /// Its id, as the agent gave it.
    pub id: AuthMethodId,
    /// Its name, in the agent's words (cut at [`AuthMethod::MAX_NAME`]
    /// characters).
    pub name: String,
    /// What it does, in the agent's words (cut at
    /// [`AuthMethod::MAX_DESCRIPTION`] characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AuthMethod {
    /// How many methods of one agent the node keeps.
    pub const MAX: usize = 16;
    /// A name's length, in characters.
    pub const MAX_NAME: usize = 128;
    /// A description's length, in characters.
    pub const MAX_DESCRIPTION: usize = 512;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_round_trips_with_text_and_secrets() {
        let definition = AgentDefinition {
            label: Some("Goose".parse().unwrap()),
            program: "goose".parse().unwrap(),
            args: vec!["acp".to_owned()],
            env: vec![
                AgentVariable {
                    name: "GOOSE_MODE".parse().unwrap(),
                    value: ConfigValue::Text("auto".to_owned()),
                },
                AgentVariable {
                    name: "ANTHROPIC_API_KEY".parse().unwrap(),
                    value: ConfigValue::Secret("anthropic".parse().unwrap()),
                },
            ],
            inherit: Vec::new(),
            mcp: Vec::new(),
        };
        let add = AgentToAdd::Command {
            definition: definition.clone(),
        };
        let json = serde_json::to_string(&add).unwrap();
        assert!(json.starts_with(r#"{"from":"command""#), "{json}");
        assert!(json.contains(r#"{"secret":"anthropic"}"#), "{json}");
        assert_eq!(serde_json::from_str::<AgentToAdd>(&json).unwrap(), add);
    }

    #[test]
    fn sources_are_tagged() {
        let source = AgentSource::Registry {
            id: "goose".parse().unwrap(),
            version: "1.9.0".parse().unwrap(),
        };
        assert_eq!(
            serde_json::to_string(&source).unwrap(),
            r#"{"from":"registry","id":"goose","version":"1.9.0"}"#
        );
        assert_eq!(
            serde_json::to_string(&AgentSource::Command).unwrap(),
            r#"{"from":"command"}"#
        );
    }
}
