//! Serialized lifecycle commands keep failed-stop ownership alive for a retry.

use super::{Action, Configuration, Confirmation, Connection, ManagerError, Snapshot};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::local_node::{LocalProvenance, Management, Status};
use vessel_local_node::{LocalNode, LocalNodeError, LocalStatus, QuitPolicy, StopPolicy};

pub(super) enum Command {
    Logs(oneshot::Sender<Result<super::LogText, ManagerError>>),
    Action(Action),
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
    if config.preferences.management == Management::Enabled && config.launch.is_some() {
        let result = start(&config, &mut node, &mut log_source).await;
        publish(&published, node.as_ref(), result.err());
    }
    let mut observe = true;
    loop {
        let command = if let Some(current) = &mut node {
            if observe
                && matches!(
                    current.status(),
                    LocalStatus::Owned(_) | LocalStatus::Attached(_) | LocalStatus::Kept(_)
                )
            {
                tokio::select! {
                    command = commands.recv() => command,
                    exited = current.wait_exit() => {
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
                let result = match node
                    .as_ref()
                    .and_then(LocalNode::logs)
                    .or_else(|| log_source.clone())
                {
                    Some(logs) => logs.read().await.map_err(ManagerError::Logs),
                    None => config.home.log_tail().await.map_err(ManagerError::Files),
                };
                let _closed = reply.send(result);
            }
            Command::Action(action) => {
                if let Action::InterruptConfirmed(confirmation) = &action
                    && !valid_confirmation(&published, confirmation)
                {
                    continue;
                }
                let prior = published.borrow().clone();
                let prior_status = prior.status;
                let _previous = published.send_replace(Snapshot {
                    status: Status::Working,
                    ..prior
                });
                let result = match action {
                    Action::Start => start(&config, &mut node, &mut log_source).await,
                    Action::Stop => control(&mut node, StopPolicy::Protect, false).await,
                    Action::Restart => control(&mut node, StopPolicy::Protect, true).await,
                    Action::InterruptConfirmed(_confirmation) => {
                        control(&mut node, StopPolicy::InterruptConfirmed, false).await
                    }
                };
                observe = true;
                if node.is_none() && result.is_ok() {
                    published.send_modify(|snapshot| snapshot.status = prior_status);
                }
                publish(&published, node.as_ref(), result.err());
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

async fn finalize(
    node: &mut Option<LocalNode>,
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
    result.map_err(ManagerError::Control)
}

async fn start(
    config: &Configuration,
    node: &mut Option<LocalNode>,
    log_source: &mut Option<vessel_local_node::LogReader>,
) -> Result<(), Arc<LocalNodeError>> {
    if matches!(
        node.as_ref().map(LocalNode::status),
        Some(LocalStatus::Owned(_) | LocalStatus::Attached(_) | LocalStatus::Kept(_))
    ) {
        return Ok(());
    }
    if let Some(launch) = &config.launch {
        *log_source = None;
        *node = Some(
            LocalNode::start_observed(launch.clone(), |logs| {
                *log_source = Some(logs);
            })
            .await
            .map_err(Arc::new)?,
        );
    }
    Ok(())
}

async fn control(
    node: &mut Option<LocalNode>,
    policy: StopPolicy,
    restart: bool,
) -> Result<(), Arc<LocalNodeError>> {
    let Some(node) = node else {
        return Ok(());
    };
    if restart {
        let _status = node.restart(policy).await.map_err(Arc::new)?;
    } else {
        let _exit = node.stop(policy).await.map_err(Arc::new)?;
    }
    Ok(())
}

fn publish(
    published: &watch::Sender<Snapshot>,
    node: Option<&LocalNode>,
    failure: Option<Arc<LocalNodeError>>,
) {
    let (status, provenance) = match node.map(LocalNode::status) {
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
    let connection = provenance.and_then(|provenance| {
        node.and_then(LocalNode::client).map(|client| Connection {
            client: client.clone(),
            provenance,
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
    let current = published.borrow();
    matches!(current.status, Status::Owned(_) | Status::Working)
        && Arc::ptr_eq(&current.permit, &confirmation.0)
}
