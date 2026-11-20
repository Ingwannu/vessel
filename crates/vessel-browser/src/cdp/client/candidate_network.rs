//! The positive fixture and two private peers never leave loopback.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::time::Duration;

use axum::http::{StatusCode, Uri};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpStream, UdpSocket};
use vessel_proto::browser::{RequiredInput, WaitBudget, WaitFor, WaitMatch, WaitTimeout};

use crate::page::{Page, PageError};
use crate::proxy::PublicProxy;

const REFUSAL_DEADLINE: Duration = Duration::from_secs(2);
const REFUSED_HTTP: &[u8; 12] = b"HTTP/1.1 403";
const REFUSAL_PATH: &str = "/fixture-refused";
const REDIRECTED_PATH: &str = "/fixture-refused-after-redirect";
const EMBEDDED_PATH: &str = "/fixture-refused-embedded";
const WORKER_PATH: &str = "/fixture-refused-worker";
const REDIRECTS: [&str; 2] = ["/fixture-redirect-v4", "/fixture-redirect-v6"];
const DATAGRAM_CONTROL: &[u8] = b"local fixture";

#[derive(Clone, Copy)]
pub(super) struct Routes {
    proxy: SocketAddr,
    canaries: [Canary; 2],
}

#[derive(Clone, Copy)]
struct Canary {
    address: SocketAddr,
    datagram: SocketAddr,
    refused: &'static str,
    refused_after_redirect: &'static str,
}

#[derive(Debug, thiserror::Error)]
#[error("the local browser fixture did not observe {marker}")]
pub(super) struct ProbeError {
    marker: &'static str,
    #[source]
    cause: PageError,
}

impl Routes {
    pub(super) async fn verify_browser(&self, page: &mut Page) -> Result<(), ProbeError> {
        for (canary, redirect) in self.canaries.into_iter().zip(REDIRECTS) {
            let address = canary.url(REFUSAL_PATH);
            let indirect = format!("http://fixture.test{redirect}");
            for (route, marker) in [
                (address, canary.refused),
                (indirect, canary.refused_after_redirect),
            ] {
                visit(page, &route, marker).await?;
            }
        }
        Ok(())
    }

    pub(super) fn redirect(&self, path: &str) -> Option<String> {
        REDIRECTS
            .into_iter()
            .zip(self.canaries)
            .find_map(|(route, peer)| (path == route).then(|| peer.url(REDIRECTED_PATH)))
    }

    pub(super) fn expected_reads(&self, worker: bool) -> [(String, &'static str); 2] {
        let path = if worker { WORKER_PATH } else { EMBEDDED_PATH };
        self.canaries.map(|peer| (peer.url(path), peer.refused))
    }

    pub(super) fn ice_urls(&self) -> impl Iterator<Item = String> {
        self.canaries.into_iter().flat_map(|peer| {
            [
                format!("stun:{}", peer.datagram),
                format!("turn:{}?transport=udp", peer.datagram),
                format!("turn:{}?transport=tcp", peer.address),
            ]
        })
    }

    /// This test-only front door prevents any unrelated Chromium background
    /// request from reaching the actual policy proxy or starting DNS resolution.
    pub(super) async fn try_refusal(&self, uri: &Uri) -> Option<Result<&'static str, StatusCode>> {
        let path = uri.path();
        if !matches!(
            path,
            REFUSAL_PATH | REDIRECTED_PATH | EMBEDDED_PATH | WORKER_PATH
        ) || uri.query().is_some()
        {
            return None;
        }
        let destination = uri.authority()?.as_str().parse::<SocketAddr>().ok()?;
        let canary = self
            .canaries
            .iter()
            .find(|item| item.address == destination)?;
        let reply = tokio::time::timeout(
            REFUSAL_DEADLINE,
            read_refusal(self.proxy, destination, path),
        )
        .await;
        let marker = if path == REDIRECTED_PATH {
            canary.refused_after_redirect
        } else {
            canary.refused
        };
        Some(match reply {
            Ok(Ok(true)) => Ok(marker),
            Ok(Ok(false) | Err(_)) | Err(_) => Err(StatusCode::BAD_GATEWAY),
        })
    }
}

impl Canary {
    fn url(self, fixture_path: &str) -> String {
        format!("http://{}{fixture_path}", self.address)
    }
}

pub(super) async fn visit(
    page: &mut Page,
    address: &str,
    marker: &'static str,
) -> Result<(), ProbeError> {
    let probe = async {
        page.navigate(address.parse().unwrap()).await?;
        wait_for(page, marker).await
    };
    probe.await.map_err(|cause| ProbeError { marker, cause })
}

async fn wait_for(page: &mut Page, marker: &str) -> Result<(), PageError> {
    let expected = RequiredInput::try_from(marker.to_owned()).unwrap();
    let budget = WaitBudget::Explicit(WaitTimeout::try_from(3500).unwrap());
    let matched = page.wait(WaitFor::Text(expected), budget).await?;
    assert_eq!(matched, WaitMatch::Text);
    Ok(())
}

async fn read_refusal(
    proxy: SocketAddr,
    destination: SocketAddr,
    path: &str,
) -> std::io::Result<bool> {
    let mut socket = TcpStream::connect(proxy).await?;
    let request = format!(
        "GET http://{destination}{path} HTTP/1.1\r\nHost: {destination}\r\nConnection: close\r\n\r\n"
    );
    socket.write_all(request.as_bytes()).await?;
    let mut status = [0; REFUSED_HTTP.len()];
    let _received = socket.read_exact(&mut status).await?;
    Ok(&status == REFUSED_HTTP)
}

pub(super) struct NetworkFixture {
    proxy: PublicProxy,
    stream_peers: [TcpListener; 2],
    datagram_peers: [std::net::UdpSocket; 2],
    routes: Routes,
}

impl NetworkFixture {
    pub(super) async fn start() -> Self {
        let ipv4 = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let ipv6 = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let udp4 = checked_datagram(Ipv4Addr::LOCALHOST.into()).await;
        let udp6 = checked_datagram(Ipv6Addr::LOCALHOST.into()).await;
        let proxy = PublicProxy::start().await.unwrap();
        let routes = Routes {
            proxy: proxy.endpoint(),
            canaries: [
                Canary {
                    address: positive_control(&ipv4).await,
                    datagram: udp4.local_addr().unwrap(),
                    refused: "Fixture IPv4 refused",
                    refused_after_redirect: "Fixture redirected IPv4 refused",
                },
                Canary {
                    address: positive_control(&ipv6).await,
                    datagram: udp6.local_addr().unwrap(),
                    refused: "Fixture IPv6 refused",
                    refused_after_redirect: "Fixture redirected IPv6 refused",
                },
            ],
        };
        Self {
            proxy,
            stream_peers: [ipv4, ipv6],
            datagram_peers: [udp4, udp6],
            routes,
        }
    }

    pub(super) const fn routes(&self) -> Routes {
        self.routes
    }

    pub(super) async fn finish(self) {
        self.proxy.close().await.unwrap();
        // A direct Chromium bypass or a successful policy dial leaves a buffered connection.
        for canary in self.stream_peers {
            assert_eq!(canary.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        }
        for canary in self.datagram_peers {
            let mut payload = [0; 128];
            assert_eq!(
                canary.recv_from(&mut payload).unwrap_err().kind(),
                ErrorKind::WouldBlock
            );
        }
    }
}

async fn checked_datagram(ip: std::net::IpAddr) -> std::net::UdpSocket {
    let canary = UdpSocket::bind((ip, 0)).await.unwrap();
    let control = UdpSocket::bind((ip, 0)).await.unwrap();
    let written = control
        .send_to(DATAGRAM_CONTROL, canary.local_addr().unwrap())
        .await
        .unwrap();
    let mut payload = [0; 32];
    let (received, from) = tokio::time::timeout(REFUSAL_DEADLINE, canary.recv_from(&mut payload))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(from, control.local_addr().unwrap());
    assert_eq!(written, received);
    assert_eq!(payload.get(..received), Some(DATAGRAM_CONTROL));
    canary.into_std().unwrap()
}

async fn positive_control(canary: &TcpListener) -> SocketAddr {
    let address = canary.local_addr().unwrap();
    canary.set_nonblocking(true).unwrap();
    let stream = TcpStream::connect(address).await.unwrap();
    let (accepted, _) = canary.accept().unwrap();
    drop(stream);
    drop(accepted);
    address
}
