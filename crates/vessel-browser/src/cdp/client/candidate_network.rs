//! The positive fixture and two private peers never leave loopback.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::time::Duration;

use axum::http::{StatusCode, Uri};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
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

#[derive(Clone, Copy)]
pub(super) struct Routes {
    proxy: SocketAddr,
    canaries: [Canary; 2],
}

#[derive(Clone, Copy)]
struct Canary {
    address: SocketAddr,
    refused: &'static str,
    refused_after_redirect: &'static str,
}

impl Routes {
    pub(super) async fn verify_browser(&self, page: &mut Page) -> Result<(), PageError> {
        for (canary, redirect) in self.canaries.into_iter().zip(REDIRECTS) {
            let address = canary.url(REFUSAL_PATH);
            let indirect = format!("http://fixture.test{redirect}");
            for (route, marker) in [
                (address, canary.refused),
                (indirect, canary.refused_after_redirect),
            ] {
                page.navigate(route.parse().unwrap()).await?;
                wait_for(page, marker).await?;
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

pub(super) async fn wait_for(page: &mut Page, marker: &str) -> Result<(), PageError> {
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
    peers: [TcpListener; 2],
    routes: Routes,
}

impl NetworkFixture {
    pub(super) async fn start() -> Self {
        let ipv4 = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let ipv6 = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let proxy = PublicProxy::start().await.unwrap();
        let routes = Routes {
            proxy: proxy.endpoint(),
            canaries: [
                Canary {
                    address: positive_control(&ipv4).await,
                    refused: "Fixture IPv4 refused",
                    refused_after_redirect: "Fixture redirected IPv4 refused",
                },
                Canary {
                    address: positive_control(&ipv6).await,
                    refused: "Fixture IPv6 refused",
                    refused_after_redirect: "Fixture redirected IPv6 refused",
                },
            ],
        };
        Self {
            proxy,
            peers: [ipv4, ipv6],
            routes,
        }
    }

    pub(super) const fn routes(&self) -> Routes {
        self.routes
    }

    pub(super) async fn finish(self) {
        self.proxy.close().await.unwrap();
        // A direct Chromium bypass or a successful policy dial leaves a buffered connection.
        for canary in self.peers {
            assert_eq!(canary.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        }
    }
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
