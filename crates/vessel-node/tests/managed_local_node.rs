//! Actual owned-child readiness/reuse/finalization with isolated homes and no real agents.

#![cfg(test)]

use std::path::Path;
use std::time::Duration;
use vessel_local_node::{
    Launch, LocalNode, LocalNodeError as Error, LocalStatus, NodeHome, QuitPolicy, StopPolicy,
};
use vessel_proto::{ClientInfo, NodeState, StopWhen};
use vessel_testkit::Fake;

const SAFETY_NET: Duration = Duration::from_secs(60);

fn launch(root: &Path, binary: &Path) -> Launch {
    let home = root.join("user-home");
    let programs = root.join("programs");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&programs).unwrap();
    Launch::new(
        binary.to_owned(),
        NodeHome::new(root.join("node")).unwrap(),
        ClientInfo {
            name: "managed-test".to_owned(),
            version: "test".to_owned(),
        },
    )
    .unwrap()
    .environment(|mut command| {
        command.environment = vessel_proc::Environment::Clear;
        command.env.extend([
            (
                "HOME".into(),
                vessel_proc::EnvValue::Set(home.clone().into_os_string()),
            ),
            (
                "USERPROFILE".into(),
                vessel_proc::EnvValue::Set(home.into_os_string()),
            ),
            (
                "PATH".into(),
                vessel_proc::EnvValue::Set(programs.into_os_string()),
            ),
        ]);
        #[cfg(windows)]
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            // Keep only the OS loader root; agent homes and PATH stay private.
            command
                .env
                .push(("SystemRoot".into(), vessel_proc::EnvValue::Set(system_root)));
        }
        command
    })
    .unwrap()
}

#[tokio::test]
async fn authenticated_existing_node_stays_attached_and_cannot_be_stopped_by_reuse() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let recipe = launch(root.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")));
        let mut owned = LocalNode::start(recipe.clone()).await.unwrap();
        let LocalStatus::Owned(node) = owned.status() else {
            panic!("must own the launched child");
        };
        let mut attached = LocalNode::start(recipe).await.unwrap();
        assert_eq!(attached.status(), LocalStatus::Attached(node));
        assert!(matches!(
            attached.stop(StopPolicy::InterruptConfirmed).await,
            Err(Error::NotOwned)
        ));
        assert_eq!(
            attached
                .quit(QuitPolicy::Stop(StopPolicy::InterruptConfirmed))
                .await
                .unwrap(),
            LocalStatus::Attached(node)
        );
        assert_eq!(
            owned.client().unwrap().node_info().await.unwrap().state,
            NodeState::Serving
        );
        let exit = owned.stop(StopPolicy::Protect).await.unwrap();
        assert_eq!(exit.code(), Some(0));
        assert_eq!(owned.status(), LocalStatus::Exited(exit));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(root.path().join("node"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(root.path().join("node/token"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn owned_restart_waits_for_old_exit_and_keep_on_quit_releases_control() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let mut local = LocalNode::start(launch(root.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")))).await.unwrap();
        let old = local.client().unwrap().clone();
        let node = old.node().id;
        let mut replacement_logs = None;
        assert_eq!(local.restart_observed(StopPolicy::Protect, |logs| replacement_logs = Some(logs)).await.unwrap(), LocalStatus::Owned(node));
        assert!(replacement_logs.is_some(), "replacement startup diagnostics were not retained");
        old.closed().await;
        let cleanup = local.client().unwrap().clone();
        assert_eq!(local.quit(QuitPolicy::KeepRunning).await.unwrap(), LocalStatus::Kept(node));
        assert!(root.path().join("node/logs/vesseld.log").is_file());
        assert_eq!(cleanup.node_info().await.unwrap().state, NodeState::Serving);
        assert!(matches!(local.stop(StopPolicy::InterruptConfirmed).await, Err(Error::NotOwned)));
        // A failed app save after Keep may leave its UI open. Explicit reuse
        // authenticates the existing process without recovering child authority.
        let mut reattached = LocalNode::start(launch(root.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")))).await.unwrap();
        assert_eq!(reattached.status(), LocalStatus::Attached(node));
        assert!(matches!(reattached.stop(StopPolicy::Protect).await, Err(Error::NotOwned)));
        assert!(matches!(reattached.restart(StopPolicy::Protect).await, Err(Error::NotOwned)));
        assert_eq!(reattached.quit(QuitPolicy::Stop(StopPolicy::Protect)).await.unwrap(), LocalStatus::Attached(node));
        assert_eq!(cleanup.node_info().await.unwrap().state, NodeState::Serving);
        cleanup.stop_node(StopWhen::WhenIdle).await.unwrap();
        assert!(matches!(local.wait_exit().await.unwrap(), LocalStatus::Exited(exit) if exit.code() == Some(0)));
        let retained = NodeHome::new(root.path().join("node")).unwrap().log_tail().await.unwrap();
        assert!(retained.as_str().contains("stopping"));
    }).await.unwrap();
}

#[tokio::test]
async fn active_fake_session_is_protected_and_a_drain_deadline_retains_ownership() {
    vessel_testkit::within(SAFETY_NET, Box::pin(async {
        use vessel_proto::agents::{AddAgent, AgentDefinition, AgentToAdd};
        use vessel_proto::{HarnessRef, SessionEvent, SessionMode, SessionSpec, SessionStatus};
        let root = tempfile::tempdir().unwrap();
        let mut local = LocalNode::start(launch(root.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")))).await.unwrap();
        let client = local.client().unwrap().clone();
        let node = client.node().id;
        let name = "lifecycle-fixture".parse().unwrap();
        let _added = client.call(AddAgent {
            name,
            agent: AgentToAdd::Command { definition: AgentDefinition {
                label: None,
                program: Fake::Acp.path().unwrap().to_str().unwrap().parse().unwrap(),
                args: Vec::new(), env: Vec::new(), inherit: Vec::new(), mcp: Vec::new(),
            }},
            replace: false,
        }).await.unwrap();
        let spec = SessionSpec::new(HarnessRef::Added("lifecycle-fixture".parse().unwrap()), SessionMode::Structured {
            model: None, resume: None, options: Vec::new(),
        }).with_cwd(root.path().to_owned());
        let session = client.create_session(spec.clone()).await.unwrap();
        let mut events = client.follow(session.id, vessel_proto::EventSeq::ZERO).await.unwrap();
        while let Some(event) = events.next().await {
            if matches!(event.unwrap().event, SessionEvent::Status {status: SessionStatus::Running}) { break; }
        }
        assert!(matches!(local.stop(StopPolicy::Protect).await, Err(Error::ActiveSessions(ids)) if ids.contains(&session.id)));
        assert!(matches!(local.restart(StopPolicy::Protect).await, Err(Error::ActiveSessions(ids)) if ids.contains(&session.id)));
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Serving);
        assert!(matches!(local.stop(StopPolicy::Drain).await, Err(Error::Deadline {operation: vessel_local_node::Operation::Stop, ..})));
        assert_eq!(local.status(), LocalStatus::Owned(node));
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Draining);
        assert!(client.session(session.id).await.unwrap().status.is_live());
        assert!(matches!(client.create_session(spec).await, Err(vessel_client::CallError::Remote(error)) if error.code == vessel_proto::ErrorCode::Unavailable));
        let exit = local.stop(StopPolicy::InterruptConfirmed).await.unwrap();
        assert_eq!(exit.code(), Some(0));
        assert_eq!(local.status(), LocalStatus::Exited(exit));
    })).await.unwrap();
}

#[cfg(unix)]
async fn stub_start(
    root: &Path,
    ready: Option<&[u8]>,
) -> (
    tokio::task::JoinHandle<Result<LocalNode, Error>>,
    tokio::io::BufReader<tokio::net::UnixStream>,
    tokio::sync::oneshot::Receiver<vessel_local_node::LogReader>,
) {
    use tokio::io::{AsyncBufReadExt as _, BufReader};
    let control = root.join("control.sock");
    let listener = tokio::net::UnixListener::bind(&control).unwrap();
    let ready_file = ready.map(|bytes| {
        let path = root.join("ready.txt");
        std::fs::write(&path, bytes).unwrap();
        path
    });
    let recipe = launch(root, &Fake::LocalNode.path().unwrap())
        .environment(|mut command| {
            command.env.push((
                "FAKE_LOCAL_CONTROL_SOCKET".into(),
                vessel_proc::EnvValue::Set(control.into_os_string()),
            ));
            if let Some(path) = ready_file {
                command.env.push((
                    "FAKE_LOCAL_READY_FILE".into(),
                    vessel_proc::EnvValue::Set(path.into_os_string()),
                ));
            }
            command
        })
        .unwrap();
    let (logs, retained) = tokio::sync::oneshot::channel();
    let starting = tokio::spawn(async move {
        LocalNode::start_observed(recipe, |reader| {
            let _closed = logs.send(reader);
        })
        .await
    });
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = BufReader::new(stream);
    let mut announced = String::new();
    let _read = stream.read_line(&mut announced).await.unwrap();
    assert_eq!(announced, "started\n");
    (starting, stream, retained)
}

#[cfg(unix)]
async fn refused_start(root: &Path, ready: Option<&[u8]>) -> Error {
    use tokio::io::AsyncReadExt as _;
    let (starting, mut stream, retained) = stub_start(root, ready).await;
    let logs = retained.await.unwrap();
    let error = starting.await.unwrap().unwrap_err();
    // The fake holds this socket until process exit. EOF proves startup failure
    // retained the original owned handle through its group/job cleanup.
    let mut rest = Vec::new();
    let _eof = stream.read_to_end(&mut rest).await.unwrap();
    // Diagnostics survive the owned startup failure without retaining a child
    // handle or granting another stop path to the viewer.
    assert!(logs.read().await.is_ok());
    error
}

#[cfg(unix)]
#[tokio::test]
async fn malformed_oversized_and_missing_readiness_clean_up_the_owned_child() {
    vessel_testkit::within(SAFETY_NET, async {
        let malformed = tempfile::tempdir().unwrap();
        assert!(matches!(
            refused_start(malformed.path(), Some(b"human log line\n")).await,
            Error::Launch(_)
        ));
        let oversized = tempfile::tempdir().unwrap();
        let bytes = vec![b'x'; vessel_proto::local_launch::MAX_READY_BYTES.saturating_add(1)];
        assert!(matches!(
            refused_start(oversized.path(), Some(&bytes)).await,
            Error::Launch(_)
        ));
        let absent = tempfile::tempdir().unwrap();
        assert!(matches!(
            refused_start(absent.path(), None).await,
            Error::Deadline {
                operation: vessel_local_node::Operation::Start,
                ..
            }
        ));
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn caller_cancellation_and_wrong_ready_identity_cleanup_owned_children() {
    use tokio::io::AsyncReadExt as _;
    vessel_testkit::within(SAFETY_NET, async {
        let cancelled = tempfile::tempdir().unwrap();
        let (starting, mut control, _logs) = stub_start(cancelled.path(), None).await;
        starting.abort();
        assert!(starting.await.unwrap_err().is_cancelled());
        let mut rest = Vec::new();
        let _eof = control.read_to_end(&mut rest).await.unwrap();

        let wrong = tempfile::tempdir().unwrap();
        let directory = vessel_node::DataDir::new(wrong.path().join("node"));
        directory.prepare().unwrap();
        let expected = directory.load_or_create_node_id().unwrap();
        let _credential = directory.load_or_create_token().unwrap();
        let found = vessel_proto::NodeId::random();
        assert_ne!(expected, found);
        let ready = vessel_proto::local_launch::Ready::new(found).encode();
        assert!(matches!(refused_start(wrong.path(), Some(ready.as_bytes())).await, Error::Identity { expected: wanted, found: observed } if wanted == expected && observed == found));
    }).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn authenticated_socket_identity_mismatch_never_becomes_owned_or_stops_the_peer() {
    vessel_testkit::within(SAFETY_NET, async {
        let serving = tempfile::tempdir().unwrap();
        let mut actual = LocalNode::start(launch(serving.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")))).await.unwrap();
        let actual_id = actual.client().unwrap().node().id;
        let wrong = tempfile::tempdir().unwrap();
        let home = vessel_node::DataDir::new(wrong.path().join("node"));
        home.prepare().unwrap();
        let expected = home.load_or_create_node_id().unwrap();
        assert_ne!(expected, actual_id);
        let _copied = std::fs::copy(serving.path().join("node/token"), home.token_path()).unwrap();
        std::os::unix::fs::symlink(serving.path().join("node/vesseld.sock"), home.socket_path()).unwrap();
        let result = LocalNode::start(launch(wrong.path(), Path::new(env!("CARGO_BIN_EXE_vesseld")))).await;
        assert!(matches!(result, Err(Error::Identity { expected: wanted, found }) if wanted == expected && found == actual_id));
        assert_eq!(actual.client().unwrap().node_info().await.unwrap().state, NodeState::Serving);
        let _exit = actual.stop(StopPolicy::Protect).await.unwrap();
    }).await.unwrap();
}
