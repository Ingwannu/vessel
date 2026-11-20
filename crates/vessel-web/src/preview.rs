//! The page's HTML preview route (ADR 0122, TRACKER 814): an HTML file's
//! Preview runs its own scripts, as Paseo's does, in a document of its own
//! origin.
//!
//! [Decision Log]
//! - Purpose: Paseo previews an HTML file in a frame whose scripts run
//!   (`allow-scripts`, an opaque origin, its own content policy), so a plan
//!   page keeps its diagrams and charts.
//! - Constraint: a `srcdoc`, `blob:` or `data:` frame takes the page's own
//!   policy (ADR 0017: `script-src 'self'` and the import map's hash), which
//!   refuses the file's inline scripts, and that policy stays as it is (the
//!   coordinator's ruling 1 for the file viewers). A document with a policy
//!   of its own has to come over the network.
//! - Alternatives: another port (another origin, the same site: no more
//!   isolation, a second listener to guard and to proxy); another site (a
//!   process boundary too, but a host name per deployment).
//! - Choice: the page posts the document it would have framed
//!   (`POST /api/preview`, the session key as a bearer, our origin only),
//!   and frames `GET /preview/<id>`. The answer carries Paseo's preview
//!   policy as its own header, with `sandbox allow-scripts` (an opaque
//!   origin even when opened outside the frame) and
//!   `frame-ancestors 'self'`. The guard leaves that answer's policy alone
//!   and adds the headers every answer carries.
//! - Cost: documents are held in memory, per launch, at most `MAX_DOCUMENTS`
//!   and `MAX_BYTES`, one per panel (`slot`), each dropped `IDLE` after its
//!   last load. The id is the only key to a document (a frame's load can
//!   carry no header): 128 random bits. A document may still navigate
//!   itself, Paseo's own stated hole.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Json;
use axum::body::{Bytes, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_LENGTH, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse as _, Response};
use serde::Serialize;
use tokio::time::Instant;

use crate::server::{Ui, authorized};
use crate::source::NodeSource;

/// Paseo's preview policy (`file-panels/html-preview-csp.ts`, pinned
/// 0.15.4), then the frame's sandbox and who may frame it.
const POLICY: &str = concat!(
    "default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval' blob:; ",
    "style-src 'unsafe-inline'; img-src data: blob:; font-src data:; ",
    "media-src data: blob:; connect-src 'none'; form-action 'none'; ",
    "base-uri 'none'; frame-src 'none'; object-src 'none'; ",
    "sandbox allow-scripts; frame-ancestors 'self'"
);

/// The most documents kept at once.
pub(crate) const MAX_DOCUMENTS: usize = 8;

/// The most bytes kept at once, and so the largest document.
pub(crate) const MAX_BYTES: usize = 64 * 1024 * 1024;

/// How long a document is kept after it was last loaded (or posted).
pub(crate) const IDLE: Duration = Duration::from_secs(10 * 60);

/// The header naming the panel a document is for.
const SLOT: &str = "x-vessel-preview-slot";

/// Marks the answer whose own policy the guard keeps.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreviewDocument;

/// One document.
#[derive(Debug)]
struct Document {
    slot: String,
    body: Bytes,
    used: Instant,
}

/// The documents of this launch, by id.
#[derive(Debug, Default)]
pub(crate) struct Previews {
    documents: Mutex<HashMap<String, Document>>,
}

/// Why a document was not kept.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// Larger than `MAX_BYTES`.
    TooLarge,
    /// No randomness for its id.
    NoId,
}

impl Previews {
    /// Keep `body` for `slot`, replacing that panel's last document; its id.
    /// The same document posted again for its panel keeps its id and starts
    /// its idle time again, so a panel that reposts while it is shown keeps
    /// its frame's address alive.
    pub(crate) fn put(&self, slot: &str, body: Bytes, now: Instant) -> Result<String, Refused> {
        if body.len() > MAX_BYTES {
            return Err(Refused::TooLarge);
        }
        let mut documents = self
            .documents
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        documents.retain(|_, kept| !expired(kept, now));
        if let Some((id, kept)) = documents
            .iter_mut()
            .find(|(_, kept)| kept.slot == slot && kept.body == body)
        {
            kept.used = now;
            return Ok(id.clone());
        }
        let id = fresh_id().ok_or(Refused::NoId)?;
        documents.retain(|_, kept| kept.slot != slot);
        while documents.len() >= MAX_DOCUMENTS
            || total(&documents).saturating_add(body.len()) > MAX_BYTES
        {
            let Some(oldest) = documents
                .iter()
                .min_by_key(|(_, kept)| kept.used)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            let _dropped = documents.remove(&oldest);
        }
        let _before = documents.insert(
            id.clone(),
            Document {
                slot: slot.to_owned(),
                body,
                used: now,
            },
        );
        Ok(id)
    }

    /// The document `id`, if it is kept; its idle time starts again.
    pub(crate) fn show(&self, id: &str, now: Instant) -> Option<Bytes> {
        let mut documents = self
            .documents
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        documents.retain(|_, kept| !expired(kept, now));
        let kept = documents.get_mut(id)?;
        kept.used = now;
        Some(kept.body.clone())
    }
}

fn expired(document: &Document, now: Instant) -> bool {
    now.saturating_duration_since(document.used) > IDLE
}

fn total(documents: &HashMap<String, Document>) -> usize {
    documents
        .values()
        .fold(0_usize, |sum, kept| sum.saturating_add(kept.body.len()))
}

/// 128 random bits as 32 lowercase hex digits.
fn fresh_id() -> Option<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().fold(String::new(), |mut hex, byte| {
        let _written = write!(hex, "{byte:02x}");
        hex
    }))
}

/// A panel's name: 1 to 64 of `[A-Za-z0-9_-]`.
fn slot(headers: &HeaderMap) -> Option<&str> {
    let slot = headers.get(SLOT)?.to_str().ok()?;
    let fits = (1..=64).contains(&slot.len())
        && slot
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    fits.then_some(slot)
}

/// What `POST /api/preview` answers.
#[derive(Debug, Serialize)]
struct Placed {
    /// Where the document is framed from.
    url: String,
}

/// `POST /api/preview`: keep the page's document, answer its address.
///
/// The session key and the panel are checked before the body is read, so a
/// caller without the key cannot make the server take in 64 MiB.
pub(crate) async fn put<S: NodeSource>(State(ui): State<Arc<Ui<S>>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    if !authorized(&ui.auth, &parts.headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(slot) = slot(&parts.headers) else {
        return (StatusCode::BAD_REQUEST, "no preview slot").into_response();
    };
    let declared = parts
        .headers
        .get(CONTENT_LENGTH)
        .and_then(|length| length.to_str().ok())
        .and_then(|length| length.parse::<usize>().ok());
    if declared.is_some_and(|length| length > MAX_BYTES) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    // Past the cap without a declared length, or the body broke off.
    let Ok(body) = to_bytes(body, MAX_BYTES).await else {
        return (StatusCode::BAD_REQUEST, "the document was not read").into_response();
    };
    if std::str::from_utf8(&body).is_err() {
        return (StatusCode::BAD_REQUEST, "the document is not UTF-8").into_response();
    }
    match ui.previews.put(slot, body, Instant::now()) {
        Ok(id) => Json(Placed {
            url: format!("/preview/{id}"),
        })
        .into_response(),
        Err(Refused::TooLarge) => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(Refused::NoId) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// `GET /preview/<id>`: the document under its own policy, or 404.
pub(crate) async fn show<S: NodeSource>(
    State(ui): State<Arc<Ui<S>>>,
    Path(id): Path<String>,
) -> Response {
    let Some(body) = ui.previews.show(&id, Instant::now()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = body.into_response();
    let headers = response.headers_mut();
    let _previous = headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    let _previous = headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(POLICY));
    let _previous = headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    let _previous = headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    let _previous = response.extensions_mut().insert(PreviewDocument);
    response
}

#[cfg(test)]
mod tests;
