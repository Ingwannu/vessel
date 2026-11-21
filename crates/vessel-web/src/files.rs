//! The web UI's files, embedded at build time (`build.rs`), and how they
//! are served.
//!
//! The app routes in the browser, so a path that names no file is answered
//! with `index.html` when it is one of the app's routes ([`APP_ROUTES`]:
//! `/fleet/gpu.box` is a page even though a node name may hold a dot) or
//! has no extension; any other path with an extension that is not there
//! (`/missing.js`) is a `404`, so a broken bundle shows as a missing file
//! rather than as a page.
//! `index.html` is never cached, so a new build is picked up at once;
//! files whose names carry their content's hash are cached for a year as
//! immutable; anything else revalidates by its content `ETag`. Nothing here
//! touches the disk: the table is all there is.
//!
//! [Decision Log] Immutable hashed assets (milestone 10, track E)
//! - Purpose: the page preloads a 2 MB font and loads its script and
//!   stylesheet from `assets/`. With `no-cache` every visit asks for each
//!   again (a `304`, but a round trip each); through a tunnel or a slow
//!   link the page then draws its fallback face first on every visit.
//! - Constraint: the poisoned-cache defence (ADR 0017, `web/e2e/
//!   security.spec.ts`). A process that held this port earlier may have
//!   left an attacker's file in the browser's cache under one of our
//!   asset paths, itself marked immutable. Before this change assets were
//!   `no-cache` "not immutable" for that reason.
//! - What protects against it is not this header: our headers only
//!   describe our own responses, and cannot evict or revalidate an entry
//!   another server wrote (its own `immutable` keeps the browser from
//!   asking us at all). The defences are the index's SRI hashes on every
//!   script and stylesheet (a poisoned one is refused, not run) and a
//!   fresh random origin per launch (nothing can be cached for it before
//!   it exists). Marking our own hashed files immutable gives an earlier
//!   squatter nothing new: what we cache under a hashed path is the file
//!   that hash names, and the index that references it is `no-store`.
//! - Alternatives: keep `no-cache` (a round trip per asset per visit);
//!   `max-age` without `immutable` (Firefox still revalidates on reload);
//!   a service worker cache (the per-launch origin rules it out, and it
//!   is what the fresh origin defends against).
//! - Choice: `public, max-age=31536000, immutable` for files that
//!   `build.rs` recognises as content-hashed (`assets/<name>-<hash>`);
//!   the `ETag` stays on them for clients that revalidate anyway.
//! - Cost: a hashed file served wrong by a broken build stays cached for
//!   that origin; a new build gives new names, and each launch a new
//!   origin, so it cannot outlive the build.
//!
//! Compressed responses (milestone 11, `docs/PERFORMANCE.md`): a text
//! file the build compressed is answered in the encoding `encoding`
//! chooses from the request's `Accept-Encoding`, with `Content-Encoding`,
//! `Vary: Accept-Encoding` on every answer for it (so a cache never hands
//! one client's encoding to another), and an entity tag of its own per
//! encoding (`"<sha256>-br"`): a strong tag names one representation's
//! bytes, and a `304` must only confirm the bytes the client holds. The
//! index is the exception: it is painted per request (`paint`), so it is
//! sent uncompressed (`index` below says why).

use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH, VARY,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse as _, Response};

use crate::encoding::{self, Encoding};

/// One embedded file.
#[derive(Debug)]
pub(crate) struct File {
    /// Its URL path, from `/`.
    pub(crate) path: &'static str,
    /// Its `Content-Type`.
    pub(crate) kind: &'static str,
    /// Its content.
    pub(crate) etag: &'static str,
    /// Content bytes.
    pub(crate) bytes: &'static [u8],
    /// Whether its name carries its content's hash, so a changed file
    /// never comes under the same path (`build.rs`'s `hashed`).
    pub(crate) hashed: bool,
    /// Its gzip form, when the build made one.
    pub(crate) gzip: Option<Variant>,
    /// Its brotli form, when the build made one.
    pub(crate) brotli: Option<Variant>,
}

/// A compressed form of a file.
#[derive(Debug)]
pub(crate) struct Variant {
    /// The compressed bytes.
    pub(crate) bytes: &'static [u8],
    /// Its entity tag: the file's, marked with the encoding.
    pub(crate) etag: &'static str,
}

impl File {
    /// The bytes and entity tag to send in `encoding`.
    fn form(&self, encoding: Encoding) -> (&'static [u8], &'static str) {
        let variant = match encoding {
            Encoding::Identity => None,
            Encoding::Gzip => self.gzip.as_ref(),
            Encoding::Brotli => self.brotli.as_ref(),
        };
        variant.map_or((self.bytes, self.etag), |variant| {
            (variant.bytes, variant.etag)
        })
    }
}

/// `index.html`: never stored, so a new build is used at once.
const INDEX_CACHE: &str = "no-store";
/// A file named by its content's hash: the same path is the same bytes.
const HASHED_CACHE: &str = "public, max-age=31536000, immutable";
/// Anything else: kept, but asked about (by `ETag`) before each use.
const OTHER_CACHE: &str = "no-cache";

include!(concat!(env!("OUT_DIR"), "/files.rs"));

/// The first segments of the app's routes (`web/src/app.tsx`, and `/kit`
/// in `web/src/main.tsx`; keep the two in step). Later segments are node
/// names and session ids, which may hold dots, so the extension rule
/// cannot tell them from files. `plugin` is a plugin's screen
/// (`/plugin/<node>/<plugin>/<screen>`, ADR 0115 "Surfaces").
const APP_ROUTES: [&str; 9] = [
    "fleet", "new", "sessions", "schedules", "sync", "devices", "settings", "kit", "plugin",
];

/// Whether `path` lies under one of the app's routes.
fn is_app_route(path: &str) -> bool {
    let first = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();
    APP_ROUTES.contains(&first)
}

/// The file at `path`, if embedded (`FILES` is sorted by path).
pub(crate) fn find(path: &str) -> Option<&'static File> {
    FILES
        .binary_search_by(|file| file.path.cmp(path))
        .ok()
        .and_then(|index| FILES.get(index))
}

/// Answer a request for a file; the index carries `paint` (`paint`).
pub(crate) fn serve(
    method: &Method,
    uri: &Uri,
    request: &HeaderMap,
    paint: crate::paint::Paint,
) -> Response {
    if *method != Method::GET && *method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let path = uri.path();
    if path.starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let last = path.rsplit('/').next().unwrap_or_default();
    let file = match find(path) {
        Some(file) => file,
        None if path == "/" || is_app_route(path) || !last.contains('.') => {
            match find("/index.html") {
                Some(index) => index,
                None => return StatusCode::NOT_FOUND.into_response(),
            }
        }
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    if file.path == "/index.html" {
        index(file, method, paint)
    } else {
        respond(file, method, request)
    }
}

/// The index: painted for this request (`paint`), so it is sent as it is,
/// never compressed, never `304`.
///
/// [Decision Log] (milestone 11: tracks P and W together)
/// - Purpose: track W writes the page's appearance into the index per
///   request (`paint`, ADR 0028); track P compresses every text file once
///   at build. A form compressed at build cannot carry a value chosen per
///   request.
/// - Alternatives: compressing the painted index per request (a brotli or
///   gzip encoder in this crate, a new dependency, for about 1.5 KB);
///   sending the index compressed and letting the page set the attributes
///   from a value in the document (a script, which runs after the
///   stylesheet may have painted: the flash `paint` exists to remove).
/// - Choice: the index is the one text file sent uncompressed: the web
///   build does not compress it (`web/scripts/compress.mjs`), and this
///   answer ignores a form if one were embedded. It has one representation,
///   so no `Vary`; it is `no-store` and has no entity tag, as before.
/// - Cost: the index's bytes on the wire as built (`docs/PERFORMANCE.md`,
///   "Tracks P and W merged").
fn index(file: &File, method: &Method, paint: crate::paint::Paint) -> Response {
    let mut response = if *method == Method::HEAD {
        StatusCode::OK.into_response()
    } else {
        crate::paint::index(file.bytes, paint).into_response()
    };
    let headers = response.headers_mut();
    let _previous = headers.insert(CONTENT_TYPE, HeaderValue::from_static(file.kind));
    let _previous = headers.insert(CACHE_CONTROL, HeaderValue::from_static(INDEX_CACHE));
    response
}

/// Any file but the index, as the answer to a `GET` or `HEAD` with
/// `request`'s headers, in the encoding it accepts.
fn respond(file: &File, method: &Method, request: &HeaderMap) -> Response {
    let cache = cache_control(file);
    let accept = request
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok());
    let encoding = encoding::choose(accept, file.gzip.is_some(), file.brotli.is_some());
    let (bytes, etag) = file.form(encoding);
    let unchanged = request
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|tag| tag.trim().trim_start_matches("W/") == etag)
        });
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else if *method == Method::HEAD {
        StatusCode::OK.into_response()
    } else {
        bytes.into_response()
    };
    let headers = response.headers_mut();
    let _previous = headers.insert(CONTENT_TYPE, HeaderValue::from_static(file.kind));
    let _previous = headers.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    let _previous = headers.insert(ETAG, HeaderValue::from_static(etag));
    if file.gzip.is_some() || file.brotli.is_some() {
        let _previous = headers.insert(VARY, HeaderValue::from_static("accept-encoding"));
    }
    if let Some(token) = encoding.token() {
        let _previous = headers.insert(CONTENT_ENCODING, HeaderValue::from_static(token));
    }
    response
}

/// How long the browser may keep `file`.
fn cache_control(file: &File) -> &'static str {
    if file.path == "/index.html" {
        INDEX_CACHE
    } else if file.hashed {
        HASHED_CACHE
    } else {
        OTHER_CACHE
    }
}

/// The embedded `index.html` as text, for tests of what it names.
#[cfg(test)]
pub(crate) fn index_text() -> &'static str {
    find("/index.html")
        .and_then(|index| std::str::from_utf8(index.bytes).ok())
        .unwrap_or_default()
}

/// Whether the real bundle is embedded (else the placeholder page that
/// says how to build it).
pub(crate) const fn bundled() -> bool {
    BUNDLED
}

#[cfg(test)]
mod tests;
