//! Existing node calls, projected to the safe owner result at this boundary.
use super::{Action, Output, Problem, runner::Node};
use vessel_client::{CallError, NodeClient};
use vessel_proto::mcp::{Injection, MCP_PARITY, McpSettingsPatch};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::skills::{GetSkillsStatus, ReconcileSkills, UninstallSkills};
use vessel_proto::{ErrorCode, NodeId};

impl Node for NodeClient {
    fn identity(&self) -> NodeId {
        self.node().id
    }
    fn supported(&self) -> bool {
        MCP_PARITY.is_some_and(|required| self.protocol() >= required)
    }
    async fn execute(&self, action: Action) -> Result<Output, Problem> {
        match action {
            Action::ReadTools => {
                let current = self.call(GetNodeSettings {}).await.map_err(problem)?;
                current.mcp.map(Output::Tools).ok_or(Problem::Unsupported)
            }
            Action::Inject(choice) => {
                let current = self.call(injection(choice)).await.map_err(problem)?;
                current.mcp.map(Output::Tools).ok_or(Problem::Unsupported)
            }
            Action::ReadSkills => self
                .call(GetSkillsStatus {})
                .await
                .map(Output::Skills)
                .map_err(problem),
            Action::Reconcile => self
                .call(ReconcileSkills {})
                .await
                .map(Output::Skills)
                .map_err(problem),
            Action::Uninstall => self
                .call(UninstallSkills {})
                .await
                .map(Output::Skills)
                .map_err(problem),
            Action::Save(request) => self.call(request).await.map(Output::Saved).map_err(problem),
        }
    }
}
pub(super) fn injection(choice: Injection) -> SetNodeSettings {
    SetNodeSettings {
        mcp: Some(McpSettingsPatch {
            inject: Some(choice),
            policy: None,
        }),
        ..SetNodeSettings::default()
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
