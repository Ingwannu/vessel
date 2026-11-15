//! Real synthetic files plus live owner tasks prove ordering and cancellation.

use super::*;
use std::collections::BTreeMap;
use vessel_harness::providers::{ConfigurationRequirement, PluginRevision};
use vessel_proto::providers::{
    CatalogueScope, LaunchSettings, ProviderKind, ProviderOverride, UnavailableReason,
};

async fn open(directory: &DataDir) -> Started {
    Configuration::open(directory.clone(), vec![], |_, _, _| async {
        Observation::Unavailable(UnavailableReason::NotInstalled)
    })
    .await
    .unwrap()
}

async fn stop(started: Started) {
    started.configuration.stop().await.unwrap();
    started.task.await.unwrap();
    started.catalogue_task.await.unwrap();
}

#[tokio::test]
async fn concurrent_enabled_edits_read_the_latest_generation_and_survive_restart() {
    let root = tempfile::tempdir().unwrap();
    let directory = DataDir::new(root.path());
    let started = open(&directory).await;
    let first = started
        .configuration
        .set_enabled("claude".parse().unwrap(), Enabled::Off);
    let second = started
        .configuration
        .set_enabled("codex".parse().unwrap(), Enabled::Off);
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();
    let saved = directory.read_provider_overrides().unwrap().unwrap();
    for provider in ["claude", "codex"] {
        assert_eq!(
            saved.entries()[&provider.parse().unwrap()].enabled,
            Some(Enabled::Off)
        );
        assert_eq!(
            started
                .cache
                .cached(provider.parse().unwrap(), CatalogueScope::Global)
                .await
                .unwrap()
                .entry
                .enabled,
            Enabled::Off
        );
    }
    stop(started).await;
    let reopened = open(&directory).await;
    assert_eq!(
        reopened
            .cache
            .cached("codex".parse().unwrap(), CatalogueScope::Global)
            .await
            .unwrap()
            .entry
            .enabled,
        Enabled::Off
    );
    stop(reopened).await;
}

#[tokio::test]
async fn an_admitted_edit_finishes_even_when_its_waiter_is_dropped() {
    let root = tempfile::tempdir().unwrap();
    let directory = DataDir::new(root.path());
    let started = open(&directory).await;
    let (reply, response) = oneshot::channel();
    started
        .configuration
        .edits
        .send(Edit::Enabled {
            provider: "omp".parse().unwrap(),
            enabled: Enabled::On,
            reply,
        })
        .await
        .unwrap();
    drop(response);
    // FIFO edit completion is a barrier, not a timer or scheduler guess.
    started.configuration.plugins(vec![]).await.unwrap();
    assert_eq!(
        directory
            .read_provider_overrides()
            .unwrap()
            .unwrap()
            .entries()[&"omp".parse().unwrap()]
            .enabled,
        Some(Enabled::On)
    );
    assert_eq!(
        started
            .cache
            .cached("omp".parse().unwrap(), CatalogueScope::Global)
            .await
            .unwrap()
            .entry
            .enabled,
        Enabled::On
    );
    stop(started).await;
}

#[tokio::test]
async fn invalid_generations_and_failed_writes_preserve_the_applied_state() {
    let root = tempfile::tempdir().unwrap();
    let directory = DataDir::new(root.path());
    let started = open(&directory).await;
    started
        .configuration
        .set_enabled("claude".parse().unwrap(), Enabled::Off)
        .await
        .unwrap();
    let before = std::fs::read(directory.providers_path()).unwrap();
    let invalid = ProviderOverrides::new(BTreeMap::from([(
        "custom".parse().unwrap(),
        ProviderOverride {
            extends: Some(ProviderKind::Acp),
            ..ProviderOverride::default()
        },
    )]))
    .unwrap();
    assert!(matches!(
        started.configuration.replace(invalid).await,
        Err(ConfigurationFailure::Invalid(_))
    ));
    assert_eq!(std::fs::read(directory.providers_path()).unwrap(), before);
    assert!(matches!(
        started
            .configuration
            .set_enabled("unknown".parse().unwrap(), Enabled::On)
            .await,
        Err(ConfigurationFailure::Unknown)
    ));
    // A nonregular target is refused before it can replace durable state.
    std::fs::remove_file(directory.providers_path()).unwrap();
    std::fs::create_dir(directory.providers_path()).unwrap();
    assert!(matches!(
        started
            .configuration
            .set_enabled("claude".parse().unwrap(), Enabled::On)
            .await,
        Err(ConfigurationFailure::Store(_))
    ));
    assert!(directory.providers_path().is_dir());
    assert_eq!(
        started
            .cache
            .cached("claude".parse().unwrap(), CatalogueScope::Global)
            .await
            .unwrap()
            .entry
            .enabled,
        Enabled::Off
    );
    stop(started).await;
}

#[tokio::test]
async fn plugin_generations_validate_before_retiring_the_previous_registration() {
    let root = tempfile::tempdir().unwrap();
    let directory = DataDir::new(root.path());
    let started = open(&directory).await;
    let plugin = PluginDefinition {
        id: "local-plugin".parse().unwrap(),
        label: "Plugin".parse().unwrap(),
        configuration: ConfigurationRequirement::Optional,
        revision: PluginRevision::new(1),
        launch: LaunchSettings::default(),
    };
    started
        .configuration
        .plugins(vec![plugin.clone()])
        .await
        .unwrap();
    assert!(
        started
            .cache
            .cached(plugin.id.clone(), CatalogueScope::Global)
            .await
            .is_ok()
    );
    assert!(matches!(
        started
            .configuration
            .plugins(vec![plugin.clone(), plugin.clone()])
            .await,
        Err(ConfigurationFailure::Invalid(_))
    ));
    assert!(
        started
            .cache
            .cached(plugin.id, CatalogueScope::Global)
            .await
            .is_ok()
    );
    assert!(!directory.providers_path().exists());
    stop(started).await;
}
