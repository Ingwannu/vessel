//! Actual CLI input and pending-call refusal against the disposable fake node.
use crate::support::{Fixture, Ran};
use std::{future::Future, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt as _;

const PHASE: Duration = Duration::from_secs(15);
async fn phase<T>(name: &str, job: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, job)
        .await
        .unwrap_or_else(|_| panic!("launch CLI phase timed out: {name}"))
}
async fn run(fixture: &Fixture, args: &[&str]) -> Ran {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_vessel"));
    let _configured = command
        .arg("--node")
        .arg(fixture.node_address())
        .arg("--data-dir")
        .arg(fixture.data())
        .args(args)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", fixture.data())
        .env("VESSEL_CONFIG_DIR", fixture.data().join("config"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap().take(8193);
    let mut stderr = child.stderr.take().unwrap().take(8193);
    let output = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _read = stdout.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    let error = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _read = stderr.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    let status = if let Ok(status) = tokio::time::timeout(PHASE, child.wait()).await {
        status.unwrap()
    } else {
        let _already_exited = child.start_kill();
        let _killed = phase("CLI killed and reaped", child.wait()).await.unwrap();
        let _out = phase("killed CLI stdout drains", output).await.unwrap();
        let _err = phase("killed CLI stderr drains", error).await.unwrap();
        panic!("launch CLI did not complete")
    };
    let stdout = phase("CLI stdout drains", output).await.unwrap();
    let stderr = phase("CLI stderr drains", error).await.unwrap();
    assert!(
        stdout.len() <= 8192 && stderr.len() <= 8192,
        "unexpected fixture output volume"
    );
    Ran {
        ok: status.success(),
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn launch_cli_preserves_pending_refusal_for_every_typed_operation() {
    vessel_testkit::within(
        Duration::from_secs(90),
        Box::pin(async {
            assert_eq!(
                vessel_proto::launch_profiles::NAMED_LAUNCH_PROFILES,
                None,
                "assigned happy-wire coverage is a separate integration obligation"
            );
            let fixture = Box::pin(phase("fixture starts", Fixture::start())).await;
            let input = tempfile::tempdir().unwrap();
            let replace = input.path().join("replace.json");
            let options = input.path().join("options.json");
            let apply = input.path().join("apply.json");
            std::fs::write(
                &replace,
                include_str!("../../../vessel-testkit/fixtures/launch-profiles-replace.json"),
            )
            .unwrap();
            std::fs::write(
                &options,
                include_str!("../../../vessel-testkit/fixtures/launch-profiles-options.json"),
            )
            .unwrap();
            let session = "131d469e-0b08-4596-bb5a-93b5c0ce9cc0";
            std::fs::write(
                &apply,
                format!("{{\"session\":\"{session}\",\"options\":[]}}"),
            )
            .unwrap();
            for (args, method) in [
                (vec!["list"], "launch_profile.list"),
                (
                    vec!["replace", replace.to_str().unwrap()],
                    "launch_profile.replace",
                ),
                (
                    vec!["options", options.to_str().unwrap()],
                    "launch_profile.options",
                ),
                (vec!["applicable", session], "launch_profile.applicable"),
                (
                    vec!["apply", apply.to_str().unwrap()],
                    "session.apply_options",
                ),
            ] {
                let mut command = vec!["launch-profiles"];
                command.extend(args);
                let result = run(&fixture, &command).await;
                assert!(!result.ok && result.stdout.is_empty());
                assert!(
                    result.stderr.contains(method) && result.stderr.contains("not assigned"),
                    "{}",
                    result.stderr
                );
            }
            phase("fixture stops", fixture.stop()).await;
        }),
    )
    .await
    .unwrap();
}
