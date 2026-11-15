//! Real owned fake helpers; no agent, network listener, credential or node store.
#![cfg(all(test, feature = "fixture"))]

use std::{error::Error, path::Path, time::Duration};

use tokio_util::sync::CancellationToken;
use vessel_plugin::{
    CallError, PreparedCommand, RuntimeHandle, RuntimeState, SharedLogs, Supervisor,
};
use vessel_proc::CommandSpec;
use vessel_proto::{
    NodeId, RawJson,
    plugins::{HelperCall, HelperPayload, PluginAddress, RuntimeGeneration},
};

const STATE_WAIT_BUDGET: Duration = Duration::from_secs(10);
#[cfg(target_os = "linux")]
const PROCESS_EXIT_BUDGET: Duration = Duration::from_secs(5);
#[cfg(target_os = "linux")]
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CLEANUP_WAIT_BUDGET: Duration = Duration::from_secs(8);

fn command(mode: &str, directory: &Path) -> CommandSpec {
    let mut command = CommandSpec::new(env!("CARGO_BIN_EXE_vessel-plugin-fixture"))
        .arg(mode)
        .env_clear()
        .cwd(directory);
    if cfg!(windows) {
        command = command.env(
            "SystemRoot",
            std::env::var_os("SystemRoot").expect("platform root"),
        );
    }
    command
}

fn address() -> PluginAddress {
    PluginAddress {
        node: NodeId::random(),
        plugin: "fixture".parse().expect("plugin"),
        generation: RuntimeGeneration::random(),
    }
}

async fn ready(handle: &RuntimeHandle) -> PluginAddress {
    let mut state = handle.subscribe();
    tokio::time::timeout(STATE_WAIT_BUDGET, async {
        loop {
            let current = state.borrow().clone();
            match current {
                RuntimeState::Running { address, .. } => return address,
                RuntimeState::Starting => state.changed().await.expect("ready state"),
                RuntimeState::Stopped => panic!("fixture stopped before ready"),
            }
        }
    })
    .await
    .expect("fixture handshake deadline")
}

fn call() -> HelperCall {
    HelperCall::Invoke {
        method: "echo".parse().expect("method"),
        input: HelperPayload::try_from(RawJson::from_text(r#"{"fixture":true}"#).expect("json"))
            .expect("bounded"),
    }
}

#[tokio::test]
async fn real_helper_echoes_only_live_registered_calls_and_withdraws_on_shutdown() {
    let directory = tempfile::tempdir().expect("isolated directory");
    let supervisor = Supervisor::spawn(
        PreparedCommand::new(command("echo", directory.path())).expect("prepared"),
        address(),
        SharedLogs::default(),
        &CancellationToken::new(),
    )
    .expect("spawn");
    let handle = supervisor.handle();
    let current = ready(&handle).await;
    let result = handle
        .call(current.clone(), call(), &CancellationToken::new())
        .await
        .expect("echo");
    assert_eq!(result.raw().as_text(), r#"{"fixture":true}"#);
    let mut stale = current;
    stale.generation = RuntimeGeneration::random();
    assert_eq!(
        handle.call(stale, call(), &CancellationToken::new()).await,
        Err(CallError::Stale)
    );
    let receipt = supervisor.shutdown().await.expect("reaped shutdown");
    assert_eq!(receipt.code(), Some(0));
    assert_eq!(*handle.subscribe().borrow(), RuntimeState::Stopped);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancelled_call_has_no_late_success_and_owner_drop_stops_retained_handles() {
    let directory = tempfile::tempdir().expect("isolated directory");
    let pid_path = directory.path().join("leader.pid");
    let supervisor = Supervisor::spawn(
        PreparedCommand::new(command("stall", directory.path()).arg(&pid_path)).expect("prepared"),
        address(),
        SharedLogs::default(),
        &CancellationToken::new(),
    )
    .expect("spawn");
    let handle = supervisor.handle();
    let current = ready(&handle).await;
    let pid = std::fs::read_to_string(&pid_path)
        .expect("fixture pid")
        .parse::<u32>()
        .expect("pid");
    let cancel = CancellationToken::new();
    let child = cancel.clone();
    let retained = handle.clone();
    let request = tokio::spawn(async move { retained.call(current, call(), &child).await });
    cancel.cancel();
    assert_eq!(request.await.expect("join"), Err(CallError::Cancelled));
    let mut state = handle.subscribe();
    drop(supervisor);
    tokio::time::timeout(STATE_WAIT_BUDGET, async {
        while *state.borrow() != RuntimeState::Stopped {
            state.changed().await.expect("withdraw");
        }
    })
    .await
    .expect("owner drop withdrawal");
    cannot_run(pid).await;
}

#[tokio::test]
async fn malformed_input_and_fatal_diagnostics_never_leak_through_error_chains() {
    for mode in ["malformed", "fatal"] {
        let directory = tempfile::tempdir().expect("isolated directory");
        let logs = SharedLogs::default();
        let supervisor = Supervisor::spawn(
            PreparedCommand::new(command(mode, directory.path())).expect("prepared"),
            address(),
            logs.clone(),
            &CancellationToken::new(),
        )
        .expect("spawn");
        let mut state = supervisor.handle().subscribe();
        tokio::time::timeout(STATE_WAIT_BUDGET, async {
            while *state.borrow() != RuntimeState::Stopped {
                state.changed().await.expect("withdraw");
            }
        })
        .await
        .expect("refusal deadline");
        let error = supervisor.shutdown().await.expect_err("invalid helper");
        let mut cause: &dyn Error = &error;
        loop {
            assert!(!format!("{cause:?} {cause}").contains("PRIVATE-HELPER-CANARY"));
            match cause.source() {
                Some(next) => cause = next,
                None => break,
            }
        }
        if mode == "fatal" {
            assert!(
                logs.snapshot()
                    .expect("management tail")
                    .iter()
                    .any(|entry| entry.message.as_str().contains("PRIVATE-HELPER-CANARY"))
            );
        }
    }
}

#[tokio::test]
async fn private_stderr_retains_bounded_prefix_and_last_line() {
    let directory = tempfile::tempdir().expect("isolated directory");
    let logs = SharedLogs::default();
    let supervisor = Supervisor::spawn(
        PreparedCommand::new(command("stderr-tail", directory.path())).expect("prepared"),
        address(),
        logs.clone(),
        &CancellationToken::new(),
    )
    .expect("spawn");
    let _address = ready(&supervisor.handle()).await;
    let receipt = supervisor.shutdown().await.expect("shutdown");
    assert!(!receipt.run().is_live());
    let entries = logs.snapshot().expect("management tail");
    assert!(
        entries
            .iter()
            .any(|entry| entry.truncation == vessel_proto::plugins::LogTruncation::PrefixRetained)
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.message.as_str() == "last diagnostic")
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry.message.as_str().len() <= vessel_proto::plugins::MAX_LOG_LINE_BYTES)
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn graceful_leader_exit_stops_its_descendant_with_private_account_owned_files() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let directory = tempfile::tempdir().expect("isolated directory");
    let pid_path = directory.path().join("descendant.pid");
    let spec = command("descendant", directory.path()).arg(&pid_path);
    let supervisor = Supervisor::spawn(
        PreparedCommand::new(spec).expect("prepared"),
        address(),
        SharedLogs::default(),
        &CancellationToken::new(),
    )
    .expect("spawn");
    let _address = ready(&supervisor.handle()).await;
    let pid = std::fs::read_to_string(&pid_path)
        .expect("fixture pid")
        .parse::<u32>()
        .expect("pid");
    let metadata = std::fs::metadata(&pid_path).expect("metadata");
    let account = std::fs::metadata("/proc/self").expect("account");
    assert_eq!(metadata.uid(), account.uid());
    assert_eq!(metadata.gid(), account.gid());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    let receipt = supervisor.shutdown().await.expect("group reaped");
    assert_eq!(receipt.code(), Some(0));
    cannot_run(pid).await;
}

#[cfg(target_os = "linux")]
async fn cannot_run(pid: u32) {
    tokio::time::timeout(PROCESS_EXIT_BUDGET, async {
        loop {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
            match stat {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => panic!("could not observe owned fixture process: {error}"),
                Ok(stat)
                    if stat
                        .rsplit_once(')')
                        .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z')) =>
                {
                    break;
                }
                Ok(_running) => {}
            }
            tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
        }
    })
    .await
    .expect("descendant cannot continue");
}

#[tokio::test]
async fn ignoring_dispose_requires_owned_group_termination_before_success() {
    let directory = tempfile::tempdir().expect("isolated directory");
    let supervisor = Supervisor::spawn(
        PreparedCommand::new(command("ignore-shutdown", directory.path())).expect("prepared"),
        address(),
        SharedLogs::default(),
        &CancellationToken::new(),
    )
    .expect("spawn");
    let _address = ready(&supervisor.handle()).await;
    let receipt = tokio::time::timeout(CLEANUP_WAIT_BUDGET, supervisor.shutdown())
        .await
        .expect("bounded cleanup")
        .expect("group reaped");
    assert!(!receipt.run().is_live());
}
