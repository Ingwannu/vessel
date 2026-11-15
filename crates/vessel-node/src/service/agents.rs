//! Protocol 11's agents (ADR 0032): agents beyond the four, as the node's
//! service routes them. Routing only, as the rest of the service: the
//! definitions and their file are `crate::added`'s, launching is the
//! launcher's.

use vessel_harness::{Profile, Profiles};
use vessel_proto::agents::{
    AddAgent, AgentInfo, Catalogue, GetCatalogue, InstallAgent, InstallJob, InstallOutput,
    InstallPart, ListAgents, ReadInstall, RemoveAgent,
};
use vessel_proto::{
    Ack, ErrorCode, Harness, HarnessRef, InstallId, ProfileName, SessionId, WireError,
};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::transfers::export::named_in;

impl NodeService {
    /// Refuse a call that would act on, or subscribe to, an added agent's
    /// session for a client of protocol 2 to 10, before it runs: the
    /// client could not decode that session, so it must not change it
    /// unseen (its replies are withheld too, `Reply::withheld`). A session
    /// this node does not have passes, for the call itself to answer.
    pub(crate) async fn refuse_older(
        &self,
        connection: &Connection,
        session: SessionId,
    ) -> Result<(), WireError> {
        let protocol = connection.protocol();
        if protocol >= vessel_proto::agents::READS_ADDED_AGENTS {
            return Ok(());
        }
        match self.sessions.get(session).await {
            Ok(info) if !vessel_proto::agents::readable(&info, protocol) => {
                Err(vessel_proto::agents::unreadable())
            }
            Ok(_) | Err(ServiceError::NotFound(_)) => Ok(()),
            Err(error) => Err(wire(&error)),
        }
    }

    /// `agent.list`: the four, then the agents added here, by name.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn list_agents(
        &self,
        _: &Connection,
        _: ListAgents,
    ) -> Result<Vec<AgentInfo>, WireError> {
        Ok(crate::added::list(&self.profiles.current()))
    }

    /// `agent.add`: a person only (the guard checked `admin`; an agent
    /// principal holding it is refused here).
    pub(crate) async fn add_agent(
        &self,
        connection: &Connection,
        request: AddAgent,
    ) -> Result<AgentInfo, WireError> {
        crate::added::person(connection.principal()).map_err(|error| crate::added::wire(&error))?;
        let name = request.name.clone();
        let added = crate::added::add(&self.profiles, &self.probes, &self.catalogue, request)
            .await
            .map_err(|error| crate::added::wire(&error))?;
        self.readiness.forget(&HarnessRef::Added(name));
        Ok(added)
    }

    /// `agent.remove`: as `agent.add`. A live session of the agent goes on
    /// with the command it started with; a profile extending it makes the
    /// removal a conflict (`profiles.toml` would no longer read).
    pub(crate) async fn remove_agent(
        &self,
        connection: &Connection,
        request: RemoveAgent,
    ) -> Result<Ack, WireError> {
        crate::added::person(connection.principal()).map_err(|error| crate::added::wire(&error))?;
        let name = request.name.clone();
        crate::added::remove(&self.profiles, &self.probes, request)
            .await
            .map_err(|error| crate::added::wire(&error))?;
        self.readiness.forget(&HarnessRef::Added(name));
        Ok(Ack)
    }

    /// Refuse to start `harness` while an install of it runs.
    pub(crate) fn refuse_installing(&self, harness: &HarnessRef) -> Result<(), ServiceError> {
        match harness {
            HarnessRef::Known(kind) if self.installs.running(*kind) => {
                Err(ServiceError::Installing(*kind))
            }
            HarnessRef::Known(_) | HarnessRef::Added(_) => Ok(()),
        }
    }

    /// `agent.install`: the vendor's documented command for one of the
    /// four, run on the node for a person (module `install`).
    pub(crate) async fn install_agent(
        &self,
        connection: &Connection,
        request: InstallAgent,
    ) -> Result<InstallJob, WireError> {
        crate::added::person(connection.principal()).map_err(|error| crate::added::wire(&error))?;
        let InstallAgent { agent, part } = request;
        let wanted = match part {
            InstallPart::Harness => vessel_harness::InstallPart::Harness,
            InstallPart::Adapter => vessel_harness::InstallPart::Adapter,
        };
        let install = vessel_harness::installs(agent)
            .into_iter()
            .find(|install| install.part == wanted)
            .ok_or_else(|| {
                WireError::new(
                    ErrorCode::InvalidRequest,
                    format!(
                        "{agent} has no such part to install (`agent.list` shows its installs)"
                    ),
                )
            })?;
        if self
            .sessions
            .live_profiles()
            .iter()
            .any(|(running, _profile)| *running == agent)
        {
            return Err(WireError::new(
                ErrorCode::Conflict,
                format!(
                    "{agent} has live sessions on this node: an install replaces what they run; stop them first"
                ),
            ));
        }
        let (lookup, env) = (self.lookup.clone(), self.profiles.env().clone());
        let command = tokio::task::spawn_blocking(move || {
            vessel_harness::installer(
                &install,
                &lookup,
                &env,
                |name| std::env::var_os(name),
                std::env::vars_os(),
            )
            .map(|command| (command, install))
        })
        .await
        .map_err(|error| wire(&ServiceError::Task(error)))?;
        let Some((command, install)) = command else {
            return Err(WireError::new(
                ErrorCode::HarnessNotInstalled,
                "the installer's program (`npm`) is not on the node's search path",
            ));
        };
        let job = InstallJob {
            id: InstallId::random(),
            agent,
            part,
            command: install.command.to_owned(),
            started_at: chrono::Utc::now(),
        };
        self.installs
            .start(job, command)
            .map_err(|error| WireError::new(ErrorCode::Conflict, error.to_string()))
    }

    /// `agent.install_output`: a page of the last install's output.
    pub(crate) async fn install_output(
        &self,
        _: &Connection,
        request: ReadInstall,
    ) -> Result<InstallOutput, WireError> {
        self.installs.read(request).await
    }

    /// `agent.catalogue`: the node's copy of the ACP registry; `refresh`
    /// fetches it first, which only a person asks for.
    pub(crate) async fn agent_catalogue(
        &self,
        connection: &Connection,
        request: GetCatalogue,
    ) -> Result<Catalogue, WireError> {
        if !request.refresh {
            return Ok(self.catalogue.current());
        }
        crate::added::person(connection.principal()).map_err(|_not| {
            WireError::new(
                ErrorCode::Forbidden,
                "a fetch of the ACP registry is a person's ask; an agent reads the copy",
            )
        })?;
        self.catalogue
            .refresh(&self.lookup)
            .await
            .map_err(|error| WireError::new(ErrorCode::Unavailable, error.to_string()))
    }

    /// `harness.list`'s answer: the four found here, then the added agents
    /// whose program is found (never run to ask a version).
    pub(crate) async fn harnesses(&self) -> Vec<Harness> {
        let mut found = vessel_harness::discover(&self.lookup).await;
        let (lookup, profiles) = (self.lookup.clone(), self.profiles.current());
        match tokio::task::spawn_blocking(move || {
            vessel_harness::discover_added(&lookup, profiles.added())
        })
        .await
        {
            Ok(added) => found.extend(added),
            Err(error) => tracing::error!(%error, "added agents lookup task failed"),
        }
        found
    }

    /// The profile `name` of `harness` (its default without one), ready to
    /// launch: an added agent's with its secrets filled in.
    pub(crate) fn resolve(
        &self,
        harness: &HarnessRef,
        name: Option<&ProfileName>,
    ) -> Result<Profile, ServiceError> {
        self.resolve_in(&self.profiles.current(), harness, name)
    }

    /// A catalog resolves and validates against the same retained profile set.
    pub(crate) fn resolve_in(
        &self,
        snapshot: &Profiles,
        harness: &HarnessRef,
        name: Option<&ProfileName>,
    ) -> Result<Profile, ServiceError> {
        let agent = match harness {
            HarnessRef::Known(kind) => return named_in(snapshot, Some(*kind), name),
            HarnessRef::Added(agent) => agent,
        };
        let profile = match name {
            None => Some(
                snapshot
                    .default_of(harness)
                    .ok_or_else(|| ServiceError::UnknownAgent(agent.clone()))?,
            ),
            Some(name) => {
                let profile = snapshot
                    .get(name)
                    .cloned()
                    .ok_or_else(|| ServiceError::UnknownProfile(name.clone()))?;
                if profile.harness() != *harness {
                    return Err(ServiceError::ProfileHarness {
                        profile: name.clone(),
                        profile_harness: profile.harness(),
                        harness: harness.clone(),
                    });
                }
                Some(profile)
            }
        };
        self.launcher
            .added(&self.profiles, harness, profile)?
            .ok_or_else(|| ServiceError::UnknownAgent(agent.clone()))
    }
}
