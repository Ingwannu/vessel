//! Driven state-machine proof; these closures do not claim assigned wire traffic.

use super::*;
use crate::launch_profiles::migration::fixtures::*;
use std::{
    future::{Future, Ready, ready as immediate},
    sync::{Arc, Mutex},
};
use vessel_proto::{ErrorCode, WireError};

const PHASE: Duration = Duration::from_secs(5);
const SAFETY: Duration = Duration::from_secs(20);
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    phase_for(name, PHASE, future).await
}
async fn phase_for<T>(name: &str, limit: Duration, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(limit, future)
        .await
        .unwrap_or_else(|_| panic!("phase timed out: {name}"))
}
#[derive(Clone, Copy)]
enum WriteMode {
    Save,
    Fail,
    ConflictOnce,
    AlwaysStale,
    UnknownApplied,
    UnknownAbsent,
    WrongAck,
    WrongRevision,
    AheadRevision,
}
struct Host {
    current: LaunchProfilesSnapshot,
    reads: usize,
    writes: usize,
    mode: WriteMode,
}
fn host(mode: WriteMode) -> Arc<Mutex<Host>> {
    Arc::new(Mutex::new(Host {
        current: snapshot(0, Vec::new()),
        reads: 0,
        writes: 0,
        mode,
    }))
}
fn read(host: &Arc<Mutex<Host>>) -> Ready<Result<LaunchProfilesSnapshot, CallError>> {
    let mut host = host.lock().unwrap();
    host.reads = host.reads.checked_add(1).unwrap();
    immediate(Ok(host.current.clone()))
}
fn stale_error(current: LaunchProfileRevision) -> CallError {
    CallError::Remote(WireError {
        code: ErrorCode::Conflict,
        message: "changed".to_owned(),
        detail: Some(ErrorDetail::LaunchProfile {
            problem: LaunchProfileRefusal::StaleRevision { current },
        }),
    })
}
fn write(
    host: &Arc<Mutex<Host>>,
    expected: LaunchProfileRevision,
    profiles: LaunchProfiles,
) -> Ready<Result<LaunchProfilesSnapshot, CallError>> {
    let mut host = host.lock().unwrap();
    host.writes = host.writes.checked_add(1).unwrap();
    let result = match host.mode {
        WriteMode::ConflictOnce | WriteMode::AlwaysStale => {
            let manual = profile(&format!("manual {}", host.writes), Vec::new());
            let mut list = host.current.profiles().as_slice().to_vec();
            list.push(manual);
            host.current = LaunchProfilesSnapshot::new(
                host.current.revision().next().unwrap(),
                list.try_into().unwrap(),
            );
            if matches!(host.mode, WriteMode::ConflictOnce) {
                host.mode = WriteMode::Save;
            }
            Err(stale_error(host.current.revision()))
        }
        WriteMode::Fail => Err(CallError::Remote(WireError::new(
            ErrorCode::Internal,
            "test disk failure",
        ))),
        WriteMode::UnknownAbsent => Err(CallError::Disconnected),
        WriteMode::WrongAck => Ok(host.current.clone()),
        WriteMode::WrongRevision => Ok(LaunchProfilesSnapshot::new(expected, profiles)),
        WriteMode::AheadRevision => Ok(LaunchProfilesSnapshot::new(
            expected.next().unwrap().next().unwrap(),
            profiles,
        )),
        WriteMode::Save | WriteMode::UnknownApplied => {
            assert_eq!(host.current.revision(), expected);
            host.current = LaunchProfilesSnapshot::new(expected.next().unwrap(), profiles);
            if matches!(host.mode, WriteMode::UnknownApplied) {
                Err(CallError::Disconnected)
            } else {
                Ok(host.current.clone())
            }
        }
    };
    immediate(result)
}
async fn run(host: &Arc<Mutex<Host>>) -> Result<LegacyMigrationReceipt, LaunchProfileClientError> {
    phase(
        "migration state machine",
        execute(
            node(),
            favorites(&["m0", "m1", "m0"]),
            |_| immediate(Ok(LegacyCatalogueState::Ready(ready_catalogue()))),
            || read(host),
            |rev, profiles| write(host, rev, profiles),
        ),
    )
    .await
}
fn ready_catalogue() -> MigrationCatalogue {
    ready()
}

#[tokio::test]
async fn empty_inputs_do_not_read_catalogue_or_host_or_write() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let receipt = phase(
                "empty input completion",
                execute(
                    node(),
                    LegacyFavorites::default(),
                    |_| async { panic!("catalogue read") },
                    || async { panic!("host read") },
                    |_, _| async { panic!("host write") },
                ),
            )
            .await
            .unwrap();
            assert_eq!(receipt.node(), node());
            assert!(matches!(
                receipt.completion(),
                LegacyMigrationCompletion::NoFavorites
            ));
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn stale_rebuilds_preserve_manual_changes_and_persistent_failure_is_finite() {
    vessel_testkit::within(SAFETY, Box::pin(async {
        let one_conflict = host(WriteMode::ConflictOnce);
        let receipt = run(&one_conflict).await.unwrap();
        assert!(matches!(receipt.completion(), LegacyMigrationCompletion::Saved { revision, added } if revision.get() == 2 && added.len() == 2));
        {
            let saved = one_conflict.lock().unwrap();
            assert_eq!((saved.reads, saved.writes), (2, 2));
            assert_eq!(saved.current.profiles().as_slice().first().unwrap().display_name().as_str(), "manual 1");
        }
        let repeated = host(WriteMode::AlwaysStale);
        assert!(matches!(run(&repeated).await, Err(LaunchProfileClientError::LegacyConflictBudget { last: Some(_) })));
        assert_eq!(repeated.lock().unwrap().writes, MAX_CAS_ATTEMPTS);
    })).await.unwrap();
}

#[tokio::test]
async fn failed_write_remains_retryable_and_wrong_ack_is_not_completion() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let failed = host(WriteMode::Fail);
            assert!(matches!(
                run(&failed).await,
                Err(LaunchProfileClientError::Call(CallError::Remote(_)))
            ));
            assert!(
                failed
                    .lock()
                    .unwrap()
                    .current
                    .profiles()
                    .as_slice()
                    .is_empty()
            );
            failed.lock().unwrap().mode = WriteMode::Save;
            assert!(matches!(
                run(&failed).await.unwrap().completion(),
                LegacyMigrationCompletion::Saved { .. }
            ));
            for mode in [
                WriteMode::WrongAck,
                WriteMode::WrongRevision,
                WriteMode::AheadRevision,
            ] {
                let wrong = host(mode);
                assert!(matches!(
                    run(&wrong).await,
                    Err(LaunchProfileClientError::LegacyWriteNotConfirmed)
                ));
                assert_eq!(wrong.lock().unwrap().writes, 1);
            }
            let forged = CallError::Remote(WireError {
                code: ErrorCode::Forbidden,
                message: "denied".to_owned(),
                detail: Some(ErrorDetail::LaunchProfile {
                    problem: LaunchProfileRefusal::StaleRevision {
                        current: LaunchProfileRevision::ZERO,
                    },
                }),
            });
            assert!(!stale(&forged));
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn unknown_write_ack_is_read_before_completion_and_never_blindly_replayed() {
    vessel_testkit::within(SAFETY, Box::pin(async {
        let applied = host(WriteMode::UnknownApplied);
        assert!(matches!(run(&applied).await.unwrap().completion(), LegacyMigrationCompletion::AlreadyPresent { revision } if revision.get() == 1));
        let counts = { let host = applied.lock().unwrap(); (host.reads, host.writes) };
        assert_eq!(counts, (2, 1));
        let absent = host(WriteMode::UnknownAbsent);
        let error = run(&absent).await.unwrap_err();
        assert!(matches!(error, LaunchProfileClientError::LegacyUnknownWrite { write: CallError::Disconnected, reconciliation: None }));
        assert_eq!(absent.lock().unwrap().writes, 1);
        // A new attempt starts with a fresh read. It may safely finish a
        // later known append; the original uncertain call was never replayed.
        absent.lock().unwrap().mode = WriteMode::Save;
        assert!(matches!(run(&absent).await.unwrap().completion(), LegacyMigrationCompletion::Saved { .. }));
        assert_eq!(absent.lock().unwrap().writes, 2);
    })).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn only_explicit_loading_or_busy_uses_the_exact_source_retry_schedule() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let host = host(WriteMode::Save);
            let mut loads = 0;
            let began = tokio::time::Instant::now();
            let result = phase_for(
                "loading catalogue completion",
                Duration::from_millis(12_850),
                execute(
                    node(),
                    favorites(&["m0"]),
                    |_| {
                        loads += 1;
                        immediate(Ok(if loads <= CATALOGUE_LOADING_DELAYS.len() {
                            LegacyCatalogueState::Loading
                        } else {
                            LegacyCatalogueState::Ready(ready_catalogue())
                        }))
                    },
                    || read(&host),
                    |revision, profiles| write(&host, revision, profiles),
                ),
            )
            .await
            .unwrap();
            assert!(matches!(
                result.completion(),
                LegacyMigrationCompletion::Saved { .. }
            ));
            assert_eq!(loads, 7);
            assert_eq!(began.elapsed(), Duration::from_millis(7_850));
            let mut unavailable = 0;
            let error = phase(
                "ordinary unavailable is not retried",
                load_one(&"provider".parse().unwrap(), &mut |_| {
                    unavailable += 1;
                    immediate(Err(LaunchProfileClientError::Call(CallError::Remote(
                        WireError::new(ErrorCode::Unavailable, "ordinary failure"),
                    ))))
                }),
            )
            .await
            .unwrap_err();
            assert!(matches!(
                error,
                LaunchProfileClientError::Call(CallError::Remote(_))
            ));
            assert_eq!(unavailable, 1);
            let mut busy = 0;
            let began = tokio::time::Instant::now();
            let error = phase_for(
                "busy catalogue exhausts source schedule",
                Duration::from_millis(12_850),
                load_one(&"provider".parse().unwrap(), &mut |_| {
                    busy += 1;
                    immediate(Ok(LegacyCatalogueState::Busy))
                }),
            )
            .await
            .unwrap_err();
            assert!(matches!(
                error,
                LaunchProfileClientError::LegacyCatalogueLoading
            ));
            assert_eq!(busy, 7);
            assert_eq!(began.elapsed(), Duration::from_millis(7_850));
        }),
    )
    .await
    .unwrap();
}
