//! Hold the actual Producer command-preparation closure, not a synthetic future.
use super::*;
use crate::providers::owner::{MAX_JOBS, Owner, OwnerError, PER_PROVIDER, REFRESH_TIMEOUT};
use tokio::{sync::mpsc, task::JoinHandle};
use vessel_proto::providers::{Enabled, ProviderDiagnostic, ProviderKind, ProviderState};

const PREPARATION_START_TIMEOUT: Duration = Duration::from_secs(10);
const DEADLINE_SLACK: Duration = Duration::from_secs(1);
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
const GLOBAL_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const SATURATED_PROVIDERS: usize = MAX_JOBS.div_ceil(PER_PROVIDER);

fn fixtures(root: &std::path::Path, count: usize) -> ProviderOverrides {
    let fake = vessel_testkit::Fake::CodexAppServer.path().unwrap();
    ProviderOverrides::new(
        (0..count)
            .map(|index| {
                (
                    format!("fixture-{index}").parse().unwrap(),
                    ProviderOverride {
                        extends: Some(ProviderKind::Acp),
                        label: Some("Fixture".parse().unwrap()),
                        enabled: Some(Enabled::On),
                        command: Some(CommandEdit::Replace {
                            program: fake.to_str().unwrap().parse().unwrap(),
                            args: Arguments::default(),
                        }),
                        env: ProviderEnvironment::new(BTreeMap::from([(
                            "FAKE_CODEX_CATALOGUE_AUDIT".parse().unwrap(),
                            ConfigValue::Text(
                                root.join("must-not-launch").to_str().unwrap().into(),
                            ),
                        )]))
                        .unwrap(),
                        ..ProviderOverride::default()
                    },
                )
            })
            .collect(),
    )
    .unwrap()
}

fn start(
    root: &std::path::Path,
    overrides: &ProviderOverrides,
    gate: &Gate,
) -> (Owner, JoinHandle<()>) {
    let mut producer = Producer::new(
        Lookup::new([root.to_path_buf()]),
        HarnessEnv::new(root.to_path_buf()),
        root.to_path_buf(),
        Arc::new(NoMcpServers),
    );
    producer.preparation = Some(Arc::clone(&gate.0));
    Owner::start(
        ProviderRegistry::build(overrides, &[]).unwrap(),
        move |work, cancel, lease| {
            let producer = producer.clone();
            async move { producer.observe(work, cancel, lease).await }
        },
    )
}

fn ask(owner: &Owner, provider: &str) -> JoinHandle<Result<ProviderDiagnostic, OwnerError>> {
    let owner = owner.clone();
    let provider = provider.parse().unwrap();
    tokio::spawn(async move {
        owner
            .refresh(provider, CatalogueScope::Global, Refresh::Force)
            .await
    })
}

async fn entered(calls: &mut mpsc::Receiver<()>) {
    tokio::time::timeout(PREPARATION_START_TIMEOUT, calls.recv())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn actual_preparation_retains_per_provider_slots_after_timeout_supersession_and_removal() {
    let root = tempfile::tempdir().unwrap();
    let overrides = fixtures(root.path(), 1);
    let (gate, mut calls) = gate();
    let (owner, task) = start(root.path(), &overrides, &gate);
    let first = ask(&owner, "fixture-0");
    entered(&mut calls).await;
    // The timeout exists before the actual blocking closure enters its gate.
    tokio::time::pause();
    tokio::time::advance(REFRESH_TIMEOUT + DEADLINE_SLACK).await;
    tokio::time::resume();
    let timed_out = first.await.unwrap().unwrap();
    assert!(matches!(
        timed_out.entry.state,
        ProviderState::Error {
            failure: DiscoveryFailure::TimedOut { .. }
        }
    ));
    let mut previous = None::<JoinHandle<Result<ProviderDiagnostic, OwnerError>>>;
    for index in 1..PER_PROVIDER {
        let next = ask(&owner, "fixture-0");
        entered(&mut calls).await;
        if let Some(previous) = previous.take() {
            assert!(matches!(
                previous.await.unwrap(),
                Err(OwnerError::Superseded)
            ));
        }
        if index == 1 {
            next.abort();
            assert!(next.await.unwrap_err().is_cancelled());
        } else {
            previous = Some(next);
        }
    }
    // A removed/re-added identity must reuse permits held by old physical work.
    owner
        .replace(ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap())
        .await
        .unwrap();
    if let Some(previous) = previous {
        assert!(matches!(
            previous.await.unwrap(),
            Err(OwnerError::Superseded)
        ));
    }
    owner
        .replace(ProviderRegistry::build(&overrides, &[]).unwrap())
        .await
        .unwrap();
    let queued = ask(&owner, "fixture-0");
    tokio::task::yield_now().await;
    let _barrier = owner
        .cached("fixture-0".parse().unwrap(), CatalogueScope::Global)
        .await
        .unwrap();
    assert!(calls.try_recv().is_err());
    let stopping_owner = owner.clone();
    let stopping = tokio::spawn(async move { stopping_owner.stop().await });
    assert!(matches!(queued.await.unwrap(), Err(OwnerError::Stopped)));
    assert!(!stopping.is_finished());
    assert!(!task.is_finished());
    gate.0.release();
    tokio::time::timeout(STOP_TIMEOUT, stopping)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    task.await.unwrap();
    assert!(!root.path().join("must-not-launch").exists());
    assert!(calls.try_recv().is_err());
}

#[tokio::test]
async fn actual_preparation_global_capacity_and_stop_include_all_retained_workers() {
    vessel_testkit::within(GLOBAL_TEST_TIMEOUT, async {
        let root = tempfile::tempdir().unwrap();
        let (gate, mut calls) = gate();
        let (owner, task) = start(
            root.path(),
            &fixtures(root.path(), SATURATED_PROVIDERS.saturating_add(1)),
            &gate,
        );
        for provider in 0..SATURATED_PROVIDERS {
            let name = format!("fixture-{provider}");
            let mut previous = None::<JoinHandle<Result<ProviderDiagnostic, OwnerError>>>;
            for _ in 0..PER_PROVIDER {
                let next = ask(&owner, &name);
                entered(&mut calls).await;
                if let Some(previous) = previous.take() {
                    assert!(matches!(
                        previous.await.unwrap(),
                        Err(OwnerError::Superseded)
                    ));
                }
                previous = Some(next);
            }
            // Cancel the caller, but the actual fourth preparation stays owned.
            previous.as_ref().unwrap().abort();
        }
        let extra = format!("fixture-{SATURATED_PROVIDERS}");
        let queued = ask(&owner, &extra);
        tokio::task::yield_now().await;
        let _barrier = owner
            .cached(extra.parse().unwrap(), CatalogueScope::Global)
            .await
            .unwrap();
        assert!(calls.try_recv().is_err());
        let stopping_owner = owner.clone();
        let stopping = tokio::spawn(async move { stopping_owner.stop().await });
        assert!(matches!(queued.await.unwrap(), Err(OwnerError::Stopped)));
        assert!(!stopping.is_finished());
        assert!(!task.is_finished());
        gate.0.release();
        stopping.await.unwrap().unwrap();
        task.await.unwrap();
        assert!(!root.path().join("must-not-launch").exists());
        assert!(calls.try_recv().is_err());
    })
    .await
    .unwrap();
}
