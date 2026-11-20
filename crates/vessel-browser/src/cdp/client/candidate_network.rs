//! The positive fixture and two private peers never leave loopback.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::time::Duration;

use axum::http::{StatusCode, Uri};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use vessel_proto::browser::{RequiredInput, WaitBudget, WaitFor, WaitMatch};

use crate::page::{Page, PageError};
use crate::proxy::PublicProxy;

const REFUSAL_DEADLINE: Duration = Duration::from_secs(2);
const REFUSED_HTTP: &[u8; 12] = b"HTTP/1.1 403";
const REFUSAL_PATH: &str = "/fixture-refused";

#[derive(Clone, Copy)]
pub(super) struct Routes {
    proxy: SocketAddr,
    canaries: [Canary; 2],
}

#[derive(Clone, Copy)]
struct Canary {
    address: SocketAddr,
    refused: &'static str,
}

impl Routes {
    pub(super) async fn verify_browser(&self, page: &mut Page) -> Result<(), PageError> {
        for canary in self.canaries {
            let url = format!("http://{}{REFUSAL_PATH}", canary.address)
                .parse()
                .unwrap();
            page.navigate(url).await?;
            let expected = RequiredInput::try_from(canary.refused.to_owned()).unwrap();
            let matched = page.wait(WaitFor::Text(expected), WaitBudget::Default).await?;
            assert_eq!(matched, WaitMatch::Text);
        }
        Ok(())
    }

    /// This test-only front door prevents any unrelated Chromium background
    /// request from reaching the actual policy proxy or starting DNS resolution.
    pub(super) async fn try_refusal(&self, uri: &Uri) -> Option<Result<&'static str, StatusCode>> {
        if uri.path() != REFUSAL_PATH || uri.query().is_some() {
            return None;
        }
        let destination = uri.authority()?.as_str().parse::<SocketAddr>().ok()?;
        let canary = self
            .canaries
            .iter()
            .find(|item| item.address == destination)?;
        let reply =
            tokio::time::timeout(REFUSAL_DEADLINE, read_refusal(self.proxy, destination)).await;
        Some(match reply {
            Ok(Ok(true)) => Ok(canary.refused),
            Ok(Ok(false) | Err(_)) | Err(_) => Err(StatusCode::BAD_GATEWAY),
        })
    }
}

async fn read_refusal(proxy: SocketAddr, destination: SocketAddr) -> std::io::Result<bool> {
    let mut socket = TcpStream::connect(proxy).await?;
    let request = format!(
        "GET http://{destination}{REFUSAL_PATH} HTTP/1.1\r\nHost: {destination}\r\nConnection: close\r\n\r\n"
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
                },
                Canary {
                    address: positive_control(&ipv6).await,
                    refused: "Fixture IPv6 refused",
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
