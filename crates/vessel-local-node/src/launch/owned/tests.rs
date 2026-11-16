//! Release refusal must return authority to the controller rather than an error.

use super::*;
use std::io::{Read as _, Write as _};
use tokio::io::AsyncWriteExt as _;
use vessel_proc::{CommandSpec, spawn};

const SAFETY_NET: Duration = Duration::from_secs(30);
const PHASE_DEADLINE: Duration = Duration::from_secs(5);

async fn step<F: Future>(name: &str, operation: F) -> F::Output {
    tokio::time::timeout(PHASE_DEADLINE, operation)
        .await
        .unwrap_or_else(|error| panic!("{name} timed out: {error}"))
}

const READY: &str = "owned fixture ready\n";
const FIXTURE_ENV: &str = "VESSEL_LOCAL_NODE_KEEP_REFUSAL_FIXTURE";

#[test]
fn bound_child_fixture() {
    if std::env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(READY.as_bytes()).unwrap();
    stdout.flush().unwrap();
    let mut stdin = std::io::stdin().lock();
    let mut byte = [0_u8; 1];
    while stdin.read_exact(&mut byte).is_ok() {
        assert_eq!(byte, [b'?']);
        stdout.write_all(b"!").unwrap();
        stdout.flush().unwrap();
    }
}

#[tokio::test]
async fn refused_keep_preserves_the_live_owner_until_explicit_drop() {
    tokio::time::timeout(SAFETY_NET, async {
        let command = CommandSpec::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch::owned::tests::bound_child_fixture",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, "1");
        let Spawned {
            child,
            mut stdin,
            stdout,
            stderr,
        } = spawn(&command).unwrap();
        let authority = child.lifetime();
        let owned = Owned {
            child,
            logs: LogReader::collect(stderr),
            witness: Default::default(),
        };
        let mut output = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            assert_ne!(output.read_line(&mut line).await.unwrap(), 0);
            if line.ends_with(READY) {
                break;
            }
        }
        let retained = match owned.keep() {
            Ok(_kept) => panic!("ordinary Bound child must refuse release"),
            Err(owned) => owned,
        };
        assert!(authority.is_live());
        stdin.write_all(b"?").await.unwrap();
        let mut reply = [0_u8; 1];
        let _read = output.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [b'!']);
        drop(retained);
        authority.ended().await;
        assert!(!authority.is_live());
        let mut rest = Vec::new();
        let _eof = output.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty());
    })
    .await
    .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn peer_qualification_requires_the_exact_unreaped_owner() {
    vessel_testkit::within(SAFETY_NET, async {
        let command = CommandSpec::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch::owned::tests::bound_child_fixture",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, "1");
        let Spawned {
            child,
            stdin,
            stdout,
            stderr,
        } = spawn(&command).unwrap();
        let process = std::num::NonZeroU32::new(child.pid()).unwrap();
        let wrong = std::num::NonZeroU32::new(if process.get() == u32::MAX {
            1
        } else {
            process.get() + 1
        })
        .unwrap();
        let mut owned = Owned {
            child,
            logs: LogReader::collect(stderr),
            witness: Default::default(),
        };
        let mut stdout = BufReader::new(stdout);
        step("owned fixture readiness", async {
            loop {
                let mut line = String::new();
                assert_ne!(stdout.read_line(&mut line).await.unwrap(), 0);
                if line.ends_with(READY) {
                    break;
                }
            }
        })
        .await;
        assert!(owned.qualify_peer(Some(process)).is_ok());
        assert!(matches!(
            owned.qualify_peer(None),
            Err(LaunchError::PeerUnverified)
        ));
        assert!(matches!(
            owned.qualify_peer(Some(wrong)),
            Err(LaunchError::PeerUnverified)
        ));
        drop(stdin);
        let _reaped = step("original owner checked exit", owned.wait())
            .await
            .unwrap();
        assert!(
            matches!(
                owned.qualify_peer(Some(process)),
                Err(LaunchError::PeerUnverified)
            ),
            "a stale numeric match cannot recover reaped child authority"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn first_record_completes_without_eof_and_refuses_prefetched_duplicates() {
    vessel_testkit::within(SAFETY_NET, async {
        let ready = Ready::new(vessel_proto::NodeId::random());
        let encoded = ready.encode();
        let (reader, mut retained_stdout) = tokio::io::duplex(MAX_READY_BYTES * 2);
        retained_stdout.write_all(encoded.as_bytes()).await.unwrap();
        let line = step(
            "first record without daemon EOF",
            record(reader, MAX_READY_BYTES),
        )
        .await
        .unwrap();
        assert_eq!(Ready::decode(&line).unwrap(), ready);
        // The healthy child stdout writer is deliberately still open here.
        drop(retained_stdout);
        let duplicate = format!("{encoded}{encoded}");
        assert!(matches!(
            record(duplicate.as_bytes(), MAX_READY_BYTES).await,
            Err(LaunchError::ExtraReadiness)
        ));
        let oversized = "x".repeat(MAX_READY_BYTES + 1);
        let line = record(oversized.as_bytes(), MAX_READY_BYTES).await.unwrap();
        assert!(matches!(
            Ready::decode(&line),
            Err(vessel_proto::local_launch::ReadyError::TooLong)
        ));
    })
    .await
    .unwrap();
}

#[cfg(target_os = "linux")]
mod reaping;

#[cfg(target_os = "linux")]
mod peer;
