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
async fn a_document_checked_control_cannot_retarget_a_replacement_or_another_node() {
    let node_id = NodeId::random();
    let original = snapshot(Status::Owned(node_id));
    let lease = original.control().unwrap();
    let (published, _receiver) = watch::channel(original.clone());
    let mut forged_display = original;
    forged_display.status = Status::Owned(NodeId::random());
    assert!(!valid_control(
        &published,
        &forged_display.control().unwrap()
    ));
    let _old = published.send_replace(snapshot(Status::Owned(node_id)));
    let config = Configuration {
        launch: None,
        home: vessel_local_node::NodeHome::new(
            std::env::temp_dir().join("unused-stale-control-fixture"),
        )
        .unwrap(),
        preferences: Preferences::default(),
    };
    let mut node = None;
    let mut logs = None;
    for action in [Action::Stop(lease.clone()), Action::Restart(lease.clone())] {
        assert!(matches!(
            perform_action(
                action,
                &config,
                &mut node,
                &mut logs,
                &published,
                &mut History::default()
            )
            .await,
            Err(ManagerError::StaleControl)
        ));
        assert!(node.is_none());
        assert_eq!(published.borrow().status, Status::Owned(node_id));
    }
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
            perform_action(
                action,
                &config,
                &mut node,
                &mut logs,
                &published,
                &mut History::default()
            )
            .await,
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
