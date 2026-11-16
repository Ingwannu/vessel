//! Original failure and cleanup settlement stay independent, with no PID polling.

use crate::launch::cleanup_witness::{self, Settlement};
use crate::{Launch, LaunchError, LocalNode, LocalNodeError, NodeHome};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _};
use tokio::sync::oneshot;

const SAFETY_NET: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const START_AND_CLEANUP: Duration = Duration::from_secs(10);

#[derive(Clone, Copy)]
enum Termination {
    Accepted,
    Ignored,
}

async fn step<F: Future>(name: &str, duration: Duration, operation: F) -> F::Output {
    tokio::time::timeout(duration, operation)
        .await
        .unwrap_or_else(|error| panic!("{name} timed out: {error}"))
}

fn recipe(root: &std::path::Path, binary: std::path::PathBuf, termination: Termination) -> Launch {
    let bad_record = root.join("bad-ready");
    std::fs::write(&bad_record, b"invalid managed readiness\n").unwrap();
    Launch::new(
        binary,
        NodeHome::new(root.join("node")).unwrap(),
        vessel_proto::ClientInfo {
            name: "startup-cleanup-test".to_owned(),
            version: "test".to_owned(),
        },
    )
    .unwrap()
    .environment(|command| {
        let command = command
            .env_clear()
            .env("HOME", root)
            .env("FAKE_LOCAL_CONTROL_SOCKET", root.join("control"))
            .env("FAKE_LOCAL_READY_FILE", bad_record);
        match termination {
            Termination::Accepted => command,
            Termination::Ignored => command.env("FAKE_LOCAL_IGNORE_TERMINATE", "1"),
        }
    })
    .unwrap()
}

async fn exercise(binary: std::path::PathBuf, termination: Termination) {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::UnixListener::bind(root.path().join("control")).unwrap();
    let launch = recipe(root.path(), binary, termination);
    let (settled, settlement) = oneshot::channel();
    let (dropped, drop_observed) = oneshot::channel();
    let starting = cleanup_witness::observe(settled, dropped, LocalNode::start(launch));
    let control = async {
        let (stream, _) = step(
            "original fixture control connection",
            PHASE,
            listener.accept(),
        )
        .await
        .unwrap();
        let mut stream = tokio::io::BufReader::new(stream);
        let mut entered = String::new();
        let _read = step(
            "original fixture handler-ready ACK",
            PHASE,
            stream.read_line(&mut entered),
        )
        .await
        .unwrap();
        assert_eq!(entered, "started\n");
        stream
    };
    let (result, mut control) = tokio::join!(
        step(
            "failed start and named cleanup phase",
            START_AND_CLEANUP,
            starting
        ),
        control,
    );
    let error = result.unwrap_err();
    let settlement = step("same cleanup owner settlement witness", PHASE, settlement)
        .await
        .unwrap();
    match termination {
        Termination::Accepted => {
            assert!(matches!(
                error,
                LocalNodeError::Launch(LaunchError::Ready(_))
            ));
            assert!(matches!(settlement, Settlement::Reaped(_)));
        }
        Termination::Ignored => {
            let LocalNodeError::CleanupUnconfirmed { start, cleanup } = &error else {
                panic!("ignored SIGTERM must preserve cleanup uncertainty: {error:?}");
            };
            assert!(matches!(
                start.as_ref(),
                LocalNodeError::Launch(LaunchError::Ready(_))
            ));
            assert!(matches!(cleanup, LaunchError::CleanupDeadline(_)));
            assert_eq!(settlement, Settlement::Unconfirmed);
            assert!(
                error
                    .to_string()
                    .contains("hard kill requested but not observed")
            );
        }
    }
    step(
        "retained owner dropped after cleanup outcome",
        PHASE,
        drop_observed,
    )
    .await
    .unwrap();
    let mut remaining = Vec::new();
    let _eof = step(
        "original fixture control EOF after cleanup",
        PHASE,
        control.read_to_end(&mut remaining),
    )
    .await
    .unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn failed_start_reaps_original_child_and_preserves_the_start_error() {
    let binary = vessel_testkit::Fake::LocalNode.path().unwrap();
    vessel_testkit::within(SAFETY_NET, exercise(binary, Termination::Accepted))
        .await
        .unwrap();
}

#[tokio::test]
async fn ignored_terminate_reports_unconfirmed_and_drops_the_original_owner_at_once() {
    let binary = vessel_testkit::Fake::LocalNode.path().unwrap();
    vessel_testkit::within(SAFETY_NET, exercise(binary, Termination::Ignored))
        .await
        .unwrap();
}
