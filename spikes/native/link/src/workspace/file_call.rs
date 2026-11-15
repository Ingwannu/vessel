//! File effects expose only G4's admitted typed call, never its original client.
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use vessel_app_view::workspace::problem::Problem;
use vessel_client::Unary;

const FILE_CALL_DEADLINE: Duration = Duration::from_secs(30);
const OBSERVATION_CALL_DEADLINE: Duration = Duration::from_secs(65);

pub(super) struct Client<'a>(pub(super) &'a SelectedConnection);
impl Client<'_> {
    pub(super) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, Problem> {
        self.timed(request, FILE_CALL_DEADLINE).await
    }
    // Server observation has a 30-second heartbeat and a 60-second task
    // admission deadline. Headroom avoids cancelling its ordinary heartbeat.
    pub(super) async fn observe(
        &self,
        request: vessel_proto::workspace::panel_files::ObserveFile,
    ) -> Result<vessel_proto::workspace::panel_files::ObservedFile, Problem> {
        self.timed(request, OBSERVATION_CALL_DEADLINE).await
    }
    async fn timed<R: Unary>(
        &self,
        request: R,
        deadline: Duration,
    ) -> Result<R::Response, Problem> {
        let selected = self.0;
        tokio::time::timeout(deadline, selected.call(request))
            .await
            .map_err(|_expired| Problem::Gone("file request deadline elapsed".to_owned()))?
            .map_err(|error| match error {
                RequestFailure::Refused(why) => Problem::Gone(why.to_string()),
                RequestFailure::Call(error) => closed_problem(&error),
            })
    }
}

// Raw node messages can include paths or credentials. The feature needs the
// closed code for conflict handling; display copy comes from its own words.
fn closed_problem(error: &vessel_client::CallError) -> Problem {
    use vessel_client::CallError;
    match error {
        CallError::Remote(error) => Problem::Node {
            code: error.code,
            message: String::new(),
        },
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => Problem::Older,
        CallError::RequestIdsExhausted => Problem::Unsent,
        CallError::Disconnected | CallError::UnexpectedReply(_) => {
            Problem::Gone("file request did not complete".to_owned())
        }
    }
}
