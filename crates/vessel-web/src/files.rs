//! The web UI's files, embedded at build time (`build.rs`), and how they
//! are served.
//!
//! The app routes in the browser, so a path that names no file and has no
//! extension (`/nodes/gpu-box/sessions`) is answered with `index.html`;
//! one with an extension that is not there (`/missing.js`) is a `404`, so a
//! broken bundle shows as a missing file rather than as a page.
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

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse as _, Response};

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
}

/// `index.html`: never stored, so a new build is used at once.
const INDEX_CACHE: &str = "no-store";
/// A file named by its content's hash: the same path is the same bytes.
const HASHED_CACHE: &str = "public, max-age=31536000, immutable";
/// Anything else: kept, but asked about (by `ETag`) before each use.
const OTHER_CACHE: &str = "no-cache";

include!(concat!(env!("OUT_DIR"), "/files.rs"));

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
        None if path == "/" || !last.contains('.') => match find("/index.html") {
            Some(index) => index,
            None => return StatusCode::NOT_FOUND.into_response(),
        },
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let index = file.path == "/index.html";
    let cache = cache_control(file);
    let unchanged = !index
        && request
            .get(IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value
                    .split(',')
                    .any(|tag| tag.trim().trim_start_matches("W/") == file.etag)
            });
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else if *method == Method::HEAD {
        StatusCode::OK.into_response()
    } else if index {
        crate::paint::index(file.bytes, paint).into_response()
    } else {
        file.bytes.into_response()
    };
    let headers = response.headers_mut();
    let _previous = headers.insert(CONTENT_TYPE, HeaderValue::from_static(file.kind));
    let _previous = headers.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    if !index {
        let _previous = headers.insert(ETAG, HeaderValue::from_static(file.etag));
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

/// Whether the real bundle is embedded (else the placeholder page that
/// says how to build it).
pub(crate) const fn bundled() -> bool {
    BUNDLED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_has_an_index() {
        assert!(FILES.windows(2).all(|pair| pair[0].path < pair[1].path));
        let index = find("/index.html").unwrap();
        assert_eq!(index.kind, "text/html; charset=utf-8");
        assert!(FILES.iter().all(|file| file.path.starts_with('/')));
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::paint::Paint;

    /// Each file's `Cache-Control`, and its `ETag` answered with a `304`.
    /// Whether the table holds a hashed file depends on the bundle (the
    /// placeholder has none), so the rule is checked on every file there
    /// is, and on names in `hashed`'s own tests.
    #[tokio::test]
    async fn hashed_files_are_immutable_others_revalidate_and_index_is_never_stored() {
        for file in FILES.iter().filter(|file| file.path != "/index.html") {
            let uri = file.path.parse().unwrap();
            let response = serve(&Method::GET, &uri, &HeaderMap::new(), Paint::default());
            let expected = if file.hashed {
                HASHED_CACHE
            } else {
                OTHER_CACHE
            };
            assert_eq!(
                response.headers().get(CACHE_CONTROL).unwrap(),
                expected,
                "{}",
                file.path
            );
            assert_eq!(response.headers().get(ETAG).unwrap(), file.etag);
            let mut request = HeaderMap::new();
            let _previous = request.insert(IF_NONE_MATCH, HeaderValue::from_static(file.etag));
            let response = serve(&Method::GET, &uri, &request, Paint::default());
            assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        }
        let index = find("/index.html").unwrap();
        assert!(!index.hashed);
        let mut request = HeaderMap::new();
        let _previous = request.insert(IF_NONE_MATCH, HeaderValue::from_static(index.etag));
        for path in ["/", "/index.html", "/nodes/box/sessions"] {
            let uri = path.parse().unwrap();
            let response = serve(&Method::GET, &uri, &request, Paint::default());
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), INDEX_CACHE);
            assert!(response.headers().get(ETAG).is_none());
        }
    }

    /// The rule itself, whatever the embedded bundle holds.
    #[test]
    fn the_cache_follows_the_file() {
        let file = |path, hashed| File {
            path,
            kind: "text/javascript; charset=utf-8",
            etag: "\"0\"",
            bytes: b"",
            hashed,
        };
        assert_eq!(
            cache_control(&file("/assets/index-B3kq_x9Z.js", true)),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(cache_control(&file("/licenses/a.txt", false)), "no-cache");
        assert_eq!(cache_control(&file("/index.html", false)), "no-store");
    }
}
