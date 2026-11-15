//! One closed adapter preserves legacy callers while C5 moves to admitted calls.
//! The selected variant exposes only G4's typed call, never its original client.
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use vessel_app_view::workspace::problem::Problem;
use vessel_client::{NodeClient, Unary};

const FILE_CALL_DEADLINE: Duration = Duration::from_secs(30);

pub(super) enum Client<'a> {
    Legacy(&'a NodeClient),
    Selected(&'a SelectedConnection),
}
impl Client<'_> {
    pub(super) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, Problem> {
        match self {
            Self::Legacy(client) => super::calls::call(client, request).await,
            Self::Selected(selected) => {
                tokio::time::timeout(FILE_CALL_DEADLINE, selected.call(request))
                    .await
                    .map_err(|_expired| Problem::Gone("file request deadline elapsed".to_owned()))?
                    .map_err(|error| match error {
                        RequestFailure::Refused(why) => Problem::Gone(why.to_string()),
                        RequestFailure::Call(error) => closed_problem(&error),
                    })
            }
        }
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
