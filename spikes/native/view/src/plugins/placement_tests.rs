use super::*;
use crate::plugins::testing::catalog;
use vessel_proto::plugins::{
    ClientDocument, Contribution, ContributionId, ContributionKind, PluginAddress,
    RuntimeGeneration, Surface,
};
use vessel_proto::{NodeId, SessionId};

use crate::plugins::{HostSupport, Installation, SnapshotApplied};

fn entry(
    context: PanelContext,
    locations: Vec<PanelLocation>,
) -> (Installation, ContributionAddress) {
    let installation = PluginAddress {
        node: NodeId::random(),
        plugin: "panels".parse().expect("plugin"),
        generation: RuntimeGeneration::random(),
    };
    let id: ContributionId = "details".parse().expect("id");
    let graph = ClientDocument::new(
        vec![],
        vec![Contribution {
            id: id.clone(),
            title: "Details".parse().expect("title"),
            content: ContributionContent::Panel {
                context,
                locations,
                surface: Surface { controls: vec![] },
            },
        }],
        vec![],
    )
    .expect("graph");
    let address = ContributionAddress {
        installation: installation.clone(),
        kind: ContributionKind::Panel,
        contribution: id,
    };
    (Installation::from_document(installation, graph), address)
}

fn publish(registry: &mut Registry, installation: Installation) {
    let ticket = registry
        .begin(installation.address().node, HostSupport::Supported)
        .expect("read");
    assert_eq!(
        registry.apply(ticket, catalog(ticket.node(), vec![installation])),
        Ok(SnapshotApplied::Applied)
    );
}

#[test]
fn declaration_locations_and_contexts_are_live_placement_facts() {
    let (installation, address) = entry(PanelContext::Workspace, vec![PanelLocation::Explorer]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let context = PluginPanelContext::Workspace {};
    let policy = registry
        .resolve_panel(&address, &context)
        .expect("live policy");
    assert_eq!(policy.address(), &address);
    assert_eq!(policy.context(), &context);
    assert!(policy.supports(PanelLocation::Explorer));
    assert!(!policy.supports(PanelLocation::Workspace));
    assert!(matches!(
        registry.resolve_panel(
            &address,
            &PluginPanelContext::Agent {
                agent: SessionId::random()
            }
        ),
        Err(Problem::Context)
    ));
    assert!(matches!(
        registry.resolve_panel(
            &address,
            &PluginPanelContext::File {
                path: "notes.txt".parse().expect("path")
            }
        ),
        Err(Problem::Context)
    ));
}

#[test]
fn offline_and_replaced_generations_cannot_mint_placement_policies() {
    let (installation, address) = entry(PanelContext::Agent, vec![PanelLocation::Workspace]);
    let mut registry = Registry::default();
    publish(&mut registry, installation.clone());
    let context = PluginPanelContext::Agent {
        agent: SessionId::random(),
    };
    assert!(registry.resolve_panel(&address, &context).is_ok());
    registry.offline(address.installation.node);
    assert!(matches!(
        registry.resolve_panel(&address, &context),
        Err(Problem::Offline)
    ));
    assert_eq!(
        registry.installed(address.installation.node),
        Some([installation.clone()].as_slice())
    );
    let mut replacement = address.installation.clone();
    replacement.generation = RuntimeGeneration::random();
    publish(
        &mut registry,
        Installation::from_document(replacement, installation.document().clone()),
    );
    assert!(matches!(
        registry.resolve_panel(&address, &context),
        Err(Problem::Stale)
    ));
}

#[test]
fn kind_and_identity_are_checked_before_resolving_a_panel() {
    let (installation, mut address) =
        entry(PanelContext::Workspace, vec![PanelLocation::Workspace]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    address.kind = ContributionKind::FilePanel;
    assert!(matches!(
        registry.resolve_panel(&address, &PluginPanelContext::Workspace {}),
        Err(Problem::MissingContribution)
    ));
    address.kind = ContributionKind::Panel;
    address.contribution = "missing".parse().expect("id");
    assert!(matches!(
        registry.resolve_panel(&address, &PluginPanelContext::Workspace {}),
        Err(Problem::MissingContribution)
    ));
}

#[test]
fn changed_declarations_require_a_new_generation_before_publication() {
    let (installation, address) = entry(PanelContext::Workspace, vec![PanelLocation::Workspace]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let (different, _) = entry(PanelContext::Workspace, vec![PanelLocation::Explorer]);
    let invalid =
        Installation::from_document(address.installation.clone(), different.document().clone());
    let ticket = registry
        .begin(address.installation.node, HostSupport::Supported)
        .expect("read");
    assert_eq!(
        registry.apply(ticket, catalog(ticket.node(), vec![invalid])),
        Err(Problem::InvalidCatalog)
    );
    let context = PluginPanelContext::Workspace {};
    assert!(
        registry
            .resolve_panel(&address, &context)
            .expect("retained")
            .supports(PanelLocation::Workspace)
    );
}
