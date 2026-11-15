//! Correlated project mutations use only the existing typed project RPCs.
//! A timeout is an uncertain remote result; no mutation is automatically replayed.

use super::{Answer, Hub, Reply, calls, follow};
use std::time::Duration;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::actions::{
    ProjectAction, ProjectActionKey, ProjectChanged,
};
use vessel_client::NodeClient;
use vessel_proto::workspace::RemoveProject;

/// Paseo SDK `DEFAULT_SESSION_RPC_TIMEOUT_MS` (daemon-client.ts:1125), in seconds.
const PROJECT_MUTATION_DEADLINE: Duration = Duration::from_secs(60);

/// Meaning travels with the form ticket; machine selection is never read again at completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChange {
    /// Host and unique form operation.
    pub key: ProjectActionKey,
    /// Existing node request whose validating wire types have already parsed input.
    pub action: ProjectAction,
}

/// A finite wait does not imply that the node rolled back its side effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectFailure {
    /// Captured identity no longer matches the authenticated target; no RPC was sent.
    HostChanged,
    /// The original admission retired after dispatch; reconcile before retrying.
    OutcomeUnknown,
    /// Typed transport, scope or node refusal.
    Call(Problem),
    /// Refresh authoritative projects before a person decides whether to retry.
    TimedOut,
}

pub(super) async fn run(
    hub: &Hub,
    client: &NodeClient,
    reply: &Reply,
    change: ProjectChange,
) -> Answer {
    if client.node().id != change.key.node() {
        return Answer::ProjectChanged {
            key: change.key,
            result: Err(ProjectFailure::HostChanged),
        };
    }
    let result = tokio::time::timeout(PROJECT_MUTATION_DEADLINE, send(client, change.action)).await;
    let result = match result {
        Ok(result) => result.map_err(ProjectFailure::Call),
        Err(_deadline) => Err(ProjectFailure::TimedOut),
    };
    if let Ok(ProjectChanged::Registered(project)) = &result
        && vessel_app_view::workspace::projects::is_cloning(project)
    {
        follow::clone_of(hub, client, reply, project.clone());
    }
    Answer::ProjectChanged {
        key: change.key,
        result,
    }
}

async fn send(client: &NodeClient, action: ProjectAction) -> Result<ProjectChanged, Problem> {
    match action {
        ProjectAction::Register(request) => calls::call(client, request)
            .await
            .map(ProjectChanged::Registered),
        ProjectAction::Clone(request) => calls::call(client, request)
            .await
            .map(ProjectChanged::Registered),
        ProjectAction::Initialize(request) => calls::call(client, request)
            .await
            .map(ProjectChanged::Registered),
        ProjectAction::Remove(project) => calls::call(client, RemoveProject { project })
            .await
            .map(|_ack| ProjectChanged::Removed(project)),
    }
}

#[cfg(test)]
mod tests;
