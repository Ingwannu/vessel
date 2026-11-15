//! Own ephemeral loopback fixtures only; these are not Chromium qualification.

use super::*;
use std::io::ErrorKind;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);

#[tokio::test]
async fn a_live_private_listener_is_refused_for_http_and_connect() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let canary = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        canary.set_nonblocking(true).unwrap();
        let canary_address = canary.local_addr().unwrap();
        // Positive control: the listener is reachable by an ordinary local dial.
        let direct = TcpStream::connect(canary_address).await.unwrap();
        let (accepted, _) = canary.accept().unwrap();
        drop(direct);
        drop(accepted);

        let proxy = PublicProxy::start().await.unwrap();
        for request in [
            format!("GET http://{canary_address}/ HTTP/1.1\r\nHost: ignored.test\r\nConnection: close\r\n\r\n"),
            format!("CONNECT {canary_address} HTTP/1.1\r\nHost: ignored.test\r\nConnection: close\r\n\r\n"),
        ] {
            let mut client = TcpStream::connect(proxy.endpoint()).await.unwrap();
            client.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            let _bytes = client.read_to_string(&mut response).await.unwrap();
            assert!(response.starts_with("HTTP/1.1 403"), "{response}");
            assert!(!response.contains("ignored.test"));
            assert_eq!(canary.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        }
        proxy.close().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn dropping_the_profile_cancels_an_incomplete_client_connection() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let proxy = PublicProxy::start().await.unwrap();
        let mut client = TcpStream::connect(proxy.endpoint()).await.unwrap();
        client.write_all(b"GET ").await.unwrap();
        proxy.close().await.unwrap();
        let mut data = Vec::new();
        // EOF or reset are both closed transports; no child route can survive.
        if let Err(error) = client.read_to_end(&mut data).await {
            assert!(matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::UnexpectedEof
            ));
        }
    })
    .await
    .unwrap();
}
