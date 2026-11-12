//! `agent.catalogue` end to end (protocol 11, ADR 0032): the registry served
//! by a loopback server of the test's (the registry itself is never
//! fetched by a test), and an agent added from the node's copy.

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;
use vessel_proto::ErrorCode;
use vessel_proto::agents::{
    AddAgent, AgentDetail, AgentSource, AgentToAdd, CatalogueRun, GetCatalogue,
};

use crate::agents::code;
use crate::support::{Fixture, within};

/// One entry run by npx, one by uvx.
const REGISTRY: &str = r#"{"version":"1.0.0","agents":[
  {"id":"example-agent","name":"Example Agent","version":"1.2.3","description":"by npx",
   "distribution":{"npx":{"package":"@example/acp@1.2.3","args":["--acp"],"env":{"EXAMPLE_MODE":"acp"}}}},
  {"id":"floating","name":"Floating","version":"1.0.0","description":"not pinned",
   "distribution":{"npx":{"package":"@example/floating@latest"}}}
]}"#;

/// Serve `body` to every request on a loopback port, counting requests;
/// the URL.
async fn serve(body: &'static str) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let served = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&served);
    drop(tokio::spawn(async move {
        loop {
            let Ok((mut socket, _peer)) = listener.accept().await else {
                return;
            };
            let _first = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(tokio::spawn(async move {
                let mut request = vec![0_u8; 4096];
                let _read = socket.read(&mut request).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _written = socket.write_all(response.as_bytes()).await;
                let _shut = socket.shutdown().await;
            }));
        }
    }));
    (format!("http://{address}/registry.json"), served)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_catalogue_is_fetched_when_asked_and_an_entry_is_added_from_the_copy() {
    within(async {
        let (url, served) = serve(REGISTRY).await;
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.registry_url = url.clone())
            .await;
        let owner = fixture.connect(&node).await;

        // Nothing is fetched until a person asks.
        let empty = owner.call(GetCatalogue { refresh: false }).await.unwrap();
        assert!(empty.fetched_at.is_none() && empty.entries.is_empty());
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 0);

        let catalogue = owner.call(GetCatalogue { refresh: true }).await.unwrap();
        assert!(catalogue.fetched_at.is_some());
        assert_eq!(catalogue.skipped, 1, "the unpinned entry");
        let entry = &catalogue.entries[0];
        let CatalogueRun::Command { program, args, .. } = &entry.runs else {
            panic!("a command: {entry:?}");
        };
        assert_eq!(program.as_str(), "npx");
        assert_eq!(args, &["-y", "@example/acp@1.2.3", "--acp"]);
        // Within a minute, a refresh answers the copy.
        let _again = owner.call(GetCatalogue { refresh: true }).await.unwrap();
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Added from the node's copy, at the version it showed.
        let added = owner
            .call(AddAgent {
                name: "example".parse().unwrap(),
                agent: AgentToAdd::Registry {
                    id: entry.id.clone(),
                    version: entry.version.clone(),
                    mcp: Vec::new(),
                },
                replace: false,
            })
            .await
            .unwrap();
        let AgentDetail::Added { added } = &added.detail else {
            panic!("added: {added:?}");
        };
        assert_eq!(added.program.as_str(), "npx");
        assert_eq!(added.args, ["-y", "@example/acp@1.2.3", "--acp"]);
        assert_eq!(added.env[0].as_str(), "EXAMPLE_MODE");
        assert_eq!(
            added.source,
            AgentSource::Registry {
                id: entry.id.clone(),
                version: entry.version.clone()
            }
        );
        // Not in the copy at that version.
        assert_eq!(
            code(
                owner
                    .call(AddAgent {
                        name: "other".parse().unwrap(),
                        agent: AgentToAdd::Registry {
                            id: entry.id.clone(),
                            version: "9.9.9".parse().unwrap(),
                            mcp: Vec::new(),
                        },
                        replace: false,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreachable_registry_is_unavailable_and_the_copy_stays() {
    within(async {
        // A port nothing listens on.
        let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/registry.json", closed.local_addr().unwrap());
        drop(closed);
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.registry_url = url.clone())
            .await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            code(owner.call(GetCatalogue { refresh: true }).await),
            ErrorCode::Unavailable
        );
        let copy = owner.call(GetCatalogue { refresh: false }).await.unwrap();
        assert!(copy.fetched_at.is_none());
        node.stop().await.unwrap();
    })
    .await;
}
