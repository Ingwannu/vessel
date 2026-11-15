//! Typed observation boundary; a runtime producer is required before dispatch.
//! The default is unavailable until the reviewed runtime reaches the app base.
//! It makes no guessed wire call and cannot confer workspace bootstrap permission.
use std::future::Future;
use vessel_app_view::workspace::git::RequestKey;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::setup_observation::{Address, Support};
use vessel_proto::workspace::{
    CancelSetup, CancelSetupOutcome, ReadSetupProgress, SetupProgressReply,
};

/// Local intention and its original admission, echoed even when unavailable.
#[derive(Debug)]
pub struct Ask {
    /// Exact backing and authenticated connection.
    pub address: Address,
    /// View correlation only.
    pub key: RequestKey,
    /// Requested typed observation operation.
    pub operation: Operation,
}
/// Existing canonical request values; no method strings or shell command input.
#[derive(Debug)]
pub enum Operation {
    /// Read a volatile attempt; absence means unknown history.
    Read(ReadSetupProgress),
    /// Request cancellation of only the inspected invocation.
    Cancel(CancelSetup),
}
/// Every response retains the exact original request.
#[derive(Debug)]
pub struct Answer {
    /// Captured request, never reconstructed from a selected workspace.
    pub ask: Ask,
    /// Distinct result variants prevent a cancellation receipt becoming progress.
    pub result: Result<Reply, Problem>,
}
/// Cancellation acknowledgement is never a fabricated stopped snapshot.
#[derive(Debug)]
pub enum Reply {
    /// Current structured attempt, or explicit absence.
    Read(SetupProgressReply),
    /// The node acknowledged or refused that invocation's cancellation.
    Cancel(CancelSetupOutcome),
}

// The reviewed runtime adapter will supply physical admission, authorization,
// deadlines and decoded snapshot limits. A fake is enough to verify that this
// owner does not retarget or dispatch an unsupported operation. No raw client
// access is exposed by the interface.
pub(super) trait Backend: Sync {
    fn support(&self, address: &Address) -> Support;
    fn read(
        &self,
        address: &Address,
        request: ReadSetupProgress,
    ) -> impl Future<Output = Result<SetupProgressReply, Problem>> + Send;
    fn cancel(
        &self,
        address: &Address,
        request: CancelSetup,
    ) -> impl Future<Output = Result<CancelSetupOutcome, Problem>> + Send;
}

pub(super) async fn run(backend: &impl Backend, ask: Ask) -> Answer {
    let result = match &ask.operation {
        Operation::Read(request)
            if request.worktree == ask.address.worktree
                && backend.support(&ask.address) != Support::Unavailable =>
        {
            backend
                .read(&ask.address, request.clone())
                .await
                .map(Reply::Read)
        }
        Operation::Cancel(request)
            if request.worktree == ask.address.worktree
                && backend.support(&ask.address) == Support::ObserveAndCancel =>
        {
            backend
                .cancel(&ask.address, *request)
                .await
                .map(Reply::Cancel)
        }
        Operation::Read(_) | Operation::Cancel(_) => Err(Problem::Unsent),
    };
    Answer { ask, result }
}

struct Unavailable;
impl Backend for Unavailable {
    fn support(&self, _address: &Address) -> Support {
        Support::Unavailable
    }
    async fn read(
        &self,
        _address: &Address,
        _request: ReadSetupProgress,
    ) -> Result<SetupProgressReply, Problem> {
        Err(Problem::Unsent)
    }
    async fn cancel(
        &self,
        _address: &Address,
        _request: CancelSetup,
    ) -> Result<CancelSetupOutcome, Problem> {
        Err(Problem::Unsent)
    }
}
/// Preserve a typed intention without selecting or polling any transport.
/// A real adapter requires the reviewed producer, capability and physical guard.
pub async fn unavailable(ask: Ask) -> Answer {
    run(&Unavailable, ask).await
}

#[cfg(test)]
mod tests;
