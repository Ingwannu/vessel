//! Adversarial catalog boundaries, using no process or node connection.

use super::*;
use crate::plugins::testing::catalog;
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
        registry.apply(ticket, catalog(ticket.node(), entries)),
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
        registry.apply(obsolete, catalog(obsolete.node(), vec![old])),
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
        registry.apply(ticket, catalog(ticket.node(), vec![safe, foreign])),
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
        registry.apply(ticket, catalog(ticket.node(), entries)),
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
        registry.apply(ticket, catalog(ticket.node(), vec![entry])),
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
        empty.apply(ticket, catalog(ticket.node(), entries)),
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
        current.apply(obsolete, catalog(node, vec![installation(node, "old")])),
        Ok(SnapshotApplied::Stale)
    );
    assert_eq!(
        current.apply(pending, catalog(node, vec![])),
        Ok(SnapshotApplied::Applied)
    );
}

#[test]
fn a_valid_catalog_for_another_node_and_a_newer_client_floor_cannot_publish() {
    use vessel_proto::{PROTOCOL_VERSION, ProtocolVersion, plugins::ClientInstallation};
    let node = NodeId::random();
    let mut registry = Registry::default();
    let safe = installation(node, "safe");
    let captured = safe.address().clone();
    read(&mut registry, node, vec![safe.clone()]);
    let foreign = NodeId::random();
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        registry.apply(
            ticket,
            catalog(foreign, vec![installation(foreign, "foreign")])
        ),
        Err(Problem::InvalidCatalog)
    );
    let next = PROTOCOL_VERSION
        .get()
        .checked_add(1)
        .expect("synthetic newer test peer");
    let incompatible = ClientInstallation::new(
        safe.address().clone(),
        ProtocolVersion::new(next),
        safe.document().clone(),
    )
    .expect("structurally valid newer installation");
    let ticket = registry.begin(node, HostSupport::Supported).expect("read");
    assert_eq!(
        registry.apply(
            ticket,
            ClientCatalog::new(node, vec![incompatible])
                .map_err(|_invalid| Problem::InvalidCatalog)
        ),
        Err(Problem::NewerClient)
    );
    assert_eq!(registry.installed(node).expect("retained").len(), 1);
    assert!(registry.resolve_action(&captured).is_ok());
}

fn large_document() -> ClientDocument {
    use vessel_proto::plugins::{ContributionContent, Control, ControlContent, Surface};
    const TEXT_BYTES: usize = 16 * 1024;
    const CONTROLS: usize = 60;
    let text: vessel_proto::plugins::ContentText =
        "x".repeat(TEXT_BYTES).parse().expect("bounded text");
    let controls = (0..CONTROLS)
        .map(|index| Control {
            id: format!("control-{index}").parse().expect("id"),
            content: ControlContent::Text { text: text.clone() },
        })
        .collect();
    ClientDocument::new(
        vec![],
        vec![Contribution {
            id: "screen".parse().expect("id"),
            title: "Screen".parse().expect("title"),
            content: ContributionContent::Screen {
                surface: Surface { controls },
            },
        }],
        vec![],
    )
    .expect("large bounded document")
}

fn large_catalog(node: NodeId, document: &ClientDocument) -> ClientCatalog {
    const INSTALLATIONS: usize = 4;
    let entries = (0..INSTALLATIONS)
        .map(|index| {
            Installation::from_document(
                PluginAddress {
                    node,
                    plugin: format!("plugin-{index}").parse().expect("id"),
                    generation: RuntimeGeneration::random(),
                },
                document.clone(),
            )
        })
        .collect();
    catalog(node, entries).expect("bounded canonical host catalog")
}

fn retained_bytes(registry: &Registry) -> usize {
    registry.hosts.values().map(|host| host.encoded_bytes).sum()
}

#[test]
fn fleet_byte_admission_retains_offline_facts_and_releases_only_explicit_withdrawal() {
    let document = large_document();
    let candidate_node = NodeId::random();
    let candidate = large_catalog(candidate_node, &document);
    let fill = MAX_RETAINED_CATALOG_BYTES.div_euclid(candidate.encoded_bytes());
    assert!(fill < MAX_HOSTS);
    let mut registry = Registry::default();
    let mut nodes = Vec::new();
    for _ in 0..fill {
        let node = NodeId::random();
        let ticket = registry.begin(node, HostSupport::Supported).expect("read");
        assert_eq!(
            registry.apply(ticket, Ok(large_catalog(node, &document))),
            Ok(SnapshotApplied::Applied)
        );
        nodes.push(node);
    }
    let first = *nodes.first().expect("filled host");
    let original = registry.installed(first).expect("snapshot").to_vec();
    let before = retained_bytes(&registry);
    registry.offline(first);
    assert_eq!(retained_bytes(&registry), before);
    let ticket = registry
        .begin(candidate_node, HostSupport::Supported)
        .expect("read");
    assert_eq!(
        registry.apply(ticket, Ok(candidate.clone())),
        Err(Problem::Capacity(CatalogBudget::RetainedBytes))
    );
    assert_eq!(
        retained_bytes(&registry),
        before,
        "refusal does not change retained bytes"
    );
    assert_eq!(
        registry.installed(first),
        Some(original.as_slice()),
        "no implicit eviction"
    );
    registry.remove(first);
    assert!(retained_bytes(&registry) < before);
    let ticket = registry
        .begin(candidate_node, HostSupport::Supported)
        .expect("retry");
    assert_eq!(
        registry.apply(ticket, Ok(candidate)),
        Ok(SnapshotApplied::Applied)
    );
    assert!(retained_bytes(&registry) <= MAX_RETAINED_CATALOG_BYTES);
}
