use super::*;
use vessel_proto::host_registry::{
    HostEnabled, HostLabelChange, HostMetadata, IdentityChange, MetadataKey, MetadataValue,
};
use vessel_proto::{DeviceId, NodeId};

#[tokio::test]
async fn shared_inventory_projects_all_sources_and_retains_disabled_records() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let registry = HostRegistry::start(data, stop.clone(), &tasks)
        .await
        .unwrap();
    let destination = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let mut ids = Vec::new();
    for source in [
        HostSource::Owner,
        HostSource::Device {
            device: DeviceId::random(),
        },
    ] {
        let record = registry
            .add(
                source,
                AddHostRegistration {
                    destination,
                    label: Some("public worker".parse().unwrap()),
                    metadata: std::collections::BTreeMap::from([(
                        "role".parse::<MetadataKey>().unwrap(),
                        "build".parse::<MetadataValue>().unwrap(),
                    )])
                    .try_into()
                    .unwrap(),
                },
                TrustedDestinations(vec![destination]),
            )
            .await
            .unwrap();
        let disabled = registry
            .update(
                source,
                UpdateHostRegistration {
                    registration_id: record.registration_id,
                    expected_revision: record.revision,
                    label: HostLabelChange::Keep,
                    metadata: None,
                    enabled: Some(HostEnabled::Disabled),
                    destination: None,
                    identity_change: IdentityChange::KeepPinned,
                },
                TrustedDestinations(Vec::new()),
            )
            .await
            .unwrap();
        assert_eq!(disabled.enabled, HostEnabled::Disabled);
        ids.push(record.registration_id);
        assert_eq!(registry.list(source).await.unwrap().len(), 1);
    }
    let shared = registry.shared().await.unwrap();
    assert_eq!(shared.entries().len(), 2);
    assert_eq!(
        shared
            .entries()
            .iter()
            .map(|row| row.registration_id)
            .collect::<Vec<_>>(),
        ids
    );
    for row in shared.entries() {
        assert_eq!(row.destination, destination);
        assert_eq!(row.label.as_ref().unwrap().as_str(), "public worker");
        assert_eq!(
            row.annotations.entries().values().next().unwrap().as_str(),
            "build"
        );
        assert_eq!(row.destination_name, None);
    }
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}
#[tokio::test]
async fn source_identity_revision_and_persistence_are_independent_of_transport_authority() {
    let dir = tempfile::tempdir().unwrap();
    let data = DataDir::new(dir.path());
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let registry = HostRegistry::start(data.clone(), stop.clone(), &tasks)
        .await
        .unwrap();
    let destination = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let trusted = TrustedDestinations(vec![destination]);
    let input = AddHostRegistration {
        destination,
        label: Some("worker".parse().unwrap()),
        metadata: HostMetadata::default(),
    };
    assert!(matches!(
        registry
            .add(
                HostSource::Owner,
                input.clone(),
                TrustedDestinations(Vec::new())
            )
            .await,
        Err(RegistryError::Untrusted)
    ));
    let entry = registry
        .add(HostSource::Owner, input, trusted.clone())
        .await
        .unwrap();
    let other = HostSource::Device {
        device: DeviceId::random(),
    };
    assert!(registry.list(other).await.unwrap().is_empty());
    let change = UpdateHostRegistration {
        registration_id: entry.registration_id,
        expected_revision: entry.revision,
        label: HostLabelChange::Clear,
        metadata: None,
        enabled: Some(HostEnabled::Disabled),
        destination: None,
        identity_change: IdentityChange::KeepPinned,
    };
    assert!(matches!(
        registry
            .update(other, change.clone(), trusted.clone())
            .await,
        Err(RegistryError::Forbidden)
    ));
    let disabled = registry
        .update(
            HostSource::Owner,
            change.clone(),
            TrustedDestinations(Vec::new()),
        )
        .await
        .unwrap();
    assert_eq!(disabled.enabled, HostEnabled::Disabled);
    assert!(matches!(
        registry
            .update(HostSource::Owner, change, trusted.clone())
            .await,
        Err(RegistryError::Stale)
    ));
    let replaced = identity_rebind(&registry, &data, &entry, &disabled).await;
    registry
        .remove(
            HostSource::Owner,
            RemoveHostRegistration {
                registration_id: entry.registration_id,
                expected_revision: replaced.revision,
            },
        )
        .await
        .unwrap();
    assert!(data.read_host_registry().unwrap().entries().is_empty());
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}

#[tokio::test]
async fn queued_commit_finishes_after_reply_receiver_is_dropped() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let registry = HostRegistry::start(data.clone(), stop.clone(), &tasks)
        .await
        .unwrap();
    let destination = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let (reply, response) = oneshot::channel();
    drop(response);
    registry
        .commands
        .send(Command::Change {
            source: HostSource::Owner,
            change: Change::Add(
                AddHostRegistration {
                    destination,
                    label: None,
                    metadata: HostMetadata::default(),
                },
                TrustedDestinations(vec![destination]),
            ),
            reply,
        })
        .await
        .unwrap();
    // FIFO list is a barrier after the accepted mutation, not a timing guess.
    let records = registry.list(HostSource::Owner).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(data.read_host_registry().unwrap().entries(), records);
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}

async fn identity_rebind(
    registry: &HostRegistry,
    data: &DataDir,
    entry: &HostRegistration,
    disabled: &HostRegistration,
) -> HostRegistration {
    let new = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let change = UpdateHostRegistration {
        registration_id: entry.registration_id,
        expected_revision: disabled.revision,
        label: HostLabelChange::Keep,
        metadata: None,
        enabled: None,
        destination: Some(new),
        identity_change: IdentityChange::KeepPinned,
    };
    assert!(matches!(
        registry
            .update(
                HostSource::Owner,
                change.clone(),
                TrustedDestinations(vec![new])
            )
            .await,
        Err(RegistryError::IdentityChanged)
    ));
    let replaced = registry
        .update(
            HostSource::Owner,
            UpdateHostRegistration {
                identity_change: IdentityChange::Replace,
                ..change
            },
            TrustedDestinations(vec![new]),
        )
        .await
        .unwrap();
    assert_eq!(
        data.read_host_registry().unwrap().entries(),
        std::slice::from_ref(&replaced)
    );
    replaced
}
