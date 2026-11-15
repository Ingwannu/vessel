//! Exercise the same manager projection used by handlers, with no executable.

use super::*;
use crate::providers::{cache::Observation, configuration::Configuration};
use std::sync::atomic::{AtomicUsize, Ordering};
use vessel_proto::providers::{ProviderState, UnavailableReason};

#[tokio::test]
async fn cached_manager_and_trusted_reads_never_discover_and_handles_end_with_the_owner() {
    let root = tempfile::tempdir().unwrap();
    let directory = DataDir::new(root.path());
    let probes = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&probes);
    let started = Configuration::open(directory.clone(), vec![], move |_, _, _| {
        let _ = observed.fetch_add(1, Ordering::SeqCst);
        async { Observation::Unavailable(UnavailableReason::NotInstalled) }
    })
    .await
    .unwrap();
    let manager = Providers {
        cache: started.cache,
        configuration: started.configuration,
        // This standalone owner test stops configuration explicitly below.
        _lifetime: Arc::new(CancellationToken::new().drop_guard()),
    };
    let registrations = manager.registrations();
    let id: ProviderId = "codex".parse().unwrap();
    let scope = CatalogueScope::Global;
    for _ in 0..2 {
        assert!(matches!(
            manager
                .diagnostic(id.clone(), scope.clone(), DiagnosticRead::Cached)
                .await
                .unwrap()
                .entry
                .state,
            ProviderState::Unavailable {
                reason: UnavailableReason::NotChecked
            }
        ));
        let _ = registrations
            .cached(id.clone(), scope.clone())
            .await
            .unwrap();
        let _ = manager
            .snapshot(scope.clone(), DiagnosticRead::Cached)
            .await
            .unwrap();
    }
    assert_eq!(probes.load(Ordering::SeqCst), 0);
    let _ = manager
        .diagnostic(id.clone(), scope.clone(), DiagnosticRead::Discover)
        .await
        .unwrap();
    assert_eq!(probes.load(Ordering::SeqCst), 1);
    let _ = manager
        .diagnostic(id.clone(), scope.clone(), DiagnosticRead::Cached)
        .await
        .unwrap();
    assert_eq!(probes.load(Ordering::SeqCst), 1);
    let _ = manager
        .diagnostic(id.clone(), scope.clone(), DiagnosticRead::Refresh)
        .await
        .unwrap();
    assert_eq!(probes.load(Ordering::SeqCst), 2);
    let disabled = manager.set_enabled(id.clone(), Enabled::Off).await.unwrap();
    assert_eq!(disabled.enabled, Enabled::Off);
    assert!(matches!(
        disabled.state,
        ProviderState::Unavailable {
            reason: UnavailableReason::Disabled
        }
    ));
    assert_eq!(
        directory
            .read_provider_overrides()
            .unwrap()
            .unwrap()
            .entries()[&id]
            .enabled,
        Some(Enabled::Off)
    );
    assert_eq!(probes.load(Ordering::SeqCst), 2);
    let mut changes = registrations.changes();
    manager.configuration.stop().await.unwrap();
    assert_eq!(
        changes.recv().await.unwrap(),
        super::super::owner::Change::Stopped
    );
    started.task.await.unwrap();
    started.catalogue_task.await.unwrap();
    assert!(matches!(
        registrations.cached(id, scope).await,
        Err(OwnerError::Stopped)
    ));
    assert!(matches!(
        registrations.replace_plugins(vec![]).await,
        Err(ConfigurationFailure::Stopped)
    ));
    assert!(matches!(
        registrations.changes().recv().await,
        Err(tokio::sync::broadcast::error::RecvError::Closed)
    ));
}

#[tokio::test]
async fn prepared_product_owners_stop_under_the_node_task_tracker_with_retained_handles() {
    for close_by_drop in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let directory = DataDir::new(root.path().join("data"));
        let home = root.path().join("isolated-home");
        let mut config = Config::new(
            directory.clone(),
            vessel_harness::HarnessEnv::from_vars(home, |_| None),
        );
        config.harness_lookup = vessel_harness::Lookup::new([root.path().join("no-tools")]);
        let prepared = PreparedProviders::read(directory).unwrap();
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let manager = prepared.start(
            &config,
            root.path().join("probe"),
            Arc::new(crate::profiles::launch::NoMcpServers),
            &tasks,
            &stop,
        );
        let registrations = manager.registrations();
        let id: ProviderId = "claude".parse().unwrap();
        assert!(matches!(
            registrations
                .cached(id.clone(), CatalogueScope::Global)
                .await
                .unwrap()
                .entry
                .state,
            ProviderState::Unavailable {
                reason: UnavailableReason::NotChecked
            }
        ));
        if close_by_drop {
            let survivor = manager.clone();
            drop(manager);
            assert!(
                registrations
                    .cached(id.clone(), CatalogueScope::Global)
                    .await
                    .is_ok()
            );
            drop(survivor);
            assert!(!stop.is_cancelled());
        } else {
            stop.cancel();
            drop(manager);
        }
        let _first_close = tasks.close();
        tokio::time::timeout(std::time::Duration::from_secs(2), tasks.wait())
            .await
            .unwrap();
        assert!(matches!(
            registrations.cached(id, CatalogueScope::Global).await,
            Err(OwnerError::Stopped)
        ));
        assert!(matches!(
            registrations.replace_plugins(vec![]).await,
            Err(ConfigurationFailure::Stopped)
        ));
    }
}
