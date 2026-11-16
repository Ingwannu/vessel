//! The walk reuses G4's guard before every RPC and after every await.
use crate::connections::{RequestFailure, SelectedConnection};
use vessel_app_view::workspace::problem::Problem;
use vessel_client::Unary;

pub(super) async fn call<R: Unary>(
    client: &SelectedConnection,
    request: R,
) -> Result<R::Response, Problem> {
    client.call(request).await.map_err(|error| match error {
        RequestFailure::Call(error) => super::super::calls::problem(&error),
        RequestFailure::Refused(reason) => Problem::Gone(reason.to_string()),
    })
}
