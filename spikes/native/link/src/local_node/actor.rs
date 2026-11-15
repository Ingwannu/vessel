//! Serialized lifecycle commands keep failed-stop ownership alive for a retry.

use super::{Action, Configuration, Confirmation, Connection, ManagerError, Snapshot};
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

pub(super) enum Command {
    Logs(oneshot::Sender<Result<super::LogText, ManagerError>>),
    Action {
        action: Action,
        reply: oneshot::Sender<Result<Snapshot, ManagerError>>,
    },
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
                let result =
                    perform_action(action, &config, &mut node, &mut log_source, &published).await;
                let _closed = reply.send(result);
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

async fn perform_action(
    action: Action,
    config: &Configuration,
    node: &mut Option<Managed>,
    log_source: &mut Option<vessel_local_node::LogReader>,
    published: &watch::Sender<Snapshot>,
) -> Result<Snapshot, ManagerError> {
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
        Action::Start => start(config, node, log_source).await,
        Action::Stop => control(node, log_source, StopPolicy::Protect, false).await,
        Action::Restart => control(node, log_source, StopPolicy::Protect, true).await,
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
) -> Result<(), Arc<LocalNodeError>> {
    if matches!(
        node.as_ref().map(|managed| managed.node.status()),
        Some(LocalStatus::Owned(_) | LocalStatus::Attached(_))
    ) {
        return Ok(());
    }
    if let Some(launch) = &config.launch {
        *log_source = None;
        *node = Some(Managed::new(
            LocalNode::start_observed(launch.clone(), |logs| {
                *log_source = Some(logs);
            })
            .await
            .map_err(Arc::new)?,
        ));
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
    let current = published.borrow();
    matches!(current.status, Status::Owned(_) | Status::Working)
        && Arc::ptr_eq(&current.permit, &confirmation.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::local_node::Preferences;
    use vessel_proto::NodeId;

    fn snapshot(status: Status) -> Snapshot {
        Snapshot {
            status,
            failure: None,
            connection: None,
            permit: Arc::new(()),
        }
    }

    #[test]
    fn interruption_permission_expires_even_when_a_replacement_keeps_the_node_id() {
        let node = NodeId::random();
        let original = snapshot(Status::Owned(node));
        let permit = original.confirmation().unwrap();
        let (published, _receiver) = watch::channel(original);
        assert!(valid_confirmation(&published, &permit));
        let _old = published.send_replace(snapshot(Status::Owned(node)));
        assert!(!valid_confirmation(&published, &permit));
        assert!(snapshot(Status::Attached(node)).confirmation().is_none());
    }

    #[tokio::test]
    async fn stale_stop_and_restart_confirmation_never_enter_the_control_path() {
        let node_id = NodeId::random();
        let original = snapshot(Status::Owned(node_id));
        let confirmation = original.confirmation().unwrap();
        let (published, _receiver) = watch::channel(snapshot(Status::Owned(node_id)));
        let config = Configuration {
            launch: None,
            home: vessel_local_node::NodeHome::new(
                std::env::temp_dir().join("unused-stale-local-confirmation-fixture"),
            )
            .unwrap(),
            preferences: Preferences::default(),
        };
        let mut node = None;
        let mut logs = None;
        for action in [
            Action::InterruptConfirmed(confirmation.clone()),
            Action::RestartInterruptConfirmed(confirmation.clone()),
        ] {
            assert!(matches!(
                perform_action(action, &config, &mut node, &mut logs, &published).await,
                Err(ManagerError::StaleConfirmation)
            ));
            assert_eq!(published.borrow().status, Status::Owned(node_id));
        }
    }

    #[tokio::test]
    async fn a_missing_companion_never_leaves_an_admitted_start_spinning() {
        let config = Configuration {
            launch: None,
            home: vessel_local_node::NodeHome::new(
                std::env::temp_dir().join("unused-local-node-actor-fixture"),
            )
            .unwrap(),
            preferences: Preferences::default(),
        };
        let (published, current) = watch::channel(snapshot(Status::NotInstalled));
        let (commands, inbox) = mpsc::channel(2);
        let task = tokio::spawn(run(config, inbox, published));
        let (started, start_result) = oneshot::channel();
        commands
            .send(Command::Action {
                action: Action::Start,
                reply: started,
            })
            .await
            .unwrap();
        let acknowledged = start_result.await.unwrap().unwrap();
        assert_eq!(acknowledged.status, Status::NotInstalled);
        // A same-queue finalization acknowledgement is a deterministic barrier,
        // rather than a sleep that might miss the transient Working snapshot.
        let (reply, done) = oneshot::channel();
        commands
            .send(Command::Finalize {
                policy: QuitPolicy::KeepRunning,
                confirmation: None,
                reply,
            })
            .await
            .unwrap();
        done.await.unwrap().unwrap();
        assert_eq!(current.borrow().status, Status::NotInstalled);
        drop(commands);
        task.await.unwrap();
    }
}
