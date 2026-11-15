//! What a new session is launched with: its resolved profile, its MCP
//! servers and where their file goes, and who created it. The seam between
//! `session.create` (and `session.move_profile`) and the session runtime
//! (`vessel_session::LaunchOptions`).
//!
//! [Decision Log: where MCP servers come from] (H3, node side)
//! - Purpose: every session starts with the fleet's MCP servers, their
//!   secrets filled in from this node (ADR 0012), and agent sessions also
//!   with the `vessel` server (ADR 0013). Everything after this point
//!   (ACP for structured sessions, the owner-only file or the environment
//!   for terminal ones) already carries resolved servers.
//! - Constraint: fleet sync (Track F) owns the definitions and the secrets
//!   and builds them after this module; the node must not depend on how
//!   they are stored.
//! - Alternatives: a closure (cannot be `Debug`, which the service and
//!   its parts derive); passing resolved servers in `Config` (fixed at
//!   start, while the fleet's definitions change with each pull).
//! - Choice: [`McpSource`], a small trait the launcher asks once per launch
//!   and per move. Fleet sync implements it over its definitions and
//!   secrets (`vessel_fleet::resolve`, `fleet::FleetMcp`), which is what
//!   the node is put together with (`node.rs`). Secrets it lacks are reported
//!   back; the session starts without the servers that need them, and says
//!   so in its log.
//! - Cost: the source is called on the async runtime, so it must not
//!   block: it answers from memory.
//!
//! [Decision Log: the `vessel` server] (H4, ADR 0013)
//! - Every session's agent gets one more stdio server, `vessel`: the
//!   node's `vessel mcp` command (`Config::vessel_mcp`), with the node's
//!   local endpoint, the session's id and its credential in that server's
//!   own environment. So the secret reaches only `vessel mcp`: through
//!   ACP for a structured session, the owner-only MCP file for Claude
//!   Code's terminal, and Codex's environment for a Codex terminal, where
//!   its name (`..._SECRET`) keeps Codex from passing it to the commands
//!   the agent runs (ADR 0012, as built). It replaces a fleet server of the
//!   same name.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::watch;
use vessel_harness::Profile;
use vessel_proto::mcp::{Injection, McpSettings, TOOL_POLICY_VARIABLE, ToolAccess, ToolPolicy};
use vessel_proto::{
    AgentSecret, Creator, HarnessRef, McpServerName, McpTransport, OriginBy, RemoteCreator,
    ResolvedMcpServers, ResolvedValue, SecretName, SessionId, SessionInfo, SessionOrigin,
    SessionSpec,
};
use vessel_session::{LaunchOptions, Relaunch};

use super::NodeProfiles;
use crate::error::ServiceError;
use crate::principal::Principal;

/// The MCP servers a session gets from one source, resolved.
#[derive(Debug, Default)]
pub(crate) struct McpServers {
    /// The servers, every value filled in.
    pub(crate) servers: ResolvedMcpServers,
    /// Secrets this node lacks, each once; the servers that need them are
    /// left out.
    pub(crate) missing: Vec<SecretName>,
}

/// Where sessions' MCP servers come from (see the module's Decision Log).
pub(crate) trait McpSource: fmt::Debug + Send + Sync {
    /// The servers a session of `spec` starts with: all, or `only` those
    /// named (an added agent's, ADR 0032). Called on the async runtime:
    /// must answer from memory, without blocking.
    fn servers(&self, spec: &SessionSpec, only: Option<&[McpServerName]>) -> McpServers;

    /// The node's value of secret `name` (`secrets.toml`, ADR 0012), for an
    /// added agent's environment. From memory, as [`McpSource::servers`].
    fn secret(&self, name: &SecretName) -> Option<ResolvedValue>;
}

/// No MCP servers, for tests of the launcher alone (the node launches with
/// fleet sync's source).
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoMcpServers;

#[cfg(test)]
impl McpSource for NoMcpServers {
    fn servers(&self, _spec: &SessionSpec, _only: Option<&[McpServerName]>) -> McpServers {
        McpServers::default()
    }

    fn secret(&self, _name: &SecretName) -> Option<ResolvedValue> {
        None
    }
}

/// The `vessel` server's variables. `vessel-mcp` reads the same names;
/// its end-to-end test fails if they drift apart.
const SOCKET_VARIABLE: &str = "VESSEL_NODE_SOCKET";
const SESSION_VARIABLE: &str = "VESSEL_AGENT_SESSION";
const SECRET_VARIABLE: &str = "VESSEL_AGENT_SECRET";

/// The `vessel` server's name in every session.
const VESSEL_SERVER: &str = "vessel";

/// How sessions' agents reach this node: `vessel mcp` (ADR 0013).
#[derive(Debug, Clone)]
pub(crate) struct AgentTools {
    /// `vessel` (or what runs `vessel mcp`).
    pub(crate) program: PathBuf,
    /// `["mcp"]`.
    pub(crate) args: Vec<String>,
    /// The node's local socket or pipe name.
    pub(crate) socket: PathBuf,
}

/// Everything `session.create` adds to the spec.
#[derive(Debug)]
pub(crate) struct Launcher {
    /// Owner-only, under the data directory (`DataDir::prepare_mcp_dir`).
    mcp_dir: PathBuf,
    /// The fleet's MCP servers.
    mcp: Arc<dyn McpSource>,
    /// The `vessel` server, when the node knows how to run it.
    tools: Option<AgentTools>,
    tool_policy: Option<watch::Receiver<McpSettings>>,
    limit_environment: vessel_harness::LimitEnvironment,
}

impl Launcher {
    /// A launcher writing MCP files in `mcp_dir`, with `mcp`'s servers,
    /// and `tools` as every session's `vessel` server.
    pub(crate) fn new(
        mcp_dir: PathBuf,
        mcp: Arc<dyn McpSource>,
        tools: Option<AgentTools>,
    ) -> Self {
        Self {
            mcp_dir,
            mcp,
            tools,
            tool_policy: None,
            limit_environment: vessel_harness::LimitEnvironment::new(),
        }
    }

    /// Capture only the binary's nonsecret route facts, never credentials.
    pub(crate) fn with_limit_environment(
        mut self,
        environment: vessel_harness::LimitEnvironment,
    ) -> Self {
        self.limit_environment = environment;
        self
    }

    /// Resolve the exact profile used for a launch without any file I/O.
    pub(crate) fn limits(
        &self,
        profiles: &NodeProfiles,
        harness: &HarnessRef,
        profile: Option<&Profile>,
    ) -> vessel_session::LimitAssociation {
        let capture = |profile: &Profile| {
            vessel_harness::limits_binding(&self.limit_environment, profiles.env(), profile).into()
        };
        match profile {
            Some(profile) => capture(profile),
            None => profiles.default_of(harness).map_or(
                vessel_session::LimitAssociation::Refused(
                    vessel_proto::limits::LimitFailure::Unsupported,
                ),
                |profile| capture(&profile),
            ),
        }
    }

    pub(crate) fn account_binding(
        &self,
        profiles: &NodeProfiles,
        profile: &Profile,
    ) -> Result<vessel_harness::LimitBinding, vessel_proto::limits::LimitFailure> {
        vessel_harness::limits_binding(&self.limit_environment, profiles.env(), profile)
    }

    /// Subscribe to node choices; each launch below freezes its own exact policy.
    pub(crate) fn with_tool_policy(mut self, policy: watch::Receiver<McpSettings>) -> Self {
        self.tool_policy = Some(policy);
        self
    }

    /// Freeze before credential creation, so a live settings change cannot mix
    /// one launch's injection decision with another launch's visibility policy.
    pub(crate) fn tools_for(&self, harness: &HarnessRef) -> Option<ToolPolicy> {
        let _configured = self.tools.as_ref()?;
        let Some(source) = &self.tool_policy else {
            return Some(ToolPolicy::default());
        };
        let settings = source.borrow();
        if settings.inject == Injection::Off {
            return None;
        }
        let policy = settings.policy.for_harness(harness);
        (policy.access() == ToolAccess::Enabled).then_some(policy)
    }

    /// Add the `vessel` server for session `session`, holding `secret`.
    pub(crate) fn attach_tools(
        &self,
        servers: &mut ResolvedMcpServers,
        session: SessionId,
        secret: &AgentSecret,
        policy: &ToolPolicy,
    ) -> Result<(), ServiceError> {
        let Some(tools) = &self.tools else {
            return Ok(());
        };
        let name = |text: &str| text.parse().map_err(ServiceError::AgentTools);
        let socket = tools
            .socket
            .to_str()
            .ok_or(ServiceError::AgentToolsPath)?
            .to_owned();
        let encoded_policy = serde_json::to_string(policy).map_err(ServiceError::ToolPolicy)?;
        let env = BTreeMap::from([
            (
                name(TOOL_POLICY_VARIABLE)?,
                ResolvedValue::new(encoded_policy),
            ),
            (name(SOCKET_VARIABLE)?, ResolvedValue::new(socket)),
            (
                name(SESSION_VARIABLE)?,
                ResolvedValue::new(session.to_string()),
            ),
            (
                name(SECRET_VARIABLE)?,
                ResolvedValue::new(secret.to_string()),
            ),
        ]);
        let server: McpServerName = VESSEL_SERVER.parse().map_err(ServiceError::AgentTools)?;
        let replaced = servers.insert(
            server,
            McpTransport::Stdio {
                command: tools.program.clone(),
                args: tools.args.clone(),
                env,
            },
        );
        if replaced.is_some() {
            tracing::warn!("a fleet MCP server named `vessel` is replaced by the node's own");
        }
        Ok(())
    }

    /// The options for `spec`, created by `principal`.
    ///
    /// # Errors
    /// The spec names a profile this node does not have, or one of another
    /// harness.
    pub(crate) fn options(
        &self,
        profiles: &NodeProfiles,
        spec: &SessionSpec,
        principal: &Principal,
    ) -> Result<LaunchOptions, ServiceError> {
        let profile = match &spec.profile {
            None => None,
            Some(name) => {
                let profile = profiles
                    .get(name)
                    .ok_or_else(|| ServiceError::UnknownProfile(name.clone()))?;
                if spec.harness != profile.harness() {
                    return Err(ServiceError::ProfileHarness {
                        profile: name.clone(),
                        profile_harness: profile.harness(),
                        harness: spec.harness.clone(),
                    });
                }
                Some(profile)
            }
        };
        let profile = self.added(profiles, &spec.harness, profile)?;
        let (mcp_servers, notices) = self.mcp_servers(spec, profile.as_ref());
        let created_by = creator(principal);
        let limits = self.limits(profiles, &spec.harness, profile.as_ref());
        Ok(LaunchOptions {
            limits,
            profile,
            mcp_servers,
            mcp_dir: Some(self.mcp_dir.clone()),
            questions: watched(&created_by),
            created_by,
            notices,
            id: None,
            // An import (F4) sets its own origin.
            origin: origin(principal),
            via: match principal {
                Principal::Delegated { via, .. } => Some(*via),
                Principal::Owner | Principal::Device { .. } | Principal::Agent { .. } => None,
            },
            // A routed profile's are the node's to fill in, once the
            // session's id is known (`Models::launch`).
            model: None,
            model_proxy: None,
            fork: None,
            title_origin: None,
        })
    }

    /// What starting `info`'s ended session again runs with (protocol 10's
    /// `session.resume`, ADR 0026): its current profile (a move may have
    /// changed it), the fleet's MCP servers, and questions as for a new
    /// session of its creator, whose `created_by` stays. The `vessel`
    /// server and a routed profile's proxy are the caller's to add, once
    /// a credential and a token are minted for the new run.
    ///
    /// # Errors
    /// Its profile is no longer defined, or is now another harness's.
    pub(crate) fn again(
        &self,
        profiles: &NodeProfiles,
        info: &SessionInfo,
    ) -> Result<Relaunch, ServiceError> {
        let profile = match &info.profile {
            None => None,
            Some(name) => {
                let profile = profiles
                    .get(name)
                    .ok_or_else(|| ServiceError::ProfileRemoved {
                        session: info.id,
                        profile: name.clone(),
                    })?;
                if info.spec.harness != profile.harness() {
                    return Err(ServiceError::ProfileHarness {
                        profile: name.clone(),
                        profile_harness: profile.harness(),
                        harness: info.spec.harness.clone(),
                    });
                }
                Some(profile)
            }
        };
        let profile = self.added(profiles, &info.spec.harness, profile)?;
        let (mcp_servers, notices) = self.mcp_servers(&info.spec, profile.as_ref());
        let limits = self.limits(profiles, &info.spec.harness, profile.as_ref());
        Ok(Relaunch {
            limits,
            profile,
            mcp_servers,
            notices,
            model_proxy: None,
            questions: watched(&info.created_by),
        })
    }

    /// An added agent's profile, ready to launch (ADR 0032): its default
    /// when none is named (the session runtime knows only the four's), and
    /// the value of every secret its environment names. A profile of one
    /// of the four is returned as it is.
    ///
    /// # Errors
    /// The agent is not defined on this node, or a secret it names is not
    /// set: it would start without what its definition says it needs.
    pub(crate) fn added(
        &self,
        profiles: &NodeProfiles,
        harness: &HarnessRef,
        profile: Option<Profile>,
    ) -> Result<Option<Profile>, ServiceError> {
        let profile = match (profile, harness) {
            (Some(profile), _) => profile,
            (None, HarnessRef::Known(_)) => return Ok(None),
            (None, HarnessRef::Added(name)) => profiles
                .default_of(harness)
                .ok_or_else(|| ServiceError::UnknownAgent(name.clone()))?,
        };
        let Some(agent) = profile.added().map(|agent| agent.name().clone()) else {
            return Ok(Some(profile));
        };
        profile
            .with_secrets(|name| self.mcp.secret(name))
            .map(Some)
            .map_err(|secret| ServiceError::AgentSecret { agent, secret })
    }

    /// The servers a session of `spec` under `profile` starts with, and
    /// the lines its log gets about servers left out. An added agent gets
    /// only the fleet's servers its definition names (ADR 0032): a server's
    /// definition holds its secret values.
    pub(crate) fn mcp_servers(
        &self,
        spec: &SessionSpec,
        profile: Option<&Profile>,
    ) -> (ResolvedMcpServers, Vec<String>) {
        let only = profile.and_then(Profile::added).map(|agent| agent.mcp());
        let McpServers { servers, missing } = self.mcp.servers(spec, only);
        if !missing.is_empty() {
            tracing::warn!(
                ?missing,
                "secrets missing on this node; the MCP servers that need them are left out"
            );
        }
        (servers, missing.iter().map(missing_secret).collect())
    }
}

/// The log line for a missing secret: its name, never a value.
fn missing_secret(name: &SecretName) -> String {
    format!(
        "vessel: secret {name} is not set on this node; MCP servers that need it were left out \
         (vessel fleet secret set {name})"
    )
}

/// Who a caller is, as a session records it (ADR 0013). A delivered call
/// (protocol 6) is recorded as the origin's agent, a variant protocol 5
/// readers know, or else as the delivering device, which is true here;
/// [`origin`] keeps the rest.
pub(crate) fn creator(principal: &Principal) -> Creator {
    match principal {
        Principal::Owner => Creator::Owner,
        Principal::Device { id, .. } => Creator::Device { id: *id },
        Principal::Agent { node, session, .. } => Creator::Agent {
            node: *node,
            session: *session,
        },
        Principal::Delegated { device, origin, .. } => match origin.by {
            OriginBy::Agent { session, .. } => Creator::Agent {
                node: origin.node,
                session,
            },
            OriginBy::Owner | OriginBy::Device { .. } => Creator::Device { id: *device },
        },
    }
}

/// Whether a session `creator` starts may ask questions (protocol 10,
/// review item 4 of milestone 11's contract): only one a person watches.
///
/// [Decision Log]
/// - Purpose: advertising questions turns on Claude's `AskUserQuestion`
///   (its adapter disables the tool for a client without form
///   elicitation). A session an agent created (ADR 0013) runs unwatched:
///   a question there would wait for an answer nobody gives, and before
///   protocol 10 such sessions never asked, they went on.
/// - Choice: the owner's and a device's sessions (a delivered call of
///   either too) ask; an agent's, here or delivered from another node,
///   does not. Decided once, at creation: a move keeps it.
/// - Cost: an agent that wants its child session to ask cannot; it reads
///   the child's turns and answers in the next prompt instead.
pub(crate) const fn watched(creator: &Creator) -> bool {
    match creator {
        Creator::Owner | Creator::Device { .. } => true,
        Creator::Agent { .. } => false,
    }
}

/// Where a session a caller creates comes from: another node, for a
/// delivered call of its owner or one of its devices (an agent's is in
/// `created_by` already).
fn origin(principal: &Principal) -> Option<SessionOrigin> {
    let Principal::Delegated { origin, .. } = principal else {
        return None;
    };
    let by = match origin.by {
        OriginBy::Owner => RemoteCreator::Owner,
        OriginBy::Device { id } => RemoteCreator::Device { id },
        OriginBy::Agent { .. } => return None,
    };
    Some(SessionOrigin::Remote {
        node: origin.node,
        by,
    })
}

#[cfg(test)]
mod tests;
