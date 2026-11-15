//! Protocol-10 command reads/grants use the same selected physical guard as
//! scripts. Keeping the read independent preserves support on older nodes.
#[cfg(not(target_arch = "wasm32"))]
use crate::connections::SelectedConnection;
use vessel_app_view::workspace::command_access::Target;
use vessel_app_view::workspace::git::RequestKey;
use vessel_app_view::workspace::problem::Problem;
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::workspace::{ApproveCommands, GetProjectCommands};
use vessel_proto::workspace::{CommandsDigest, ProjectCommands};

/// One explicit command-set intention, captured before dispatch.
#[derive(Debug)]
pub struct Ask {
    /// Original authenticated admission and project.
    pub target: Target,
    /// App-only correlation.
    pub key: RequestKey,
    /// Existing typed protocol operation.
    pub operation: Operation,
}
/// No command text or unchecked method name crosses this seam.
#[derive(Debug)]
pub enum Operation {
    /// Read the current folder's complete set.
    Read,
    /// Approve/withdraw exactly the digest shown on the producing admission.
    Approve {
        /// Inspected complete set.
        digest: CommandsDigest,
        /// Whether to add or remove its grant.
        approved: bool,
    },
}
/// Echo every original identity on success, refusal and unsent paths.
#[derive(Debug)]
pub struct Answer {
    /// Original request, including digest for mutation receipts.
    pub ask: Ask,
    /// Complete updated set or safe failure.
    pub result: Result<ProjectCommands, Problem>,
}
impl Ask {
    /// Refuse without polling any node client.
    #[must_use]
    pub fn refused(self, problem: &Problem) -> Answer {
        Answer {
            ask: self,
            result: Err(problem.clone()),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn run(client: &SelectedConnection, ask: Ask) -> Answer {
    let result = match &ask.operation {
        Operation::Read => {
            super::scripts::bounded(
                client,
                GetProjectCommands {
                    project: ask.target.project,
                    base: None,
                },
            )
            .await
        }
        Operation::Approve { digest, approved } => {
            super::scripts::bounded(
                client,
                ApproveCommands {
                    project: ask.target.project,
                    digest: digest.clone(),
                    approved: *approved,
                },
            )
            .await
        }
    };
    Answer { ask, result }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(super) fn workspace_commands(&self, machine: String, ask: Ask) {
        let events = self.events.clone();
        let selected = (ask.target.context.machine().as_str() == machine)
            .then(|| self.selected_connection(&ask.target.context));
        let Some(Ok(client)) = selected else {
            super::Hub::refuse(
                machine,
                super::Ask::CommandAccess(ask),
                Problem::Gone(String::new()),
                events,
            );
            return;
        };
        drop(tokio::spawn(async move {
            let answer = run(&client, ask).await;
            super::Reply { machine, events }
                .send(super::Answer::CommandAccess(answer))
                .await;
        }));
    }
}

#[cfg(test)]
mod tests;
