//! The adapter over a real scope, driven through reqwest's actual hook. Only
//! numeric literals and loopback `localhost` are resolved; nothing leaves
//! the machine.
use super::HttpResolver;
use reqwest::dns::{Name, Resolve};
use std::{
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use vessel_relay::resolution::{ResolutionBudget, ResolutionQuota, ScopeState};

const PHASE: Duration = Duration::from_secs(20);

fn scope() -> vessel_relay::resolution::ResolutionScope<()> {
    ResolutionBudget::default().scope(ResolutionQuota::new(1).unwrap(), Arc::new(()))
}

#[tokio::test]
async fn names_resolve_through_the_scope_with_port_zero_and_drain() {
    tokio::time::timeout(PHASE, async {
        let scope = scope();
        let resolver = HttpResolver::new(scope.handle());
        let found: Vec<SocketAddr> = resolver
            .resolve(Name::from_str("127.0.0.1").unwrap())
            .await
            .unwrap()
            .collect();
        assert_eq!(found, [SocketAddr::from((Ipv4Addr::LOCALHOST, 0))]);
        let _drained = scope.drain().await.unwrap();
        assert_eq!(scope.state(), ScopeState::Drained);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_closed_scope_refuses_safely_and_reqwest_never_falls_back() {
    tokio::time::timeout(PHASE, async {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let scope = scope();
        let resolver = Arc::new(HttpResolver::new(scope.handle()));
        scope.close();
        let refused = match resolver.resolve(Name::from_str("localhost").unwrap()).await {
            Ok(_found) => panic!("a closed scope resolved a name"),
            Err(error) => error,
        };
        assert_eq!(refused.to_string(), "resolution scope is closed");
        assert!(!format!("{refused:?}").contains("localhost"));
        let client = reqwest::Client::builder()
            .no_proxy()
            .dns_resolver(resolver)
            .build()
            .unwrap();
        let sent = client.get(format!("http://localhost:{port}/")).send().await;
        assert!(sent.is_err());
        // A default-resolver fallback would have completed a TCP handshake
        // into this queue before the request returned.
        assert_eq!(
            listener.accept().map(|_accepted| ()).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let _drained = scope.drain().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_real_request_connects_through_the_tracked_lookup() {
    tokio::time::timeout(PHASE, async {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _read = stream.read(&mut request).await.unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let scope = scope();
        let client = reqwest::Client::builder()
            .no_proxy()
            .dns_resolver(Arc::new(HttpResolver::new(scope.handle())))
            .build()
            .unwrap();
        let response = client
            .get(format!("http://localhost:{port}/"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
        server.await.unwrap();
        drop(client);
        let _drained = scope.drain().await.unwrap();
        assert_eq!(scope.state(), ScopeState::Drained);
    })
    .await
    .unwrap();
}
