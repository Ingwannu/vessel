//! The tests of `files`: the embedded table, the caching rule, and the
//! encodings each request gets (the painted index's included), apart from
//! the module so it stays the size of what it does.

mod table {
    use super::super::*;

    #[test]
    fn the_table_is_sorted_and_has_an_index() {
        assert!(FILES.windows(2).all(|pair| pair[0].path < pair[1].path));
        let index = find("/index.html").unwrap();
        assert_eq!(index.kind, "text/html; charset=utf-8");
        assert!(FILES.iter().all(|file| file.path.starts_with('/')));
    }
}

mod cache {
    use super::super::*;
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

mod encodings {
    use super::super::*;

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

mod routes {
    use super::super::*;
    use crate::paint::Paint;

    #[test]
    fn app_routes_with_dotted_names_are_pages_and_missing_files_are_not() {
        for page in [
            "/fleet/gpu.box",
            "/sessions/gpu.box/3f2a",
            "/fleet",
            "/devices",
            "/settings/appearance",
            "/kit",
            "/plugin/gpu.box/prefs/main",
        ] {
            let response = serve(
                &Method::GET,
                &page.parse().unwrap(),
                &HeaderMap::new(),
                Paint::default(),
            );
            assert_eq!(response.status(), StatusCode::OK, "{page}");
            assert_eq!(
                response.headers().get(CACHE_CONTROL).unwrap(),
                INDEX_CACHE,
                "{page}"
            );
        }
        for missing in ["/missing.js", "/assets/gone.css", "/fleetx/a.js"] {
            let response = serve(
                &Method::GET,
                &missing.parse().unwrap(),
                &HeaderMap::new(),
                Paint::default(),
            );
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{missing}");
        }
    }
}
