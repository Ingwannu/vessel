//! Protocol 11's agents (ADR 0032): agents beyond the four, as the node's
//! service routes them. Routing only, as the rest of the service: the
//! definitions and their file are `crate::added`'s, launching is the
//! launcher's.

use vessel_harness::Profile;
use vessel_proto::agents::{AddAgent, AgentInfo, ListAgents, RemoveAgent};
use vessel_proto::{Ack, Harness, HarnessRef, ProfileName, SessionId, WireError};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::transfers::export::named;

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
        crate::added::add(&self.profiles, &self.probes, request)
            .await
            .map_err(|error| crate::added::wire(&error))
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
        crate::added::remove(&self.profiles, &self.probes, request)
            .await
            .map_err(|error| crate::added::wire(&error))?;
        Ok(Ack)
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
        let agent = match harness {
            HarnessRef::Known(kind) => return named(&self.profiles, Some(*kind), name),
            HarnessRef::Added(agent) => agent,
        };
        let profile = match name {
            None => None,
            Some(name) => {
                let profile = self
                    .profiles
                    .get(name)
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
