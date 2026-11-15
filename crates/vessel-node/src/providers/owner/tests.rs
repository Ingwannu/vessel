//! Controlled producer channels test real ownership, cancellation and deadlines.
//! They never construct or launch an agent command or inspect a real home.

use super::*;
use vessel_proto::providers::{
    Catalogue, CatalogueDirectory, Enabled, ModeCatalogue, Models, ProviderOverride,
    ProviderOverrides, ProviderState, UnavailableReason,
};

type Invocation = (Work, oneshot::Sender<Observation>);

fn registry() -> ProviderRegistry {
    ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap()
}
fn id() -> ProviderId {
    "claude".parse().unwrap()
}
fn observed() -> Observation {
    Observation::Ready {
        catalogue: Catalogue {
            models: Models::default(),
            modes: ModeCatalogue::new(vec![], None).unwrap(),
        },
        version: None,
    }
}

fn controlled() -> (
    Owner,
    tokio::task::JoinHandle<()>,
    mpsc::Receiver<Invocation>,
) {
    let (calls, receiver) = mpsc::channel(MAX_JOBS);
    let (owner, task) = Owner::start(registry(), move |work, _cancel, _lease| {
        let calls = calls.clone();
        async move {
            let (finish, answer) = oneshot::channel();
            if calls.send((work, finish)).await.is_err() {
                return Observation::Failed(DiscoveryFailure::Stopped);
            }
            answer
                .await
                .unwrap_or(Observation::Failed(DiscoveryFailure::Stopped))
        }
    });
    (owner, task, receiver)
}

#[tokio::test(start_paused = true)]
async fn passive_miss_runs_no_producer_and_warmups_coalesce_across_host_targets() {
    let (owner, task, mut calls) = controlled();
    let mut changes = owner.changes();
    let result = owner.cached(id(), CatalogueScope::Global).await.unwrap();
    assert_eq!(
        result.entry.state,
        ProviderState::Unavailable {
            reason: UnavailableReason::NotChecked
        }
    );
    assert!(matches!(
        calls.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    let (first, first_answer) = oneshot::channel();
    owner
        .mail
        .send(Mail::Refresh {
            provider: id(),
            scope: CatalogueScope::Global,
            refresh: Refresh::Cached,
            reply: first,
        })
        .await
        .unwrap();
    let (_work, finish) = calls.recv().await.unwrap();
    let scope = CatalogueScope::Workspace(
        CatalogueDirectory::new(std::env::temp_dir().join("fake-owner-workspace")).unwrap(),
    );
    let (second, second_answer) = oneshot::channel();
    owner
        .mail
        .send(Mail::Refresh {
            provider: id(),
            scope: scope.clone(),
            refresh: Refresh::Cached,
            reply: second,
        })
        .await
        .unwrap();
    let _barrier = owner.cached(id(), scope).await.unwrap();
    assert!(matches!(
        calls.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    finish.send(observed()).unwrap();
    let first = first_answer.await.unwrap().unwrap();
    assert_eq!(second_answer.await.unwrap().unwrap(), first);
    assert_eq!(
        changes.recv().await.unwrap(),
        Change::Observed {
            scope: CatalogueScope::Global,
            diagnostic: Arc::new(first)
        }
    );
    owner.stop().await.unwrap();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn changed_definition_cancels_a_producer_and_settles_its_waiters() {
    let (owner, task, mut calls) = controlled();
    let mut changes = owner.changes();
    let caller = owner.clone();
    let waiting = tokio::spawn(async move {
        caller
            .refresh(id(), CatalogueScope::Global, Refresh::Force)
            .await
    });
    let (_work, finish) = calls.recv().await.unwrap();
    let overrides = ProviderOverrides::new(BTreeMap::from([(
        id(),
        ProviderOverride {
            enabled: Some(Enabled::Off),
            ..ProviderOverride::default()
        },
    )]))
    .unwrap();
    owner
        .replace(ProviderRegistry::build(&overrides, &[]).unwrap())
        .await
        .unwrap();
    assert_eq!(changes.recv().await.unwrap(), Change::Registry);
    assert!(matches!(
        waiting.await.unwrap(),
        Err(OwnerError::Superseded)
    ));
    assert_eq!(
        owner
            .cached(id(), CatalogueScope::Global)
            .await
            .unwrap()
            .entry
            .state,
        ProviderState::Unavailable {
            reason: UnavailableReason::Disabled
        }
    );
    // Cancellation drops the receiver; even an uncooperative late producer
    // cannot put its old result back into the disabled generation.
    let _late = finish.send(observed());
    owner.stop().await.unwrap();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn snapshots_are_passive_and_registry_notifications_cover_cached_views() {
    let (owner, task, mut calls) = controlled();
    let initial = owner.snapshot(CatalogueScope::Global).await.unwrap();
    assert_eq!(initial.len(), registry().definitions().len());
    assert!(calls.try_recv().is_err());
    let mut changes = owner.changes();
    let overrides = ProviderOverrides::new(BTreeMap::from([(
        id(),
        ProviderOverride {
            enabled: Some(Enabled::Off),
            ..ProviderOverride::default()
        },
    )]))
    .unwrap();
    owner
        .replace(ProviderRegistry::build(&overrides, &[]).unwrap())
        .await
        .unwrap();
    assert_eq!(changes.recv().await.unwrap(), Change::Registry);
    let current = owner.snapshot(CatalogueScope::Global).await.unwrap();
    assert_eq!(
        current
            .iter()
            .find(|entry| entry.provider == id())
            .unwrap()
            .enabled,
        Enabled::Off
    );
    assert!(calls.try_recv().is_err());
    owner.stop().await.unwrap();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn every_discovery_has_a_finite_deadline_and_shutdown_settles_pending_calls() {
    let (owner, task, mut calls) = controlled();
    let caller = owner.clone();
    let waiting = tokio::spawn(async move {
        caller
            .refresh(id(), CatalogueScope::Global, Refresh::Force)
            .await
    });
    let (_work, unfinished) = calls.recv().await.unwrap();
    tokio::time::advance(REFRESH_TIMEOUT).await;
    let result = waiting.await.unwrap().unwrap();
    assert_eq!(
        result.entry.state,
        ProviderState::Error {
            failure: DiscoveryFailure::TimedOut {
                phase: DiscoveryPhase::Catalogue
            }
        }
    );
    assert!(unfinished.send(observed()).is_err());
    let caller = owner.clone();
    let waiting = tokio::spawn(async move {
        caller
            .refresh(id(), CatalogueScope::Global, Refresh::Force)
            .await
    });
    let (_work, unfinished) = calls.recv().await.unwrap();
    owner.stop().await.unwrap();
    assert!(matches!(waiting.await.unwrap(), Err(OwnerError::Stopped)));
    task.await.unwrap();
    assert!(unfinished.send(observed()).is_err());
    assert!(matches!(
        owner.cached(id(), CatalogueScope::Global).await,
        Err(OwnerError::Stopped)
    ));
}
