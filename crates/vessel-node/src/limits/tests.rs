//! Admission/cancellation proof with an actually occupied blocking pool.
//! All files are synthetic inside a temporary data directory.
use super::*;
use vessel_harness::{HarnessEnv, LimitEnvironment, Profile};
use vessel_proto::HarnessKind;

fn account() -> LimitAccount {
    LimitAccount {
        harness: HarnessKind::Codex.into(),
        profile: None,
    }
}
#[test]
fn rpc_cancellation_does_not_admit_replacement_blocking_reads() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let root = tempfile::tempdir().unwrap();
            let tasks = TaskTracker::new();
            let stop = CancellationToken::new();
            // Start a real blocking job before probing; no timing sleep is used.
            let (ready, started) = oneshot::channel();
            let (release, waiting) = std::sync::mpsc::channel();
            let block = tokio::task::spawn_blocking(move || {
                ready.send(()).unwrap();
                waiting.recv().unwrap();
            });
            started.await.unwrap();
            let (active, generation, binding) = actor(root.path(), stop.clone(), &tasks);
            let (reply, first_response) = oneshot::channel();
            active
                .commands
                .send(Command::Read {
                    account: account(),
                    binding: Ok(binding.clone()),
                    refresh: LimitRefresh::Refresh,
                    reply,
                })
                .await
                .unwrap();
            let seen = active
                .read(account(), Ok(binding.clone()), LimitRefresh::Cached)
                .await
                .unwrap();
            assert!(matches!(seen.state, AccountLimitState::Refreshing { .. }));
            let first = tokio::spawn(first_response);
            let another = LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: Some("another".parse().unwrap()),
            };
            let (reply, response) = oneshot::channel();
            active
                .commands
                .send(Command::Read {
                    account: another.clone(),
                    binding: Ok(binding.clone()),
                    refresh: LimitRefresh::Refresh,
                    reply,
                })
                .await
                .unwrap();
            let ready = active
                .read(another.clone(), Ok(binding.clone()), LimitRefresh::Cached)
                .await
                .unwrap();
            assert!(matches!(ready.state, AccountLimitState::Refreshing { .. }));
            first.abort();
            assert!(first.await.unwrap_err().is_cancelled());
            let third = LimitAccount {
                harness: HarnessKind::Codex.into(),
                profile: Some("third".parse().unwrap()),
            };
            assert_eq!(
                active
                    .read(third, Ok(binding), LimitRefresh::Refresh)
                    .await
                    .unwrap_err(),
                busy()
            );
            let off = active.clone();
            let changed = tokio::spawn(async move { off.policy(ExternalLookup::Off).await });
            // Releasing the pool lets cancelled reads finish and the actor's
            // write complete. Their old result cannot republish after Off.
            generation.cancelled().await;
            release.send(()).unwrap();
            block.await.unwrap();
            assert_eq!(changed.await.unwrap().unwrap(), ExternalLookup::Off);
            assert_eq!(
                response.await.unwrap().unwrap().state,
                AccountLimitState::Off
            );
            stop.cancel();
            let _closed = tasks.close();
            tasks.wait().await;
        })
        .await
        .expect("blocked quota jobs and policy did not drain");
    });
}

#[tokio::test]
async fn a_cancelled_policy_reply_does_not_abandon_its_saved_decision() {
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        let root = tempfile::tempdir().unwrap();
        let dir = DataDir::new(root.path());
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let active = Limits::start(
            dir.clone(),
            LimitProbe::Disabled,
            stop.clone(),
            &tasks,
            LimitContext::new(),
        )
        .await;
        let (reply, response) = oneshot::channel();
        active
            .commands
            .send(Command::Policy {
                policy: ExternalLookup::On,
                reply,
            })
            .await
            .unwrap();
        // Drop the RPC's receiver; the following command is a FIFO barrier
        // after the independent owner's write/apply, not a timer-based wait.
        drop(response);
        let report = active
            .read(
                account(),
                Err(LimitFailure::Unassociated),
                LimitRefresh::Cached,
            )
            .await
            .unwrap();
        assert_eq!(report.policy, ExternalLookup::On);
        assert_eq!(dir.read_usage_lookup().unwrap(), ExternalLookup::On);
        stop.cancel();
        let _closed = tasks.close();
        tasks.wait().await;
    })
    .await
    .expect("cancelled policy decision did not reach its barrier");
}

fn actor(
    root: &std::path::Path,
    stop: CancellationToken,
    tasks: &TaskTracker,
) -> (Limits, CancellationToken, LimitBinding) {
    let homes = HarnessEnv::new(root.to_owned());
    let binding = vessel_harness::limits_binding(
        &LimitEnvironment::new(),
        &homes,
        &Profile::default_for(HarnessKind::Codex),
    )
    .unwrap();
    // This backend queues actual local reads behind the occupied worker;
    // Disabled would finish immediately and prove no permit lifetime.
    let probe = LimitProbe::Recorded {
        root: root.to_owned(),
        body: root.join("never-read.json"),
        status: 200,
        retry_after: None,
    };
    let (commands, receiver) = mpsc::channel(COMMANDS);
    let generation = CancellationToken::new();
    let owner = Owner {
        dir: DataDir::new(root),
        policy: ExternalLookup::On,
        probe,
        cache: Vec::new(),
        generation: generation.clone(),
        jobs: JoinSet::new(),
        context: LimitContext::new(),
    };
    drop(tasks.spawn(owner.run(receiver, stop)));
    (
        Limits {
            commands,
            sources: crate::usage_reports::Sources::default(),
        },
        generation,
        binding,
    )
}
