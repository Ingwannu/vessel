//! ADR 0115 over the wire: a real node installs a client-only plugin from a
//! directory under the home, keeps it disabled while plugins are off, runs it
//! once `node.set_settings` turns them on, and removes it; Paseo's manifest
//! is refused with its reason; an agent may list but not install; a paired
//! Read device lists without paths, reads the catalog, and may not install,
//! read logs or call a method.

use vessel_proto::plugins::{
    GetPluginCatalog, GetPluginLogs, InstallPlugin, InvokeContext, InvokePlugin, ListPlugins,
    PLUGINS_PROTOCOL, PluginGeneration, PluginStatus, RemovePlugin, WatchPlugins,
};
use vessel_proto::settings::SetNodeSettings;
use vessel_proto::{ErrorCode, RawJson, Scope};

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

        let _removed = owner
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_device_sees_the_catalog_but_not_paths_logs_or_methods() {
    within(async {
        let fixture = Fixture::new();
        plugin(&fixture, "night");
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let _installed = owner.call(install("~/night")).await.unwrap();
        let _on = owner
            .call(SetNodeSettings {
                plugins_enabled: Some(true),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;
        let listed = reader.call(ListPlugins {}).await.unwrap();
        assert_eq!(listed.plugins.len(), 1);
        assert!(
            listed
                .plugins
                .iter()
                .all(|item| item.path.is_none() && item.error.is_none())
        );
        // The catalog is declarative data a Read device draws; the switch
        // is applied in the background, so wait for the plugin to run.
        let mut generation = PluginGeneration::new(u64::MAX);
        let mut catalogued = 0;
        for _ in 0..20 {
            let catalog = reader.call(GetPluginCatalog {}).await.unwrap();
            catalogued = catalog.catalog.installations().len();
            if catalogued == 1 {
                break;
            }
            generation = reader
                .call(WatchPlugins { after: generation })
                .await
                .unwrap();
        }
        assert_eq!(catalogued, 1);
        // A Read device draws the declarations but is told it may not run
        // them; the owner may.
        assert!(!reader.call(GetPluginCatalog {}).await.unwrap().may_invoke);
        assert!(owner.call(GetPluginCatalog {}).await.unwrap().may_invoke);
        let plugin_id = || "night".parse().unwrap();
        assert_eq!(
            code(reader.call(install("~/night")).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(
                reader
                    .call(GetPluginLogs {
                        plugin: plugin_id()
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        let catalog = reader.call(GetPluginCatalog {}).await.unwrap();
        let generation = catalog
            .catalog
            .installations()
            .first()
            .map(|installation| installation.address().generation)
            .unwrap();
        let call = InvokePlugin {
            plugin: plugin_id(),
            generation,
            method: "night.read".parse().unwrap(),
            input: RawJson::empty_object(),
            context: InvokeContext::Global,
        };
        assert_eq!(code(reader.call(call.clone()).await), ErrorCode::Forbidden);
        // The owner reaches the node, which says a client-only plugin has
        // no methods.
        assert_eq!(code(owner.call(call).await), ErrorCode::NotFound);
        node.stop().await.unwrap();
    })
    .await;
}
