//! The shutdown command must preserve sessions until a person chooses otherwise.
//! All harnesses are testkit fakes; nodes bind only OS-selected loopback ports.

use std::time::Duration;

use vessel_proto::{ErrorCode, NodeState, SessionStatus};
use vessel_store::DataDir;

use crate::support::{Fixture, within};

const HANDSHAKE_TEST_BUDGET: Duration = Duration::from_millis(50);

#[cfg(target_os = "linux")]
const LOCAL_READ_TEST_BUDGET: Duration = Duration::from_secs(5);

#[cfg(target_os = "linux")]
const LOCAL_READ_EXIT_BUDGET: Duration = Duration::from_secs(10);

async fn terminal(fixture: &Fixture) -> vessel_proto::SessionId {
    let started = fixture
        .vessel(&["run", "claude", "--detach", "--cwd", &fixture.cwd()], b"")
        .await;
    assert!(started.ok, "{}", started.stderr);
    started.stdout.trim().parse().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_stop_closes_the_connection_and_releases_the_lock() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let ran = fixture.vessel(&["node", "stop"], b"").await;
        assert!(ran.ok, "{}", ran.stderr);
        assert!(
            ran.stdout
                .contains("closed its shutdown connection; lock released")
        );
        owner.closed().await;
        let lock = std::fs::File::open(DataDir::new(fixture.data()).lock_path()).unwrap();
        // Check the CLI's postcondition before fixture cleanup can release it.
        lock.try_lock().unwrap();
        lock.unlock().unwrap();
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_sessions_are_listed_and_stay_running_without_a_timing_choice() {
    within(async {
        let fixture = Fixture::start().await;
        let session = terminal(&fixture).await;
        let ran = fixture.vessel(&["node", "stop"], b"").await;
        assert!(!ran.ok);
        assert!(ran.stdout.contains(&session.to_string()));
        assert!(ran.stderr.contains("not stopped"));
        let client = fixture.owner().await;
        assert!(client.session(session).await.unwrap().status.is_live());
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Serving);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn now_without_yes_on_a_pipe_interrupts_nothing() {
    within(async {
        let fixture = Fixture::start().await;
        let session = terminal(&fixture).await;
        let ran = fixture.vessel(&["node", "stop", "--now"], b"yes\n").await;
        assert!(!ran.ok);
        assert!(ran.stderr.contains("--yes"));
        let client = fixture.owner().await;
        assert!(client.session(session).await.unwrap().status.is_live());
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Serving);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drain_rejects_new_sessions_and_exits_only_after_the_live_session_ends() {
    within(async {
        let fixture = Fixture::start().await;
        let session = terminal(&fixture).await;
        let client = fixture.owner().await;
        let spec = client.session(session).await.unwrap().spec;
        let ran = fixture.vessel(&["node", "stop", "--when-idle"], b"").await;
        assert!(ran.ok, "{}", ran.stderr);
        assert!(ran.stdout.contains("draining"));
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Draining);
        let refused = client.create_session(spec).await.unwrap_err();
        assert_eq!(refused.code(), Some(ErrorCode::Unavailable));
        assert!(client.session(session).await.unwrap().status.is_live());
        let ended = fixture
            .vessel(&["attach", &session.to_string()], b"exit\r")
            .await;
        assert!(ended.ok, "{}", ended.stderr);
        client.closed().await;
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_interruption_is_persisted_and_not_resumed_on_restart() {
    within(async {
        let fixture = Fixture::start().await;
        let session = terminal(&fixture).await;
        let ran = fixture
            .vessel(&["node", "stop", "--now", "--yes"], b"")
            .await;
        assert!(ran.ok, "{}", ran.stderr);
        let fixture = fixture.restart().await;
        let client = fixture.owner().await;
        assert_eq!(
            client.session(session).await.unwrap().status,
            SessionStatus::Interrupted
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn command_deadline_covers_a_connection_that_never_completes_its_handshake() {
    within(async {
        let fixture = Fixture::start().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        // The listener stays open without completing the handshake. The only
        // timer is the product's deadline, not test synchronization.
        let seconds = HANDSHAKE_TEST_BUDGET.as_secs_f64().to_string();
        let ran = fixture
            .vessel_at(
                &address,
                fixture.data(),
                &["node", "stop", "--timeout", &seconds],
                b"",
            )
            .await;
        assert!(!ran.ok);
        assert!(ran.stderr.contains("--timeout"), "{}", ran.stderr);
        assert!(ran.stderr.contains(&seconds), "{}", ran.stderr);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stale_owner_record_does_not_claim_the_stopped_node_is_running() {
    within(async {
        let fixture = Fixture::start().await;
        let ran = fixture.vessel(&["node", "stop"], b"").await;
        assert!(ran.ok, "{}", ran.stderr);
        let stale = tempfile::tempdir().unwrap();
        let address = fixture.node_address();
        let stale_dir = DataDir::new(stale.path());
        let copied_bytes = std::fs::copy(
            DataDir::new(fixture.data()).owner_path(),
            stale_dir.owner_path(),
        )
        .unwrap();
        assert!(copied_bytes > 0);
        drop(std::fs::File::create(stale_dir.lock_path()).unwrap());
        fixture.stop().await;
        let ran =
            crate::support::vessel_in(stale.path(), &["--node", &address, "node", "stop"], b"")
                .await;
        assert!(!ran.ok);
        assert!(
            ran.stderr.contains("the node is not running"),
            "{}",
            ran.stderr
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreadable_live_owner_is_refused_without_shutting_down_the_node() {
    within(async {
        let fixture = Fixture::start().await;
        let dir = DataDir::new(fixture.data());
        std::fs::write(dir.owner_path(), "invalid owner").unwrap();
        let ran = fixture.vessel(&["node", "stop"], b"").await;
        assert!(!ran.ok);
        assert!(
            ran.stderr
                .contains("lock is held but its owner record is unavailable"),
            "{}",
            ran.stderr
        );
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_loopback_book_endpoint_uses_its_own_token_file_without_a_local_lock() {
    within(async {
        let fixture = Fixture::start().await;
        let client_home = tempfile::tempdir().unwrap();
        let address = fixture.node_address();
        let token_path = DataDir::new(fixture.data()).token_path();
        let token_file = token_path.to_str().unwrap();
        let added = crate::support::vessel_in(
            client_home.path(),
            &[
                "nodes",
                "add",
                "test-endpoint",
                &address,
                "--token-file",
                token_file,
            ],
            b"",
        )
        .await;
        assert!(added.ok, "{}", added.stderr);
        let stopped = crate::support::vessel_in(
            client_home.path(),
            &["--node", "test-endpoint", "node", "stop"],
            b"",
        )
        .await;
        assert!(stopped.ok, "{}", stopped.stderr);
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_book_endpoint_never_waits_for_another_running_local_nodes_lock() {
    within(async {
        let client_home = Fixture::start().await;
        let endpoint = Fixture::start().await;
        let address = endpoint.node_address();
        let token_path = DataDir::new(endpoint.data()).token_path();
        let added = crate::support::vessel_in(
            client_home.data(),
            &[
                "nodes",
                "add",
                "test-endpoint",
                &address,
                "--token-file",
                token_path.to_str().unwrap(),
            ],
            b"",
        )
        .await;
        assert!(added.ok, "{}", added.stderr);
        let stopped = crate::support::vessel_in(
            client_home.data(),
            &["--node", "test-endpoint", "node", "stop"],
            b"",
        )
        .await;
        assert!(stopped.ok, "{}", stopped.stderr);
        assert!(stopped.stdout.contains("closed its shutdown connection"));
        assert!(!stopped.stdout.contains("lock released"));
        assert_eq!(
            client_home.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        endpoint.stop().await;
        client_home.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_deadline_ends_a_process_with_a_blocked_identity_reader() {
    within(async {
        let fixture = Fixture::start().await;
        // The persisted identity's format is part of the data-directory contract.
        // A blocked read must not hold the CLI's blocking pool alive after expiry.
        let file = fixture.data().join("node.id");
        std::fs::remove_file(&file).unwrap();
        assert!(
            tokio::process::Command::new("mkfifo")
                .arg(&file)
                .status()
                .await
                .unwrap()
                .success()
        );
        let seconds = LOCAL_READ_TEST_BUDGET.as_secs_f64().to_string();
        let stopped = stop_with_blocked_reader(&fixture, &seconds).await;
        assert!(!stopped.ok);
        assert!(stopped.stderr.contains("--timeout"), "{}", stopped.stderr);
        assert!(stopped.stderr.contains(&seconds), "{}", stopped.stderr);
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        fixture.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_deadline_ends_a_process_with_a_blocked_lock_preflight() {
    within(async {
        let fixture = Fixture::start().await;
        let file = DataDir::new(fixture.data()).lock_path();
        std::fs::remove_file(&file).unwrap();
        assert!(
            tokio::process::Command::new("mkfifo")
                .arg(&file)
                .status()
                .await
                .unwrap()
                .success()
        );
        let seconds = LOCAL_READ_TEST_BUDGET.as_secs_f64().to_string();
        let stopped = stop_with_blocked_reader(&fixture, &seconds).await;
        assert!(!stopped.ok);
        assert!(stopped.stderr.contains("--timeout"), "{}", stopped.stderr);
        assert!(stopped.stderr.contains(&seconds), "{}", stopped.stderr);
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        fixture.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
async fn stop_with_blocked_reader(fixture: &Fixture, seconds: &str) -> crate::support::Ran {
    let mut child = fixture.spawn_vessel(&["node", "stop", "--timeout", seconds]);
    drop(child.stdin.take());
    // Retain the child so a regression cannot leave its blocked reader running
    // after the test's own named exit deadline has diagnosed the hang.
    let waited = tokio::time::timeout(LOCAL_READ_EXIT_BUDGET, child.wait()).await;
    if waited.is_err() {
        child.kill().await.unwrap();
        panic!("blocked-reader CLI did not exit within {LOCAL_READ_EXIT_BUDGET:?}");
    }
    assert!(!waited.unwrap().unwrap().success());
    child.wait_with_output().await.unwrap().into()
}

/// This location mirrors the isolated `HOME`/`XDG_CONFIG_HOME` in the fixture,
/// never the actual user's service namespace.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn service_file(home: &std::path::Path) -> std::path::PathBuf {
    if cfg!(target_os = "linux") {
        home.join("config/systemd/user/vessel.service")
    } else {
        home.join("Library/LaunchAgents/vessel.node.plist")
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_warns_about_a_personal_service_without_changing_or_invoking_it() {
    within(async {
        let fixture = Fixture::start().await;
        let file = service_file(fixture.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let contents = "a personal service; never execute this text\n";
        std::fs::write(&file, contents).unwrap();
        let tools = tempfile::tempdir().unwrap();
        let log = fixture.data().join("service-calls.log");
        let fake = vessel_testkit::Fake::Tool.path().unwrap();
        for name in ["systemctl", "launchctl"] {
            std::os::unix::fs::symlink(&fake, tools.path().join(name)).unwrap();
        }
        // Any manager invocation would reach a logging stand-in, never the
        // real user service namespace.
        let stopped = fixture
            .vessel_book(
                fixture.data(),
                &["--node", &fixture.node_address(), "node", "stop"],
                &[
                    ("PATH", tools.path().as_os_str()),
                    ("FAKE_TOOL_LOG", log.as_os_str()),
                ],
            )
            .await;
        assert!(stopped.ok, "{}", stopped.stderr);
        assert!(
            stopped.stderr.contains("service differs"),
            "{}",
            stopped.stderr
        );
        assert!(
            stopped.stdout.contains("lock released"),
            "{}",
            stopped.stdout
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), contents);
        assert!(!log.exists(), "service manager was invoked");
        fixture.stop().await;
    })
    .await;
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_refuses_an_unreadable_service_before_requesting_shutdown() {
    within(async {
        let fixture = Fixture::start().await;
        let file = service_file(fixture.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let contents = b"invalid UTF-8: \xff";
        std::fs::write(&file, contents).unwrap();
        let stopped = fixture.vessel(&["node", "stop"], b"").await;
        assert!(!stopped.ok);
        assert!(
            stopped.stderr.contains("cannot inspect the user service"),
            "{}",
            stopped.stderr
        );
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        assert_eq!(std::fs::read(&file).unwrap(), contents);
        fixture.stop().await;
    })
    .await;
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_warns_during_drain_without_waiting_for_or_interrupting_live_sessions() {
    within(async {
        let fixture = Fixture::start().await;
        let session = terminal(&fixture).await;
        let file = service_file(fixture.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "personal unit").unwrap();
        let stopped = fixture.vessel(&["node", "stop", "--when-idle"], b"").await;
        assert!(stopped.ok, "{}", stopped.stderr);
        assert!(
            stopped.stderr.contains("service differs"),
            "{}",
            stopped.stderr
        );
        assert!(stopped.stdout.contains("draining"));
        let client = fixture.owner().await;
        assert_eq!(client.node_info().await.unwrap().state, NodeState::Draining);
        assert!(client.session(session).await.unwrap().status.is_live());
        fixture.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_deadline_also_ends_a_process_with_a_blocked_service_reader() {
    within(async {
        let fixture = Fixture::start().await;
        let file = service_file(fixture.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        assert!(
            tokio::process::Command::new("mkfifo")
                .arg(&file)
                .status()
                .await
                .unwrap()
                .success()
        );
        // The FIFO has no writer. Its blocking open must not keep the CLI's
        // runtime alive after the product deadline expires.
        let seconds = LOCAL_READ_TEST_BUDGET.as_secs_f64().to_string();
        let stopped = stop_with_blocked_reader(&fixture, &seconds).await;
        assert!(!stopped.ok);
        assert!(stopped.stderr.contains("--timeout"), "{}", stopped.stderr);
        assert!(stopped.stderr.contains(&seconds), "{}", stopped.stderr);
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        fixture.stop().await;
    })
    .await;
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_does_not_read_another_directorys_malformed_service() {
    within(async {
        use vessel_client::book::{BookEntry, BookFile, TokenSource};

        let home = Fixture::start().await;
        let endpoint = Fixture::start().await;
        let file = service_file(home.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let contents = b"not this node's file: \xff";
        std::fs::write(&file, contents).unwrap();
        let entry = BookEntry {
            address: endpoint.node_address().parse().unwrap(),
            token: Some(TokenSource::File(
                DataDir::new(endpoint.data()).token_path(),
            )),
            credential: None,
            id: Some(endpoint.owner().await.node().id),
        };
        BookFile::open(&home.book())
            .unwrap()
            .add(&"endpoint".parse().unwrap(), &entry)
            .unwrap();
        let stopped = home
            .vessel_book(home.data(), &["--node", "endpoint", "node", "stop"], &[])
            .await;
        assert!(stopped.ok, "{}", stopped.stderr);
        assert!(!stopped.stderr.contains("service"), "{}", stopped.stderr);
        assert!(!stopped.stdout.contains("lock released"));
        assert_eq!(std::fs::read(&file).unwrap(), contents);
        assert_eq!(
            home.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        endpoint.stop().await;
        home.stop().await;
    })
    .await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_stop_cannot_send_shutdown_when_its_service_warning_cannot_be_written() {
    within(async {
        let fixture = Fixture::start().await;
        let file = service_file(fixture.data());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "personal unit").unwrap();
        // The stderr peer was closed before exec, so the write failure has
        // no race with a reader deciding when to drop its end of the pipe.
        let mut child = fixture.spawn_vessel_with_closed_stderr(&["node", "stop"]);
        let status = tokio::time::timeout(LOCAL_READ_EXIT_BUDGET, child.wait()).await;
        assert!(
            status.is_ok(),
            "warning failure did not exit within {LOCAL_READ_EXIT_BUDGET:?}"
        );
        assert!(!status.unwrap().unwrap().success());
        assert_eq!(
            fixture.owner().await.node_info().await.unwrap().state,
            NodeState::Serving
        );
        fixture.stop().await;
    })
    .await;
}
