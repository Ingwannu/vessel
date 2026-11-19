//! ADR 0115 over the wire: a real node installs a client-only plugin from a
//! directory under the home, keeps it disabled while plugins are off, runs it
//! once `node.set_settings` turns them on, and removes it; Paseo's manifest
//! is refused with its reason; an agent may list but not install.

use vessel_proto::ErrorCode;
use vessel_proto::plugins::{
    InstallPlugin, ListPlugins, PLUGINS_PROTOCOL, PluginGeneration, PluginStatus, RemovePlugin,
    WatchPlugins,
};
use vessel_proto::settings::SetNodeSettings;

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

const CLIENT: &str = r#"{"methods":[],"contributions":[],"themes":[]}"#;

fn plugin(fixture: &Fixture, name: &str) {
    let directory = fixture.user_home().join(name);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("vessel-plugin.json"),
        format!(r#"{{"id":"{name}","name":"Night","client":"client.json"}}"#),
    )
    .unwrap();
    std::fs::write(directory.join("client.json"), CLIENT).unwrap();
}

fn install(source: &str) -> InstallPlugin {
    InstallPlugin {
        source: source.to_owned(),
        id: None,
        reference: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_installs_switches_on_and_removes_a_plugin() {
    within(async {
        let fixture = Fixture::new();
        plugin(&fixture, "night");
        let paseo = fixture.user_home().join("paseo-made");
        std::fs::create_dir_all(&paseo).unwrap();
        std::fs::write(paseo.join("paseo-plugin.json"), r#"{"id":"paseo-made"}"#).unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        assert!(Some(owner.protocol()) >= PLUGINS_PROTOCOL);

        let installed = owner.call(install("~/night")).await.unwrap();
        assert_eq!(installed.status, PluginStatus::Disabled);
        assert!(installed.path.is_some());
        let refused = owner.call(install("~/paseo-made")).await;
        assert_eq!(code(refused), ErrorCode::InvalidRequest);
        let taken = owner.call(install("~/night")).await;
        assert_eq!(code(taken), ErrorCode::Conflict);

        let seen = owner
            .call(WatchPlugins {
                after: PluginGeneration::new(u64::MAX),
            })
            .await
            .unwrap();
        let settings = owner
            .call(SetNodeSettings {
                plugins_enabled: Some(true),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert!(settings.plugins_enabled);
        let mut generation = seen;
        let mut running = false;
        for _ in 0..20 {
            let list = owner.call(ListPlugins {}).await.unwrap();
            if list
                .plugins
                .iter()
                .any(|item| item.status == PluginStatus::Running)
            {
                running = true;
                break;
            }
            generation = owner
                .call(WatchPlugins { after: generation })
                .await
                .unwrap();
        }
        assert!(running);

        owner
            .call(RemovePlugin {
                plugin: "night".parse().unwrap(),
            })
            .await
            .unwrap();
        assert!(owner.call(ListPlugins {}).await.unwrap().plugins.is_empty());
        assert!(fixture.user_home().join("night").is_dir());
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_lists_plugins_without_paths_and_cannot_install() {
    within(async {
        let fixture = Fixture::new();
        plugin(&fixture, "night");
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = crate::workspace::lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let _installed = owner.call(install("~/night")).await.unwrap();
        let mut running = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        let listed = agent.call(ListPlugins {}).await.unwrap();
        assert_eq!(listed.plugins.len(), 1);
        assert!(listed.plugins.iter().all(|item| item.path.is_none()));
        let refused = agent.call(install("~/night")).await;
        assert_eq!(code(refused), ErrorCode::Forbidden);
        node.stop().await.unwrap();
    })
    .await;
}
