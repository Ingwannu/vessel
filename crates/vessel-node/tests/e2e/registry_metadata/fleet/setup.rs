//! Retain the actual Node before observing inherited relay/fleet setup steps.
use super::super::lifetime::Resources;
use crate::{
    agents::vessel_mcp,
    fleet::{fleet_config, write},
    secure::{Relay, TOKEN},
    support::{Fixture, step},
};
use std::{path::Path, sync::Arc};
use vessel_node::{
    Node,
    fleet::{self, FleetFiles, FleetInit},
};
use vessel_proto::{HarnessKind, Scope, registry::WorkspaceDescriptor};

pub(super) type Member = crate::across::Member<Arc<Node>, Arc<Fixture>>;

async fn registered(node: &Node) {
    for (relay, mut state) in node.relay_links() {
        let observed = step(
            format!("registry retained node registering with {relay}"),
            state.wait_for(|state| *state == vessel_relay::LinkState::Registered),
        )
        .await
        .unwrap();
        drop(observed);
    }
}

pub(super) async fn home(resources: &mut Resources, relay: &Relay, sources: &Path) -> Member {
    let agents = sources.join("AGENTS.md");
    write(&agents, "fleet instructions\n");
    std::fs::create_dir_all(sources.join("skills")).unwrap();
    let fixture = Arc::new(Fixture::new());
    let _toml = fleet::init(
        &FleetFiles::new(&fixture.data_dir()),
        &FleetInit {
            instructions: Some(agents),
            skills: Some(sources.join("skills")),
            skip: Vec::new(),
        },
    )
    .unwrap();
    let config_fixture = Arc::clone(&fixture);
    let relay_url = relay.url.clone();
    let node = resources
        .node(Arc::clone(&fixture), move |config| {
            config.name = Some("home".to_owned());
            config.relays = vec![relay_url];
            config.relay_token_file = Some(token_file(&config_fixture));
            config.fleet = fleet_config(
                HarnessKind::ClaudeCode,
                config_fixture.cwd().join("home-claude"),
            );
            config.vessel_mcp = Some(vessel_mcp(&config_fixture));
        })
        .await;
    registered(&node).await;
    let owner = step(
        "registry retained home owner connect",
        fixture.connect(&node),
    )
    .await;
    Member {
        fixture,
        node,
        owner,
    }
}

fn token_file(fixture: &Fixture) -> std::path::PathBuf {
    // Same fixed loopback fixture relay credential as the existing relay helper.
    let path = fixture.cwd().join("relay-token");
    std::fs::write(&path, format!("{TOKEN}\n")).unwrap();
    path
}

pub(super) async fn member(
    resources: &mut Resources,
    relay: &Relay,
    home: &Member,
    descriptor: Option<&WorkspaceDescriptor>,
    label: &str,
    scopes: Vec<Scope>,
    grant: Vec<Scope>,
) -> Member {
    let fixture = Arc::new(Fixture::new());
    if let Some(descriptor) = descriptor {
        resources
            .seed(Arc::clone(&fixture), vec![descriptor.clone()])
            .await;
    }
    let config_fixture = Arc::clone(&fixture);
    let relay_url = relay.url.clone();
    let name = label.to_owned();
    let node = resources
        .node(Arc::clone(&fixture), move |config| {
            config.name = Some(name);
            config.relays = vec![relay_url];
            config.relay_token_file = Some(token_file(&config_fixture));
            config.fleet = fleet_config(
                HarnessKind::Codex,
                config_fixture.cwd().join("member-profile"),
            );
            config.vessel_mcp = Some(vessel_mcp(&config_fixture));
        })
        .await;
    registered(&node).await;
    // Only borrowed/read-only Arcs cross this existing wait. Its cancellation
    // cannot discard the separately retained actual Node shutdown owner.
    step(
        "registry retained member fleet join",
        crate::across::member_joined(home, fixture, node, label, scopes, grant),
    )
    .await
}
