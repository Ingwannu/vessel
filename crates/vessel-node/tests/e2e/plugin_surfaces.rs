//! ADR 0115's surfaces over the wire: the testkit's `fake-plugin` declares a
//! settings screen whose card binds its `display` document, so its catalog
//! entry needs the surfaces' protocol. A current client gets it, beside a
//! client-only plugin. A client that offers only the catalog's protocol (41)
//! gets a reply it decodes, holding the client-only plugin and not the
//! helper (coordinator ruling on P3b, Q4).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use vessel_proto::plugins::{
    GetPluginCatalog, InstallPlugin, PLUGIN_CATALOG_PROTOCOL, PLUGIN_SURFACES_PROTOCOL,
    PluginCatalog, PluginGeneration, WatchPlugins,
};
use vessel_proto::settings::SetNodeSettings;
use vessel_proto::{
    ClientFrame, Hello, Outcome, ProtocolRange, ProtocolVersion, Reply, Request as _, RequestId,
    ServerFrame,
};
use vessel_testkit::Fake;

use crate::support::{Fixture, client, url, within};

/// `~/prefs`: `fake-plugin` through a one-line `sh` wrapper, as the node's
/// unit tests run it.
fn helper(fixture: &Fixture) {
    let directory = fixture.user_home().join("prefs");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("vessel-plugin.json"),
        r#"{"id":"prefs","server":{"command":["./run.sh"]}}"#,
    )
    .unwrap();
    let script = format!(
        "#!/bin/sh\nexec '{}'\n",
        Fake::Plugin.path().unwrap().display()
    );
    let run = directory.join("run.sh");
    // Never written by this process (ETXTBSY; AGENTS.md 8).
    vessel_testkit::write_executable(&run, &script).unwrap();
    std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// `~/night`: a client-only plugin using only the catalog's first kinds.
fn client_only(fixture: &Fixture) {
    let directory = fixture.user_home().join("night");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("vessel-plugin.json"),
        r#"{"id":"night","client":"client.json"}"#,
    )
    .unwrap();
    std::fs::write(
        directory.join("client.json"),
        r#"{"methods":[],"contributions":[],"themes":[]}"#,
    )
    .unwrap();
}

fn install(source: &str) -> InstallPlugin {
    InstallPlugin {
        source: source.to_owned(),
        id: None,
        reference: None,
    }
}

fn ids(catalog: &PluginCatalog) -> Vec<String> {
    catalog
        .catalog
        .installations()
        .iter()
        .map(|entry| entry.address().plugin.to_string())
        .collect()
}

/// `plugin.catalog` on a raw connection that offers only `version`, decoded
/// as this build decodes; the reply must decode at all.
async fn catalog_at(
    fixture: &Fixture,
    node: &vessel_node::Node,
    version: ProtocolVersion,
) -> PluginCatalog {
    let mut request = url(node).as_uri().clone().into_client_request().unwrap();
    let bearer = format!("Bearer {}", fixture.token().expose());
    let _previous = request
        .headers_mut()
        .insert(AUTHORIZATION, bearer.parse().unwrap());
    let (mut socket, _response) = tokio_tungstenite::connect_async(request).await.unwrap();
    let hello = ClientFrame::Hello(Hello {
        protocol: ProtocolRange {
            min: version,
            max: version,
        },
        client: client(),
    });
    let frames = [
        hello,
        ClientFrame::Request {
            id: RequestId::new(1),
            call: GetPluginCatalog {}.into_call(),
        },
    ];
    for frame in frames {
        socket
            .send(Message::text(serde_json::to_string(&frame).unwrap()))
            .await
            .unwrap();
    }
    loop {
        let Some(Ok(Message::Text(text))) = socket.next().await else {
            panic!("the node answers");
        };
        if let ServerFrame::Response { outcome, .. } =
            serde_json::from_str::<ServerFrame>(text.as_str()).unwrap()
        {
            let Outcome::Ok(Reply::GetPluginCatalog(catalog)) = outcome else {
                panic!("a catalog: {outcome:?}");
            };
            return catalog;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_client_gets_the_catalog_without_the_entry_it_cannot_decode() {
    within(async {
        let fixture = Fixture::new();
        helper(&fixture);
        client_only(&fixture);
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        for source in ["~/prefs", "~/night"] {
            let _installed = owner.call(install(source)).await.unwrap();
        }
        let _on = owner
            .call(SetNodeSettings {
                plugins_enabled: Some(true),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        // The switch applies in the background; wait for both to run.
        let mut seen = PluginGeneration::new(u64::MAX);
        let mut current = owner.call(GetPluginCatalog {}).await.unwrap();
        for _ in 0..40 {
            if ids(&current).len() == 2 {
                break;
            }
            seen = owner.call(WatchPlugins { after: seen }).await.unwrap();
            current = owner.call(GetPluginCatalog {}).await.unwrap();
        }
        assert_eq!(ids(&current), ["night", "prefs"]);
        let needs = |id: &str| {
            current
                .catalog
                .installations()
                .iter()
                .find(|entry| entry.address().plugin.as_str() == id)
                .map(|entry| Some(entry.required_client()))
        };
        assert_eq!(needs("prefs"), Some(PLUGIN_SURFACES_PROTOCOL));
        assert_eq!(needs("night"), Some(PLUGIN_CATALOG_PROTOCOL));

        let older = PLUGIN_CATALOG_PROTOCOL.unwrap();
        let catalog = catalog_at(&fixture, &node, older).await;
        assert_eq!(ids(&catalog), ["night"]);
        assert_eq!(catalog.generation, current.generation);
        let text = serde_json::to_string(&catalog).unwrap();
        for kind in ["settings_select", "\"card\"", "open_settings"] {
            assert!(!text.contains(kind), "{kind} in {text}");
        }
        node.stop().await.unwrap();
    })
    .await;
}
