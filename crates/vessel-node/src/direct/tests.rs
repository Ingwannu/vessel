//! The direct route's limits: places in all and per address, the header
//! deadline.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use vessel_relay::address::ClientAddress;

use super::{Limits, Places, run};

/// A safety net only: each wait below ends on an event first.
const NET: Duration = Duration::from_secs(20);

fn places(connections: usize, per_address: usize) -> Places {
    Places {
        all: std::sync::Arc::new(tokio::sync::Semaphore::new(connections)),
        per_address: std::sync::Arc::default(),
        limit: per_address,
    }
}

#[test]
fn one_address_holds_its_share_and_gives_it_back() {
    let places = places(16, 2);
    let host = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5));
    let first = places.take(host, true).unwrap();
    let _second = places.take(host, true).unwrap();
    assert!(places.take(host, true).is_none(), "past its share");
    let other = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 6));
    assert!(
        places.take(other, true).is_some(),
        "another address has its own"
    );
    drop(first);
    assert!(
        places.take(host, true).is_some(),
        "a closed socket's place returns"
    );
}

#[test]
fn an_ipv6_block_holds_more_and_uncounted_peers_only_the_cap_in_all() {
    let places = places(3, 1);
    let a = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 1, 0, 0, 0, 1));
    let b = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 2, 0, 0, 0, 1));
    let _a = places.take(a, true).unwrap();
    assert!(places.take(a, true).is_none(), "one per /64");
    let _b = places.take(b, true).unwrap();
    let _loopback = places.take(IpAddr::V4(Ipv4Addr::LOCALHOST), false).unwrap();
    assert!(
        places
            .take(IpAddr::V4(Ipv4Addr::LOCALHOST), false)
            .is_none(),
        "the cap in all holds everyone"
    );
}

async fn serve(limits: Limits) -> (std::net::SocketAddr, CancellationToken) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let stop = CancellationToken::new();
    drop(tokio::spawn(run(
        listener,
        app,
        ClientAddress::default(),
        limits,
        stop.clone(),
    )));
    (address, stop)
}

/// Everything the server sends until it closes.
async fn until_closed(stream: &mut TcpStream) -> String {
    let mut answer = Vec::new();
    let _read = tokio::time::timeout(NET, stream.read_to_end(&mut answer))
        .await
        .expect("the server closes the connection")
        .unwrap();
    String::from_utf8_lossy(&answer).into_owned()
}

#[tokio::test]
async fn a_socket_past_the_cap_is_refused_and_a_request_is_served() {
    let (address, stop) = serve(Limits {
        connections: 1,
        ..Limits::NODE
    })
    .await;
    let mut held = TcpStream::connect(address).await.unwrap();
    held.write_all(b"GET /health HTTP/1.1\r\nhost: x\r\n\r\n")
        .await
        .unwrap();
    let mut answer = [0_u8; 12];
    let _read = tokio::time::timeout(NET, held.read_exact(&mut answer))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&answer, b"HTTP/1.1 200");
    // `held` keeps its place (keep-alive): the next socket has none.
    let mut refused = TcpStream::connect(address).await.unwrap();
    assert!(
        until_closed(&mut refused)
            .await
            .starts_with("HTTP/1.1 503 ")
    );
    stop.cancel();
}

#[tokio::test]
async fn a_head_that_never_ends_is_closed_at_the_deadline() {
    let (address, stop) = serve(Limits {
        header_read: Duration::from_millis(200),
        ..Limits::NODE
    })
    .await;
    let mut slow = TcpStream::connect(address).await.unwrap();
    slow.write_all(b"GET /health HTTP/1.1\r\nhost: x\r\n")
        .await
        .unwrap();
    // No answer to a head that never ended: the connection just closes.
    assert!(!until_closed(&mut slow).await.contains("200"));
    stop.cancel();
}
