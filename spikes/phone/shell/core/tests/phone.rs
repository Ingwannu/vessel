//! The phone's core against a real node in this process (no harness, no
//! network beyond loopback): pairing by link into the app's own book,
//! connecting to the paired node through the source's connector, and the
//! bridge serving the page on loopback.
//!
//! No fake harness is installed: pairing and connecting need none, and
//! the testkit's nested build of the fakes (`cargo build -p
//! vessel-testkit`) cannot run from this workspace, where the testkit is
//! a path dependency and not a member (cargo's resolver panics). The
//! phone's sessions are tested end to end in the emulator instead, against
//! a node with the fakes built in the main workspace.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use vessel_client::{BearerToken, Connector as _, NodeClient};
use vessel_harness::Lookup;
use vessel_node::{Config, DataDir, Node};
use vessel_phone_core::{AppDirs, Bridge, PhoneSource, Roots, SourceError};
use vessel_proto::{ClientInfo, PairingLink};
use vessel_web::NodeSource as _;
use vessel_web::wire::{NodeKey, SecretText, Transport};

/// Every test's limit: a hang becomes a failure.
const LIMIT: Duration = Duration::from_secs(120);

fn client() -> ClientInfo {
    ClientInfo {
        name: "vessel-phone-test".to_owned(),
        version: "0".to_owned(),
    }
}

/// A node on loopback, finding no harness, in `dir`.
async fn node(dir: &Path) -> Node {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let home = dir.join("user-home");
    std::fs::create_dir_all(&home).unwrap();
    let mut config = Config::new(
        DataDir::new(dir.join("node")),
        vessel_harness::HarnessEnv::new(home),
    );
    config.bind = Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    config.harness_lookup = Lookup::new([bin]);
    Node::start(config).await.unwrap()
}

/// The node's owner, as the CLI on its machine is.
async fn owner(dir: &Path, node: &Node) -> NodeClient {
    let token: BearerToken = DataDir::new(dir.join("node"))
        .read_token()
        .unwrap()
        .expose()
        .parse()
        .unwrap();
    let url = format!("ws://{}", node.address().unwrap()).parse().unwrap();
    NodeClient::connect(&url, &token, client()).await.unwrap()
}

/// A fresh pairing link naming the node's loopback address as its direct
/// route (as `vesseld --public` would), as text.
async fn link(owner: &NodeClient, node: &Node) -> SecretText {
    let mut offer = owner
        .invite_device(None, 600)
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    offer.direct = vec![
        format!("ws://{}", node.address().unwrap())
            .parse()
            .unwrap(),
    ];
    SecretText::new(PairingLink::encode(&offer).unwrap().expose().to_owned())
}

fn app_dirs(dir: &Path) -> AppDirs {
    AppDirs {
        config: dir.join("app-files"),
        data: dir.join("app-data"),
    }
}

#[tokio::test]
async fn a_link_pairs_into_the_apps_book_and_the_node_is_reached_through_it() {
    tokio::time::timeout(LIMIT, async {
        let dir = tempfile::tempdir().unwrap();
        let node = node(dir.path()).await;
        let owner = owner(dir.path(), &node).await;
        let source = PhoneSource::new(app_dirs(dir.path()), Roots::none(), "phone".to_owned());

        let name = "desk".parse().unwrap();
        let paired = source.pair(&name, &link(&owner, &node).await).await.unwrap();
        assert_eq!(paired.transport, Transport::Paired);
        assert_eq!(paired.id, Some(node.info().id));
        // The first node paired is the default: a phone has none of its own.
        assert!(paired.default);

        let key = NodeKey::from(name.clone());
        let connected = source
            .connector(&key)
            .await
            .unwrap()
            .connect(client())
            .await
            .unwrap();
        assert_eq!(connected.node().id, node.info().id);

        // The name is taken now: refused before a second invitation is spent.
        let again = source.pair(&name, &link(&owner, &node).await).await;
        assert!(matches!(again, Err(SourceError::Book(_))));

        // A second node paired is not made the default.
        let other = source
            .pair(&"desk-2".parse().unwrap(), &link(&owner, &node).await)
            .await
            .unwrap();
        assert!(!other.default);

        // The credential is the app's alone (Unix modes, as the CLI's).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let credentials = dir.path().join("app-files").join("vessel").join("credentials");
            let mode = std::fs::metadata(&credentials).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }

        source.forget(&name).await.unwrap();
        let listed = source.nodes().await.unwrap();
        assert_eq!(listed.len(), 1);
        node.stop().await.unwrap();
    })
    .await
    .unwrap();
}

/// The bridge answers on loopback only, at the launch's own host name,
/// with the page.
#[tokio::test]
async fn the_bridge_serves_the_page_at_the_launch_host_on_loopback() {
    tokio::time::timeout(LIMIT, async {
        let dir = tempfile::tempdir().unwrap();
        let source = PhoneSource::new(app_dirs(dir.path()), Roots::none(), "phone".to_owned());
        let bridge = Bridge::start(source).await.unwrap();
        let address = bridge.address();
        assert!(address.ip().is_loopback());
        let url = bridge.url().expose().to_owned();
        assert!(!format!("{:?}", bridge.url()).contains("bootstrap"));
        let host = url
            .strip_prefix("http://")
            .and_then(|rest| rest.split('/').next())
            .unwrap()
            .to_owned();
        assert!(host.ends_with(&format!(".localhost:{}", address.port())));

        let get = |host: String| async move {
            let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
            let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
            socket.write_all(request.as_bytes()).await.unwrap();
            let mut answer = String::new();
            drop(socket.read_to_string(&mut answer).await);
            answer
        };
        let page = get(host).await;
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        // Another host name (a page another app serves, DNS rebinding)
        // is refused.
        let other = get(format!("localhost:{}", address.port() + 1)).await;
        assert!(!other.starts_with("HTTP/1.1 200"), "{other}");

        bridge.stop().await.unwrap();
    })
    .await
    .unwrap();
}
