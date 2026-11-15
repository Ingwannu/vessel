//! One bounded actor bridge: durable anchor, opened-root exclusion, then capture.
use super::{backend, runner::Client};
use crate::{sessions::Sessions, workspace::Workspaces};
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::{
    SessionInfo, WorktreeId,
    checkpoint::{
        Boundary, CHECKPOINT_WORKERS, CaptureGeneration, CaptureId, CaptureKey, CheckpointError,
    },
    workspace::SessionPlace,
};
use vessel_session::CheckpointRequest;
use vessel_store::Store;
use vessel_workspace::checkpoint::IgnoreDefaults;

pub(crate) struct Context {
    pub(crate) sessions: Arc<Sessions>,
    pub(crate) workspaces: Arc<Workspaces>,
    pub(crate) store: Store,
    pub(crate) tasks: TaskTracker,
    pub(crate) stop: CancellationToken,
}
/// Resolve Git's default once at the node edge; workers never read ambient HOME.
fn defaults() -> Result<IgnoreDefaults, CheckpointError> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .ok_or(CheckpointError::UnsupportedScope)?;
    IgnoreDefaults::new(config.join("git").join("ignore")).map_err(|error| error.reason())
}
pub(crate) fn serve(context: Context, mut requests: mpsc::Receiver<CheckpointRequest>) {
    let defaults = defaults();
    let tasks = context.tasks.clone();
    let _owner = tasks.spawn(async move {
        let workers = super::workers::Workers::default();
        super::gc::start(context.store.clone(),Arc::clone(&context.workspaces),workers.clone(),&context.tasks,context.stop.clone());
        let defaults = match defaults {
            Ok(defaults) => defaults,
            Err(reason) => {
                loop {
                    tokio::select! {
                        ()=context.stop.cancelled()=>break,
                        request=requests.recv()=>match request {
                            Some(request)=>drop(request.acknowledge(Err(reason))),
                            None=>break,
                        },
                    }
                }
                return;
            }
        };
        let (client, owner) = backend::start_shared(Arc::clone(&context.workspaces), context.store, defaults, workers);
        let mut faults = client.cleanup_faults();
        let mut faults_open = true;
        let mut jobs = JoinSet::new();
        let mut generation = Ok(CaptureGeneration::FIRST);
        let mut receiving = true;
        while receiving || !jobs.is_empty() {
            tokio::select! {
                ()=context.stop.cancelled(), if receiving => { requests.close(); receiving=false; },
                changed=faults.changed(), if faults_open=>match changed {
                    Ok(()) => if let Some(fault) = *faults.borrow_and_update() {
                        tracing::error!(capture=%fault.key.capture,reason=%fault.reason,"checkpoint cleanup needs reconciliation");
                    },
                    Err(_closed) => {faults_open=false;requests.close();receiving=false;},
                },
                completed=jobs.join_next(), if !jobs.is_empty()=> {
                    if let Some(Err(error)) = completed { tracing::error!(%error, "checkpoint bridge task stopped"); }
                },
                request=requests.recv(), if receiving && jobs.len() < CHECKPOINT_WORKERS=> {
                    let Some(request)=request else {receiving=false;continue;};
                    let next = match generation {
                        Ok(current) => { generation=current.next(); current },
                        Err(reason) => { drop(request.acknowledge(Err(reason))); continue; },
                    };
                    let _job = jobs.spawn(handle(request, next, client.clone(), Arc::clone(&context.sessions), Arc::clone(&context.workspaces)));
                },
            }
        }
        // Drain the accepted workers; a blocked syscall keeps node lock ownership.
        drop(requests);
        drop(client);
        if let Err(error) = owner.await { tracing::error!(%error, "checkpoint capture owner stopped"); }
    });
}
fn worktree(info: &SessionInfo) -> Result<WorktreeId, CheckpointError> {
    info.workspace
        .as_ref()
        .and_then(|workspace| workspace.worktree)
        .or_else(|| match info.spec.place.as_deref() {
            Some(SessionPlace::Worktree { worktree }) => Some(*worktree),
            Some(SessionPlace::Project { .. }) | None => None,
        })
        .ok_or(CheckpointError::UnsupportedScope)
}
async fn handle(
    request: CheckpointRequest,
    generation: CaptureGeneration,
    client: Client,
    sessions: Arc<Sessions>,
    workspaces: Arc<Workspaces>,
) {
    if request.abandoned() {
        return;
    }
    let session = request.session();
    let boundary = request.boundary();
    let deadline = request.deadline();
    let anchor = match boundary {
        Boundary::BeforeTurn { prompt } => prompt,
        Boundary::AfterTurn { ended } => ended,
    };
    let prepared = tokio::time::timeout_at(deadline, async {
        let info = sessions
            .checkpoint_anchor(session, anchor, deadline)
            .await?;
        let worktree = worktree(&info)?;
        let guard = workspaces.checkpoint_admission(worktree).await?;
        guard
            .revalidate()
            .await
            .map_err(|_changed| CheckpointError::StaleCapture)?;
        Ok::<_, CheckpointError>((
            CaptureKey {
                session,
                boundary,
                worktree,
                generation,
                capture: CaptureId::random(),
            },
            guard,
        ))
    })
    .await
    .unwrap_or(Err(CheckpointError::CaptureTimedOut));
    let (key, guard) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => {
            drop(request.acknowledge(Err(reason)));
            return;
        }
    };
    match client
        .capture_candidate(guard.physical(), key, deadline)
        .await
    {
        Ok(accepted) => {
            let delivered = request.acknowledge(Ok(accepted.key));
            match delivered.await {
                Ok(outcome) => accepted.complete(outcome),
                Err(_lost) => drop(accepted),
            }
        }
        Err(reason) => drop(request.acknowledge(Err(reason))),
    }
    drop(guard);
}
