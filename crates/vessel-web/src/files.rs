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
            gzip: None,
            brotli: None,
        };
        assert_eq!(
            cache_control(&file("/assets/index-B3kq_x9Z.js", true)),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(cache_control(&file("/licenses/a.txt", false)), "no-cache");
        assert_eq!(cache_control(&file("/index.html", false)), "no-store");
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    /// A text file with both compressed forms, as the web build makes them.
    static SCRIPT: File = File {
        path: "/assets/index-B3kq_x9Z.js",
        kind: "text/javascript; charset=utf-8",
        etag: "\"abc\"",
        bytes: b"plain",
        hashed: true,
        gzip: Some(Variant {
            bytes: b"gz",
            etag: "\"abc-gz\"",
        }),
        brotli: Some(Variant {
            bytes: b"br",
            etag: "\"abc-br\"",
        }),
    };

    fn ask(file: &File, accept: Option<&'static str>, tag: Option<&'static str>) -> Response {
        let mut request = HeaderMap::new();
        if let Some(accept) = accept {
            let _previous = request.insert(ACCEPT_ENCODING, HeaderValue::from_static(accept));
        }
        if let Some(tag) = tag {
            let _previous = request.insert(IF_NONE_MATCH, HeaderValue::from_static(tag));
        }
        respond(file, &Method::GET, &request)
    }

    async fn body(response: Response) -> Vec<u8> {
        axum::body::to_bytes(response.into_body(), 1 << 16)
            .await
            .unwrap()
            .to_vec()
    }

    /// Each encoding is its own representation: its bytes, its
    /// `Content-Encoding`, its entity tag; every answer varies by
    /// `Accept-Encoding`.
    #[tokio::test]
    async fn a_request_gets_the_form_it_accepts_with_its_own_tag() {
        for (accept, encoding, tag, bytes) in [
            (
                Some("gzip, deflate, br, zstd"),
                Some("br"),
                "\"abc-br\"",
                &b"br"[..],
            ),
            (Some("gzip"), Some("gzip"), "\"abc-gz\"", &b"gz"[..]),
            (None, None, "\"abc\"", &b"plain"[..]),
        ] {
            let response = ask(&SCRIPT, accept, None);
            assert_eq!(response.status(), StatusCode::OK);
            let headers = response.headers();
            assert_eq!(
                headers
                    .get(CONTENT_ENCODING)
                    .map(|value| value.to_str().unwrap()),
                encoding
            );
            assert_eq!(headers.get(ETAG).unwrap(), tag);
            assert_eq!(headers.get(VARY).unwrap(), "accept-encoding");
            assert_eq!(body(response).await, bytes);
        }
    }

    /// A `304` confirms only the representation the client holds.
    #[tokio::test]
    async fn not_modified_only_for_the_tag_of_the_form_asked_for() {
        let brotli = Some("br");
        assert_eq!(
            ask(&SCRIPT, brotli, Some("\"abc-br\"")).status(),
            StatusCode::NOT_MODIFIED
        );
        assert_eq!(
            ask(&SCRIPT, brotli, Some("\"abc\"")).status(),
            StatusCode::OK
        );
        assert_eq!(
            ask(&SCRIPT, None, Some("\"abc-br\"")).status(),
            StatusCode::OK
        );
        assert_eq!(
            ask(&SCRIPT, None, Some("W/\"abc\"")).status(),
            StatusCode::NOT_MODIFIED
        );
    }

    /// The index is painted for each request, so it goes as it is, even to
    /// a client that accepts brotli, and its paint survives.
    #[tokio::test]
    async fn the_index_is_painted_and_never_compressed() {
        let mut request = HeaderMap::new();
        let _previous = request.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br, zstd"),
        );
        let dark = crate::paint::Paint::dark_for_tests();
        for path in ["/", "/index.html", "/sessions/box/s1"] {
            let uri = path.parse().unwrap();
            let response = serve(&Method::GET, &uri, &request, dark);
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let headers = response.headers();
            assert!(headers.get(CONTENT_ENCODING).is_none(), "{path}");
            assert!(headers.get(VARY).is_none(), "{path}");
            assert!(headers.get(ETAG).is_none(), "{path}");
            assert_eq!(headers.get(CACHE_CONTROL).unwrap(), INDEX_CACHE);
            let text = String::from_utf8(body(response).await).unwrap();
            if bundled() {
                assert!(
                    text.contains("<html lang=\"en\" data-theme=\"dark\""),
                    "{path}: {text}"
                );
            }
        }
        let index = find("/index.html").unwrap();
        assert!(
            index.gzip.is_none() && index.brotli.is_none(),
            "the web build must not compress the index (compress.mjs)"
        );
    }

    /// A file the build did not compress is served as it is, with no
    /// `Vary` (it has one representation).
    #[test]
    fn a_file_without_forms_has_one_representation() {
        let font = File {
            path: "/assets/geist-latin-wght-normal-BgDaEnEv.woff2",
            kind: "font/woff2",
            etag: "\"f\"",
            bytes: b"font",
            hashed: true,
            gzip: None,
            brotli: None,
        };
        let response = ask(&font, Some("br, gzip"), None);
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert!(response.headers().get(VARY).is_none());
    }
}
