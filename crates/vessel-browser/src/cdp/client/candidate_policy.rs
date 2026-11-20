//! Exercise embedded and service worker requests in a trusted local origin.

use axum::{
    http::{Uri, header},
    response::{IntoResponse as _, Response},
};

use super::candidate_network::{Routes, wait_for};
use crate::page::{Page, PageError};

const EMBEDDED: &str = "/fixture-embedded";
const WORKER_PAGE: &str = "/fixture-worker";
const WORKER_SCRIPT: &str = "/fixture-worker.js";
const WORKER_CONTROL: &str = "/fixture-worker-control";
const READ_CANARIES: &str = r#"
async function observedReads() {
    const responses = await Promise.all(probes.map(async ([url, marker]) => {
        const response = await fetch(url, {cache: "no-store"});
        return response.ok && (await response.text()) === marker;
    }));
    return responses.every(Boolean);
}
"#;
const READ_FROM_PAGE: &str = r#"
observedReads().then(ok => {
    if (ok) document.body.textContent = "Fixture embedded traffic refused";
});
"#;
const WORKER_CLIENT: &str = r#"
navigator.serviceWorker.register("/fixture-worker.js").then(async () => {
    const {active} = await navigator.serviceWorker.ready;
    const channel = new MessageChannel();
    channel.port1.onmessage = event => {
        if (event.data === true) document.body.textContent = "Fixture worker traffic refused";
    };
    active.postMessage("probe", [channel.port2]);
});
"#;
const WORKER_PROBE: &str = r#"
self.addEventListener("install", event => event.waitUntil(self.skipWaiting()));
self.addEventListener("message", event => {
    if (event.data !== "probe") return;
    event.waitUntil((async () => {
        const control = await fetch("/fixture-worker-control", {cache: "no-store"});
        const ready = control.ok && (await control.text()) === "worker positive control";
        event.ports[0].postMessage(ready && await observedReads());
    })());
});
"#;

pub(super) struct PolicyOrigin {
    authority: String,
    origin: String,
}

impl PolicyOrigin {
    pub(super) fn new(port: u16) -> Self {
        // This socket is the already-held fixture listener. A browser that
        // bypasses its proxy sends an origin-form request and fails here.
        let authority = format!("localhost:{port}");
        let origin = format!("http://{authority}");
        Self { authority, origin }
    }

    pub(super) fn origin(&self) -> &str {
        &self.origin
    }

    pub(super) fn serve(&self, uri: &Uri, routes: Routes) -> Option<Response> {
        if uri.authority()?.as_str() != self.authority || uri.query().is_some() {
            return None;
        }
        let response = match uri.path() {
            EMBEDDED => html(&script(routes, false, READ_FROM_PAGE)),
            WORKER_PAGE => html(WORKER_CLIENT),
            WORKER_SCRIPT => (
                [
                    (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                script(routes, true, WORKER_PROBE),
            )
                .into_response(),
            WORKER_CONTROL => (
                [(header::CACHE_CONTROL, "no-store")],
                "worker positive control",
            )
                .into_response(),
            _ => return None,
        };
        Some(response)
    }

    pub(super) async fn verify(&self, page: &mut Page) -> Result<(), PageError> {
        for (path, marker) in [
            (EMBEDDED, "Fixture embedded traffic refused"),
            (WORKER_PAGE, "Fixture worker traffic refused"),
        ] {
            let address = format!("{}{path}", self.origin);
            page.navigate(address.parse().unwrap()).await?;
            wait_for(page, marker).await?;
        }
        Ok(())
    }
}

fn script(routes: Routes, worker: bool, source: &str) -> String {
    let probes = serde_json::to_string(&routes.expected_reads(worker)).unwrap();
    format!("const probes = {probes};\n{READ_CANARIES}\n{source}")
}

fn html(script: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        format!("<!doctype html><meta charset=utf-8><body><script>{script}</script></body>"),
    )
        .into_response()
}
