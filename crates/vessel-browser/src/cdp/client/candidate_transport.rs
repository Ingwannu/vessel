//! A dedicated WebTransport test uses an independently checked local H3 peer.

use std::net::SocketAddr;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::{IntoResponse as _, Response},
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use vessel_proto::browser::{ViewportSide, ViewportSize};

use super::{Client, Events};
use super::candidate_network::{ProbeError, expect_marker};
use crate::page::{Page, PageError};

const PAGE_PATH: &str = "/fixture-webtransport";
const REFUSAL: &str = "Fixture WebTransport refused";
const SCRIPT: &str = r#"
const transport = new WebTransport(target, {
    serverCertificateHashes: [{algorithm: "sha-256", value: new Uint8Array(fingerprint)}]
});
transport.closed.catch(() => {});
transport.ready.then(
    () => {
        document.body.textContent = "Fixture WebTransport unexpectedly connected";
        transport.close();
    },
    () => { document.body.textContent = "Fixture WebTransport refused"; }
);
"#;

#[derive(Debug, thiserror::Error)]
pub(super) enum FixtureError {
    #[error("no loopback H3 peer and SHA-256 fingerprint were supplied")]
    Configuration,
    #[error(transparent)]
    Page(#[from] PageError),
    #[error(transparent)]
    Probe(#[from] ProbeError),
}

#[derive(Clone)]
pub(super) struct TransportProbe {
    authority: String,
    document: String,
}

impl TransportProbe {
    pub(super) fn configured(listener_port: u16) -> Result<Self, FixtureError> {
        let peer: SocketAddr = std::env::var("VESSEL_BROWSER_H3_PEER")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|address: &SocketAddr| address.ip().is_loopback() && address.port() != 0)
            .ok_or(FixtureError::Configuration)?;
        let encoded = std::env::var("VESSEL_BROWSER_H3_SHA256")
            .map_err(|_invalid| FixtureError::Configuration)?;
        let fingerprint = sha256_bytes(&encoded).ok_or(FixtureError::Configuration)?;
        let endpoint = serde_json::to_string(&format!("https://{peer}/fixture"))
            .map_err(|_invalid| FixtureError::Configuration)?;
        let array =
            serde_json::to_string(&fingerprint).map_err(|_invalid| FixtureError::Configuration)?;
        let document = format!(
            "<!doctype html><meta charset=utf-8><body><script>const target = {endpoint};\nconst fingerprint = {array};\n{SCRIPT}</script></body>"
        );
        Ok(Self {
            authority: format!("127.0.0.1:{listener_port}"),
            document,
        })
    }

    pub(super) async fn verify(
        &self,
        client: &Client,
        events: &mut Events,
    ) -> Result<(), FixtureError> {
        events.during_fixture(self.open(client)).await.map_err(PageError::Cdp)?
    }

    async fn open(&self, client: &Client) -> Result<(), FixtureError> {
        let address = format!("http://{}{PAGE_PATH}", self.authority);
        let side = ViewportSide::try_from(390).unwrap();
        let viewport = ViewportSize {
            width: side,
            height: side,
        };
        let mut page = Page::open(client.clone(), address.parse().unwrap(), viewport).await?;
        let observed = expect_marker(&mut page, REFUSAL).await;
        page.close().await?;
        observed.map_err(FixtureError::from)
    }

    pub(super) async fn serve(
        self,
        listener: TcpListener,
        stop: CancellationToken,
    ) -> std::io::Result<()> {
        let router = axum::Router::new().fallback(move |request: Request<Body>| {
            let response = self.response(&request);
            async { response }
        });
        axum::serve(listener, router)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
    }

    fn response(&self, request: &Request<Body>) -> Response {
        if request.method() != Method::GET
            || request.uri().authority().map(axum::http::uri::Authority::as_str)
                != Some(self.authority.as_str())
            || request.uri().path() != PAGE_PATH
            || request.uri().query().is_some()
        {
            return StatusCode::FORBIDDEN.into_response();
        }
        (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            self.document.clone(),
        )
            .into_response()
    }
}

fn sha256_bytes(encoded: &str) -> Option<[u8; 32]> {
    if encoded.len() != 64 || !encoded.is_ascii() {
        return None;
    }
    let mut fingerprint = [0; 32];
    for (index, byte) in fingerprint.iter_mut().enumerate() {
        let offset = index.checked_mul(2)?;
        let pair = encoded.get(offset..offset.checked_add(2)?)?;
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(fingerprint)
}
