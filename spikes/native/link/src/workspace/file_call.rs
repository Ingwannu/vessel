//! File effects expose only G4's admitted typed call, never its original client.
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use vessel_app_view::workspace::problem::Problem;
use vessel_client::Unary;

const FILE_CALL_DEADLINE: Duration = Duration::from_secs(30);

pub(super) struct Client<'a>(pub(super) &'a SelectedConnection);
impl Client<'_> {
    pub(super) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, Problem> {
        let selected = self.0;
        tokio::time::timeout(FILE_CALL_DEADLINE, selected.call(request))
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
        CallError::Disconnected
        | CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted => {
            Problem::Gone("file request did not complete".to_owned())
        }
    }
}
