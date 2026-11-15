//! ADR 0077 bounded capture orchestration; restore calls remain pending.
//!
//! Candidate workers cannot publish. The coordinator owns deadline fencing and
//! keeps root/worker/quota admission until cleanup, even after the actor resumes.

#[cfg(target_os = "linux")]
pub(crate) mod backend;
#[cfg(target_os = "linux")]
pub(crate) mod bridge;
#[cfg(target_os = "linux")]
mod controller;
#[cfg(target_os = "linux")]
mod execution;
#[cfg(target_os = "linux")]
mod preview;
#[cfg(target_os = "linux")]
mod restoration;
#[cfg(target_os = "linux")]
pub(crate) use controller::Controller;
mod capture;
#[cfg(target_os = "linux")]
pub(crate) mod gc;
mod quota;
mod runner;

mod workers;

#[cfg(target_os = "linux")]
pub(crate) fn start(
    sessions: std::sync::Arc<crate::sessions::Sessions>,
    workspaces: std::sync::Arc<crate::workspace::Workspaces>,
    store: vessel_store::Store,
    tasks: tokio_util::task::TaskTracker,
    stop: tokio_util::sync::CancellationToken,
    requests: tokio::sync::mpsc::Receiver<vessel_session::CheckpointRequest>,
    defaults: Result<
        vessel_workspace::checkpoint::IgnoreDefaults,
        vessel_proto::checkpoint::CheckpointError,
    >,
) -> Controller {
    let context = bridge::Context::new(sessions, workspaces, store, tasks, stop, defaults);
    let controller = Controller::new(context.clone());
    bridge::serve(context, requests);
    controller
}

#[cfg(test)]
mod runner_tests;

#[cfg(test)]
mod tests;
