//! Correlated tab entity actions. Only safe failure codes leave this RPC boundary.
use vessel_app_view::layout::{HostAlias, RenameProblem, RenameSubmission};
use vessel_proto::{NodeId, SessionTitle};

/// One rename binds the machine and exact form before the task is launched.
#[derive(Debug, Clone)]
pub struct Ask {
    /// Original authenticated owner; an alias cannot substitute another node.
    pub node: NodeId,
    /// Validated machine-book alias.
    pub machine: HostAlias,
    /// Exact typed entity, title and form correlation.
    pub submission: RenameSubmission,
}
/// Entity acknowledgement is independent of the currently selected workspace.
#[derive(Debug)]
pub struct Answer {
    /// Exact form and entity submitted.
    pub ask: Ask,
    /// Authoritative server title, or a structured refusal.
    pub result: Result<SessionTitle, RenameProblem>,
}
impl Ask {
    /// Queue or connection failure takes the same response path as an RPC refusal.
    #[must_use]
    pub fn refused(self, problem: RenameProblem) -> Answer {
        Answer {
            ask: self,
            result: Err(problem),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{Answer, Ask, RenameProblem, RenameSubmission, SessionTitle};
    use std::time::Duration;
    use vessel_app_view::layout::RenameTarget;
    use vessel_client::NodeClient;

    /// A title mutation must not keep a modal waiting indefinitely.
    const RENAME_DEADLINE: Duration = Duration::from_secs(30);
    pub(crate) async fn run(client: &NodeClient, ask: Ask) -> Answer {
        if client.node().id != ask.node {
            return ask.refused(RenameProblem::NodeChanged);
        }
        let result = tokio::time::timeout(RENAME_DEADLINE, rename(client, &ask.submission))
            .await
            .unwrap_or(Err(RenameProblem::TimedOut));
        Answer { ask, result }
    }
    async fn rename(
        client: &NodeClient,
        submission: &RenameSubmission,
    ) -> Result<SessionTitle, RenameProblem> {
        let result = match submission.target {
            RenameTarget::Agent(session) => client
                .call(vessel_proto::RenameSession {
                    session,
                    title: submission.title.clone(),
                })
                .await
                .map(|info| info.title),
            RenameTarget::Terminal(terminal) => client
                .call(vessel_proto::terminals::RenameTerminal {
                    terminal,
                    title: submission.title.clone(),
                })
                .await
                .map(|info| info.title),
        };
        result
            .map_err(|error| {
                error
                    .code()
                    .map_or(RenameProblem::Offline, RenameProblem::Node)
            })?
            .ok_or(RenameProblem::MalformedResponse)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::run;
