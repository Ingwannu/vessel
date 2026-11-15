//! Existing node calls, projected to the safe owner result at this boundary.
use super::{Action, Output, Problem, runner::Node};
use crate::connections::{RequestFailure, SelectedConnection};
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_client::CallError;
use vessel_proto::mcp::MCP_PARITY;
use vessel_proto::skills::{GetSkillsStatus, ReconcileSkills, UninstallSkills};
use vessel_proto::{ErrorCode, NodeId};

pub(super) struct Selected {
    pub(super) connection: SelectedConnection,
    pub(super) context: ConnectionContext,
}
impl Node for Selected {
    fn identity(&self) -> NodeId {
        self.context.node()
    }
    fn supported(&self) -> bool {
        // The selected connection checks retirement before polling, and its
        // typed client checks each method's actual negotiated protocol.
        MCP_PARITY.is_some()
    }
    async fn execute(&self, action: Action) -> Result<Output, Problem> {
        match action {
            // The approved values carrier contains no NodeSettings.mcp field.
            // Do not substitute a legacy settings read for feature support.
            Action::ReadTools | Action::Inject(_) => Err(Problem::Unsupported),
            Action::ReadSkills => self
                .connection
                .call(GetSkillsStatus {})
                .await
                .map(Output::Skills)
                .map_err(selected_problem),
            Action::Reconcile => self
                .connection
                .call(ReconcileSkills {})
                .await
                .map(Output::Skills)
                .map_err(selected_problem),
            Action::Uninstall => self
                .connection
                .call(UninstallSkills {})
                .await
                .map(Output::Skills)
                .map_err(selected_problem),
            Action::Save(request) => self
                .connection
                .call(request)
                .await
                .map(Output::Saved)
                .map_err(selected_problem),
        }
    }
}
pub(super) fn problem(error: CallError) -> Problem {
    match error {
        CallError::Disconnected => Problem::Offline,
        CallError::Remote(error) => Problem::Refused(error.code),
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
            Problem::Unsupported
        }
        CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => {
            Problem::Refused(ErrorCode::Internal)
        }
    }
}

fn selected_problem(error: RequestFailure) -> Problem {
    match error {
        RequestFailure::Refused(Refusal::Changed) => Problem::Changed,
        RequestFailure::Refused(Refusal::Closed) => Problem::Offline,
        RequestFailure::Call(error) => problem(error),
    }
}
