//! Agents added on this node by their command (protocol 11, ADR 0032):
//! `agent.list`'s answer, and `agent.add` and `agent.remove`, which change
//! `agents.toml`.
//!
//! [Decision Log]
//! - Purpose: a person adds an agent from any client, and the node keeps
//!   it as configuration its owner could have written by hand.
//! - Constraint: the file holds commands the node runs as its own user;
//!   it must never be left in a state the node would refuse at its next
//!   start, and two changes must not lose each other.
//! - Choice: one change at a time (`NodeProfiles::editing`, a `tokio` lock
//!   held across the read, the checks and the write); the file is read as
//!   it is now (a hand edit included), changed in place through
//!   `toml_edit`, and parsed with `profiles.toml` before it is written, so
//!   what is written is what the node would read. Then the new set is put
//!   in effect and the probes kept for that agent are dropped.
//! - Who: only a person, by `admin` scope (the guard) and here: never an
//!   agent principal, whatever its scopes, and never a call delivered from
//!   another node (an added agent is this machine's).
//! - Cost: file I/O on a blocking thread, a few kilobytes each change.

use std::sync::Arc;

use vessel_harness::{AddedAgents, EditError, Profiles, add_agent, remove_agent};
use vessel_proto::agents::{
    AddAgent, AgentDefinition, AgentDetail, AgentInfo, AgentSource, AgentToAdd, AgentVariable,
    CatalogueEntry, CatalogueRun, InstallOffer, InstallPart, RegistryId, RegistryVersion,
    RemoveAgent, SignInHelp,
};
use vessel_proto::{
    AgentName, ConfigValue, ErrorCode, HarnessKind, HarnessRef, McpServerName, WireError,
};

use crate::catalogue::Catalogues;

use crate::error::ServiceError;
use crate::principal::Principal;
use crate::probes::Probes;
use crate::profiles::{NodeProfiles, ProfilesFileError, with_agents};

/// Why `agent.add` or `agent.remove` was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AddedError {
    /// Modern built-in identities cannot become legacy added-agent aliases.
    #[error("this name belongs to a built-in provider; choose another added-agent name")]
    ReservedProvider,
    /// A broken compiled manifest must not bypass the namespace admission rule.
    #[error("the provider namespace could not be checked")]
    ProviderPolicy(#[source] vessel_harness::providers::ManifestError),
    /// An agent, or a delegated call, may not change the node's agents.
    #[error(
        "only a person may add or remove an agent on this node (not an agent, nor a call from another node)"
    )]
    NotAPerson,
    /// The name is taken, ignoring case.
    #[error(
        "an agent named `{0}` is defined already (names are compared ignoring case); ask to replace it"
    )]
    Taken(String),
    /// No such agent.
    #[error("this node has no agent `{0}`")]
    NotFound(AgentName),
    /// `agents.toml` as it is now cannot be changed: it does not parse.
    #[error("agents.toml cannot be changed: {0}")]
    File(#[source] EditError),
    /// The definition breaks the file's rules (a bound, a relative path).
    #[error(transparent)]
    Invalid(vessel_harness::AgentsError),
    /// `profiles.toml` would no longer read with the change: a profile has
    /// the agent's name, or extends the agent being removed.
    #[error("profiles.toml would no longer read: {0}")]
    Profiles(#[source] vessel_harness::ProfilesError),
    /// The files could not be read or written.
    #[error(transparent)]
    Io(#[from] ProfilesFileError),
    /// `agents.toml` could not be written.
    #[error("cannot write agents.toml")]
    Write(#[source] vessel_store::StoreError),
    /// The blocking task that changes the file failed.
    #[error("the change of agents.toml did not complete")]
    Task(#[source] tokio::task::JoinError),
    /// The entry is not in the node's copy of the registry at that
    /// version.
    #[error(
        "the ACP registry's copy on this node has no `{id}` at {version} (`agent.catalogue` with refresh fetches it)"
    )]
    NotInCatalogue {
        id: RegistryId,
        version: RegistryVersion,
    },
    /// The entry runs no command here.
    #[error(
        "the ACP registry's `{0}` runs no command on this node: see what `agent.catalogue` says of it"
    )]
    NotACommand(RegistryId),
}

impl AddedError {
    pub(crate) const fn code(&self) -> ErrorCode {
        match self {
            Self::NotAPerson => ErrorCode::Forbidden,
            Self::Taken(_) | Self::Profiles(_) | Self::ReservedProvider => ErrorCode::Conflict,
            Self::NotFound(_) | Self::NotInCatalogue { .. } => ErrorCode::NotFound,
            Self::Invalid(_) | Self::NotACommand(_) => ErrorCode::InvalidRequest,
            Self::File(_)
            | Self::Io(_)
            | Self::Write(_)
            | Self::Task(_)
            | Self::ProviderPolicy(_) => ErrorCode::Internal,
        }
    }
}

/// Refuse anyone but a person: an agent (local or delegated) and any call
/// delivered from another node.
pub(crate) fn person(principal: &Principal) -> Result<(), AddedError> {
    match principal {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        Principal::Agent { .. } | Principal::Delegated { .. } => Err(AddedError::NotAPerson),
    }
}

/// `agent.list`: the four, then the agents added on this node by name.
pub(crate) fn list(profiles: &Profiles) -> Vec<AgentInfo> {
    HarnessKind::ALL
        .into_iter()
        .map(known)
        .chain(profiles.added().all().map(|agent| AgentInfo {
            agent: HarnessRef::Added(agent.name().clone()),
            detail: AgentDetail::Added {
                added: agent.info(),
            },
        }))
        .collect()
}

/// One of the four as `agent.list` shows it: its vendor's installs and how
/// a person signs in.
pub(crate) fn known(kind: HarnessKind) -> AgentInfo {
    let installs = vessel_harness::installs(kind)
        .into_iter()
        .map(|install| InstallOffer {
            part: match install.part {
                vessel_harness::InstallPart::Harness => InstallPart::Harness,
                vessel_harness::InstallPart::Adapter => InstallPart::Adapter,
            },
            command: install.command.to_owned(),
        })
        .collect();
    AgentInfo {
        agent: HarnessRef::Known(kind),
        detail: AgentDetail::Known {
            installs,
            sign_in: sign_in_help(kind),
        },
    }
}

/// How a person signs in to one of the four: its command and the
/// variable a profile sets for another account.
pub(crate) fn sign_in_help(kind: HarnessKind) -> SignInHelp {
    SignInHelp {
        command: vessel_harness::login_command(kind).map(|(binary, args)| {
            std::iter::once(binary)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ")
        }),
        home: vessel_harness::home_variable(kind).and_then(|name| name.parse().ok()),
        methods: Vec::new(),
    }
}

/// `agent.add`: define `request.name` as asked, and answer with it as now
/// in effect.
pub(crate) async fn add(
    profiles: &Arc<NodeProfiles>,
    probes: &Probes,
    catalogue: &Catalogues,
    request: AddAgent,
) -> Result<AgentInfo, AddedError> {
    let AddAgent {
        name,
        agent,
        replace,
    } = request;
    // Admission, not decoding: preserve existing saved names as distinct
    // legacy identities, while refusing new/replaced aliases before lookup or I/O.
    if vessel_harness::providers::builtin_named(name.as_str())
        .map_err(AddedError::ProviderPolicy)?
        .is_some()
    {
        return Err(AddedError::ReservedProvider);
    }
    let (definition, source) = match agent {
        AgentToAdd::Command { definition } => (definition, AgentSource::Command),
        AgentToAdd::Registry { id, version, mcp } => {
            let entry =
                catalogue
                    .entry(&id, &version)
                    .ok_or_else(|| AddedError::NotInCatalogue {
                        id: id.clone(),
                        version: version.clone(),
                    })?;
            (
                from_registry(entry, mcp)?,
                AgentSource::Registry { id, version },
            )
        }
    };
    let _editing = profiles.editing.lock().await;
    let changed = {
        let (profiles, name) = (Arc::clone(profiles), name.clone());
        tokio::task::spawn_blocking(move || {
            change(&profiles, |text| {
                add_agent(text, &name, &definition, &source, replace).map_err(|error| match error {
                    EditError::Taken(defined) => AddedError::Taken(defined),
                    other
                    @ (EditError::NotToml | EditError::NotATable | EditError::NotFound(_)) => {
                        AddedError::File(other)
                    }
                })
            })
        })
        .await
        .map_err(AddedError::Task)??
    };
    probes.forget(&HarnessRef::Added(name.clone()));
    let info = changed
        .added()
        .get(&name)
        .map(|agent| AgentInfo {
            agent: HarnessRef::Added(name.clone()),
            detail: AgentDetail::Added {
                added: agent.info(),
            },
        })
        .ok_or(AddedError::NotFound(name))?;
    Ok(info)
}

/// The definition an entry of the registry's copy runs: its command, its
/// variables as text, its name as the label (cut to a label's length).
fn from_registry(
    entry: CatalogueEntry,
    mcp: Vec<McpServerName>,
) -> Result<AgentDefinition, AddedError> {
    let CatalogueRun::Command {
        program, args, env, ..
    } = entry.runs
    else {
        return Err(AddedError::NotACommand(entry.id));
    };
    let label: String = entry.name.chars().take(80).collect();
    Ok(AgentDefinition {
        label: label.parse().ok(),
        program,
        args,
        env: env
            .into_iter()
            .map(|variable| AgentVariable {
                name: variable.name,
                value: ConfigValue::Text(variable.value),
            })
            .collect(),
        inherit: Vec::new(),
        mcp,
    })
}

/// `agent.remove`: forget `request.name`.
pub(crate) async fn remove(
    profiles: &Arc<NodeProfiles>,
    probes: &Probes,
    request: RemoveAgent,
) -> Result<(), AddedError> {
    let RemoveAgent { name } = request;
    let _editing = profiles.editing.lock().await;
    {
        let (profiles, name) = (Arc::clone(profiles), name.clone());
        let _changed = tokio::task::spawn_blocking(move || {
            change(&profiles, |text| {
                remove_agent(text, &name).map_err(|error| match error {
                    EditError::NotFound(name) => AddedError::NotFound(name),
                    other @ (EditError::NotToml | EditError::NotATable | EditError::Taken(_)) => {
                        AddedError::File(other)
                    }
                })
            })
        })
        .await
        .map_err(AddedError::Task)??;
    }
    probes.forget(&HarnessRef::Added(name));
    Ok(())
}

/// Read `agents.toml` as it is now, change its text with `edit`, check the
/// result reads with `profiles.toml`, write it, and put it in effect.
/// Blocks on the file system.
fn change(
    profiles: &NodeProfiles,
    edit: impl FnOnce(&str) -> Result<String, AddedError>,
) -> Result<Arc<Profiles>, AddedError> {
    let dir = profiles.dir();
    let text = dir
        .read_agents()
        .map_err(ProfilesFileError::ReadAgents)?
        .map(|file| file.text)
        .unwrap_or_default();
    let changed = edit(&text)?;
    let home = profiles.user_home();
    let added: AddedAgents =
        vessel_harness::parse_agents(&changed, Some(home)).map_err(AddedError::Invalid)?;
    let set = match with_agents(dir, home, added) {
        Ok(set) => set,
        Err(ProfilesFileError::Parse(error)) => return Err(AddedError::Profiles(error)),
        Err(other) => return Err(other.into()),
    };
    dir.write_agents(&changed).map_err(AddedError::Write)?;
    profiles.install(set);
    Ok(profiles.current())
}

impl From<AddedError> for ServiceError {
    fn from(error: AddedError) -> Self {
        Self::Added(error)
    }
}

/// The error a client gets.
pub(crate) fn wire(error: &AddedError) -> WireError {
    WireError::new(error.code(), error.to_string())
}
