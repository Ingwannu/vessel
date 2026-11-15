//! Adversarial catalog boundaries, using no process or node connection.

use super::*;
use vessel_proto::plugins::RuntimeGeneration;

fn installation(node: NodeId, id: &str) -> Installation {
    Installation::new(
        PluginAddress {
            node,
            plugin: id.parse().expect("fixture id"),
            generation: RuntimeGeneration::random(),
        },
        vec![],
    )
    .expect("fixture installation")
}

fn read(registry: &mut Registry, node: NodeId, entries: Vec<Installation>) {
    let ticket = registry
        .begin(node, HostSupport::Supported)
        .expect("start read");
    assert_eq!(
        registry.apply(ticket, Ok(entries)),
        Ok(SnapshotApplied::Applied)
    );
}

#[test]
fn same_plugin_name_on_another_host_never_authorizes_an_action() {
    let mut registry = Registry::default();
    let first = NodeId::random();
    let second = NodeId::random();
    let entry = installation(first, "same");
    let captured = entry.address().clone();
    read(&mut registry, first, vec![entry]);
    read(&mut registry, second, vec![installation(second, "same")]);
    registry.offline(first);
    assert_eq!(registry.resolve_action(&captured), Err(Problem::Offline));
    assert_eq!(
        registry.installed(first).map(<[Installation]>::len),
        Some(1)
    );
    registry.remove(first);
    assert_eq!(registry.resolve_action(&captured), Err(Problem::Stale));
}

#[test]
fn replacement_fences_captured_actions_and_old_replies_cannot_restore_them() {
    let mut registry = Registry::default();
    let node = NodeId::random();
    let old = installation(node, "example");
    let captured = old.address().clone();
    read(&mut registry, node, vec![old.clone()]);
    let obsolete = registry.begin(node, HostSupport::Supported).expect("read");
    registry.offline(node);
    read(&mut registry, node, vec![installation(node, "example")]);
    assert_eq!(
        registry.apply(obsolete, Ok(vec![old])),
        Ok(SnapshotApplied::Stale)
    );
    assert_eq!(registry.resolve_action(&captured), Err(Problem::Stale));
}

#[test]
fn malformed_replacement_is_atomic_and_authoritative_removal_withdraws() {
    let mut registry = Registry::default();
    let node = NodeId::random();
    let safe = installation(node, "safe");
    let captured = safe.address().clone();
    read(&mut registry, node, vec![safe.clone()]);
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    let foreign = installation(NodeId::random(), "foreign");
    assert_eq!(
        registry.apply(ticket, Ok(vec![safe, foreign])),
        Err(Problem::InvalidCatalog)
    );
    assert!(registry.resolve_action(&captured).is_ok());
    assert_eq!(registry.installed(node).map(<[Installation]>::len), Some(1));
    assert_eq!(registry.failure(node), Some(Problem::InvalidCatalog));
    read(&mut registry, node, vec![]);
    assert_eq!(registry.resolve_action(&captured), Err(Problem::Stale));
}

#[test]
fn duplicate_installation_ids_refuse_the_whole_catalog_and_refreshes_serialize() {
    let mut registry = Registry::default();
    let node = NodeId::random();
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        registry.begin(node, HostSupport::Supported),
        Err(Problem::Busy)
    );
    let entries = vec![installation(node, "same"), installation(node, "same")];
    assert_eq!(
        registry.apply(ticket, Ok(entries)),
        Err(Problem::InvalidCatalog)
    );
    assert_eq!(registry.installed(node), Some([].as_slice()));
}

#[test]
fn unsupported_capability_withdraws_cached_contributions_and_late_read() {
    let mut registry = Registry::default();
    let node = NodeId::random();
    let entry = installation(node, "example");
    read(&mut registry, node, vec![entry.clone()]);
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        registry.begin(node, HostSupport::Unsupported),
        Err(Problem::Unsupported)
    );
    assert_eq!(registry.installed(node), None);
    assert_eq!(
        registry.apply(ticket, Ok(vec![entry])),
        Ok(SnapshotApplied::Stale)
    );
}

#[test]
fn host_and_installation_budgets_refuse_without_evicting_valid_data() {
    let mut registry = Registry::default();
    for _ in 0..MAX_HOSTS {
        let node = NodeId::random();
        read(&mut registry, node, vec![]);
    }
    assert_eq!(
        registry.begin(NodeId::random(), HostSupport::Supported),
        Err(Problem::Capacity(CatalogBudget::Hosts))
    );
    let node = NodeId::random();
    let mut empty = Registry::default();
    let ticket = empty.begin(node, HostSupport::Supported).expect("read");
    let entries = (0..MAX_INSTALLATIONS.saturating_add(1))
        .map(|number| installation(node, &format!("plugin-{number}")))
        .collect();
    assert_eq!(
        empty.apply(ticket, Ok(entries)),
        Err(Problem::Capacity(CatalogBudget::Installations))
    );
    assert_eq!(empty.installed(node), Some([].as_slice()));
}

#[test]
fn an_offline_read_error_never_reauthorizes_a_retained_action() {
    let mut registry = Registry::default();
    let node = NodeId::random();
    let entry = installation(node, "example");
    let captured = entry.address().clone();
    read(&mut registry, node, vec![entry]);
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        registry.apply(ticket, Err(Problem::Offline)),
        Err(Problem::Offline)
    );
    assert_eq!(registry.resolve_action(&captured), Err(Problem::Offline));
    assert_eq!(registry.installed(node).map(<[Installation]>::len), Some(1));
}

#[test]
fn replies_from_a_replaced_registry_cannot_collide_with_its_restarted_ticket_counter() {
    let node = NodeId::random();
    let mut old = Registry::default();
    let obsolete = old.begin(node, HostSupport::Supported).expect("read");
    let mut current = Registry::default();
    let pending = current.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        current.apply(obsolete, Ok(vec![installation(node, "old")])),
        Ok(SnapshotApplied::Stale)
    );
    assert_eq!(
        current.apply(pending, Ok(vec![])),
        Ok(SnapshotApplied::Applied)
    );
}
