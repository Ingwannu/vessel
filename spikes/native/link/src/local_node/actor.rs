//! Serialized lifecycle commands keep failed-stop ownership alive for a retry.

mod history;

use history::History;

use super::{
    Action, Configuration, Confirmation, Connection, ControlLease, ManagerError, Snapshot,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::local_node::{LocalProvenance, Management, Status};
use vessel_local_node::{LocalNode, LocalNodeError, LocalStatus, QuitPolicy, StopPolicy};

struct Managed {
    node: LocalNode,
    source: Option<super::transport::Source>,
}

impl Managed {
    fn new(node: LocalNode) -> Self {
        let source = node.client().cloned().map(super::transport::Source::new);
        Self { node, source }
    }
}

impl Drop for Managed {
    fn drop(&mut self) {
        // Revoke admission before the owned node's RAII cleanup can affect its
        // process. Field destruction alone would drop `node` before `source`.
        if let Some(source) = &self.source {
            source.retire();
        }
    }
}

pub(super) enum Command {
    Logs(oneshot::Sender<Result<super::LogText, ManagerError>>),
    Action {
        action: Action,
        reply: oneshot::Sender<Result<Snapshot, ManagerError>>,
    },
    PreserveExit(oneshot::Sender<Result<(), ManagerError>>),
    Finalize {
        policy: QuitPolicy,
        confirmation: Option<Confirmation>,
        reply: oneshot::Sender<Result<(), ManagerError>>,
    },
}

pub(super) async fn run(
    config: Configuration,
    mut commands: mpsc::Receiver<Command>,
    published: watch::Sender<Snapshot>,
) {
    let mut node = None;
    let mut log_source = None;
    let mut history = History::default();
    if config.preferences.management == Management::Enabled && config.launch.is_some() {
        let result = start(&config, &mut node, &mut log_source, &mut history).await;
        publish(&published, node.as_ref(), result.err());
    }
    let mut observe = true;
    loop {
        history.prune();
        let command = if let Some(current) = &mut node {
            if observe
                && matches!(
                    current.node.status(),
                    LocalStatus::Owned(_) | LocalStatus::Attached(_) | LocalStatus::Kept(_)
                )
            {
                tokio::select! {
                    command = commands.recv() => command,
                    exited = current.node.wait_exit() => {
                        observe = exited.is_ok();
                        publish(&published, node.as_ref(), exited.err().map(Arc::new));
                        continue;
                    }
                }
            } else {
                commands.recv().await
            }
        } else {
            commands.recv().await
        };
        let Some(command) = command else {
            break;
        };
        match command {
            Command::Logs(reply) => {
                let result = match log_source.clone().or_else(|| {
                    node.as_ref()
                        .filter(|node| {
                            matches!(
                                node.node.status(),
                                LocalStatus::Owned(_) | LocalStatus::Kept(_)
                            )
                        })
                        .and_then(|managed| managed.node.logs())
                }) {
                    Some(logs) => logs.read().await.map_err(ManagerError::Logs),
                    None => config.home.log_tail().await.map_err(ManagerError::Files),
                };
                let _closed = reply.send(result);
            }
            Command::Action { action, reply } => {
                observe = true;
                let result = perform_action(
                    action,
                    &config,
                    &mut node,
                    &mut log_source,
                    &published,
                    &mut history,
                )
                .await;
                let _closed = reply.send(result);
            }
            Command::PreserveExit(reply) => {
                // No later UI request can create a new owner after this handoff.
                commands.close();
                preserve_exit(&mut node, &published, &mut history).await;
                let _receiver_gone = reply.send(Ok(()));
                break;
            }
            Command::Finalize {
                policy,
                confirmation,
                reply,
            } => {
                let result = finalize(&mut node, policy, confirmation, &published).await;
                let _receiver_gone = reply.send(result);
            }
        }
    }
    // The last UI sender should finalize first. If all senders disappear during
    // startup or a crash-like UI teardown, ordinary Owned RAII still cleans up.
}

async fn preserve_exit(
    node: &mut Option<Managed>,
    published: &watch::Sender<Snapshot>,
    history: &mut History,
) {
    if finalize(node, QuitPolicy::KeepRunning, None, published)
        .await
        .is_err()
    {
        // A failed surrender must not become implicit owned Drop cleanup merely
        // because the platform's short shutdown callback has expired.
        std::future::pending::<()>().await;
    }
    if let Some(current) = node
        && matches!(current.node.status(), LocalStatus::Kept(_))
    {
        if current.node.wait_exit().await.is_err() {
            std::future::pending::<()>().await;
        }
        publish(published, node.as_ref(), None);
    }
    history.wait().await;
}

async fn perform_action(
    action: Action,
    config: &Configuration,
    node: &mut Option<Managed>,
    log_source: &mut Option<vessel_local_node::LogReader>,
    published: &watch::Sender<Snapshot>,
    history: &mut History,
) -> Result<Snapshot, ManagerError> {
    if matches!(action, Action::Start) && !history.has_capacity() {
        return Err(ManagerError::ObservationLimit);
    }
    if let Action::Stop(lease) | Action::Restart(lease) = &action
        && !valid_control(published, lease)
    {
        return Err(ManagerError::StaleControl);
    }
    if let Action::InterruptConfirmed(confirmation)
    | Action::RestartInterruptConfirmed(confirmation) = &action
        && !valid_confirmation(published, confirmation)
    {
        return Err(ManagerError::StaleConfirmation);
    }
    let prior = published.borrow().clone();
    let prior_status = prior.status;
    let _previous = published.send_replace(Snapshot {
        status: Status::Working,
        ..prior
    });
    let result = match action {
        Action::Start => start(config, node, log_source, history).await,
        Action::Stop(_lease) => control(node, log_source, StopPolicy::Protect, false).await,
        Action::Restart(_lease) => control(node, log_source, StopPolicy::Protect, true).await,
        Action::InterruptConfirmed(_confirmation) => {
            control(node, log_source, StopPolicy::InterruptConfirmed, false).await
        }
        Action::RestartInterruptConfirmed(_confirmation) => {
            control(node, log_source, StopPolicy::InterruptConfirmed, true).await
        }
    };
    if node.is_none() && result.is_ok() {
        published.send_modify(|snapshot| snapshot.status = prior_status);
    }
    publish(published, node.as_ref(), result.err());
    Ok(published.borrow().clone())
}

async fn finalize(
    node: &mut Option<Managed>,
    policy: QuitPolicy,
    confirmation: Option<Confirmation>,
    published: &watch::Sender<Snapshot>,
) -> Result<(), ManagerError> {
    if confirmation
        .as_ref()
        .is_some_and(|confirmation| !valid_confirmation(published, confirmation))
    {
        return Err(ManagerError::StaleConfirmation);
    }
    let result = match node {
        Some(current) => current
            .node
            .quit(policy)
            .await
            .map(|_status| ())
            .map_err(Arc::new),
        None => Ok(()),
    };
    publish(
        published,
        node.as_ref(),
        result.as_ref().err().map(Arc::clone),
    );
    result.map_err(|cause| ManagerError::Control(cause, Box::new(published.borrow().clone())))
}

async fn start(
    config: &Configuration,
    node: &mut Option<Managed>,
    log_source: &mut Option<vessel_local_node::LogReader>,
    history: &mut History,
) -> Result<(), Arc<LocalNodeError>> {
    if matches!(
        node.as_ref().map(|managed| managed.node.status()),
        Some(LocalStatus::Owned(_) | LocalStatus::Attached(_))
    ) {
        return Ok(());
    }
    if let Some(launch) = &config.launch {
        *log_source = None;
        let replacement = Managed::new(
            LocalNode::start_observed(launch.clone(), |logs| {
                *log_source = Some(logs);
            })
            .await
            .map_err(Arc::new)?,
        );
        if let Some(mut previous) = node.replace(replacement)
            && matches!(previous.node.status(), LocalStatus::Kept(_))
        {
            // Reconnect never converts released authority back into ownership.
            // Preserve the original direct-child observation independently of
            // the new Attached transport and withdraw its old source first.
            drop(previous.source.take());
            history.retain(async move {
                if previous.node.wait_exit().await.is_err() {
                    tracing::warn!("released child exit remains unconfirmed");
                    std::future::pending::<()>().await;
                }
            });
        }
    }
    Ok(())
}

async fn control(
    node: &mut Option<Managed>,
    log_source: &mut Option<vessel_local_node::LogReader>,
    policy: StopPolicy,
    restart: bool,
) -> Result<(), Arc<LocalNodeError>> {
    let Some(node) = node else {
        return Ok(());
    };
    if restart {
        *log_source = None;
        let _status = node
            .node
            .restart_observed(policy, |logs| *log_source = Some(logs))
            .await
            .map_err(Arc::new)?;
        node.source = node
            .node
            .client()
            .cloned()
            .map(super::transport::Source::new);
    } else {
        let _exit = node.node.stop(policy).await.map_err(Arc::new)?;
    }
    Ok(())
}

fn publish(
    published: &watch::Sender<Snapshot>,
    node: Option<&Managed>,
    failure: Option<Arc<LocalNodeError>>,
) {
    let (status, provenance) = match node.map(|managed| managed.node.status()) {
        Some(LocalStatus::Owned(id)) => (Status::Owned(id), Some(LocalProvenance::OwnedChild)),
        Some(LocalStatus::Attached(id)) => (
            Status::Attached(id),
            Some(LocalProvenance::AttachedManagedHome),
        ),
        Some(LocalStatus::Kept(id)) => (Status::Kept(id), None),
        Some(LocalStatus::Exited(_)) => (Status::Stopped, None),
        Some(LocalStatus::AttachedDisconnected(_)) => (Status::Disconnected, None),
        None if failure.is_some() => (Status::Failed, None),
        None => (published.borrow().status, None),
    };
    if provenance.is_none()
        && let Some(source) = node.and_then(|managed| managed.source.as_ref())
    {
        source.retire();
    }
    let connection = provenance.and_then(|provenance| {
        let managed = node?;
        let source = managed.source.as_ref()?.lease();
        if !source.active() {
            return None;
        }
        managed.node.client().map(|client| Connection {
            client: client.clone(),
            provenance,
            source,
        })
    });
    let _previous = published.send_replace(Snapshot {
        status,
        failure,
        connection,
        permit: Arc::new(()),
    });
}

fn valid_confirmation(published: &watch::Sender<Snapshot>, confirmation: &Confirmation) -> bool {
    valid_control(published, &confirmation.0)
}

fn valid_control(published: &watch::Sender<Snapshot>, lease: &ControlLease) -> bool {
    let current = published.borrow();
    matches!(current.status, Status::Owned(node) if node == lease.node)
        && Arc::ptr_eq(&current.permit, &lease.permit)
}

#[cfg(test)]
mod tests;
