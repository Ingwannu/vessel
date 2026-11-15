//! HTTP proxy decoding: canonical destination, hop headers and origin-form path.
//!
//! This is the sole HTTP spelling edge; downstream code receives BrowserUrl
//! rather than branching on page-selected strings or trusting Host headers.

use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Uri, header};
use vessel_proto::browser::BrowserUrl;

/// Bound page-controlled Connection token processing independently of header bytes.
const MAX_CONNECTION_TOKENS: usize = 32;

#[derive(Debug, thiserror::Error)]
pub(super) enum InvalidRequest {
    #[error("browser proxy request form is invalid")]
    Form,
    #[error("browser proxy URL is invalid")]
    Url(#[source] vessel_proto::browser::InvalidBrowserUrl),
    #[error("browser proxy path is invalid")]
    Uri(#[source] axum::http::uri::InvalidUri),
    #[error("browser proxy header text is invalid")]
    HeaderText(#[source] axum::http::header::ToStrError),
    #[error("browser proxy header name is invalid")]
    HeaderName(#[source] axum::http::header::InvalidHeaderName),
    #[error("browser proxy header value is invalid")]
    HeaderValue(#[source] axum::http::header::InvalidHeaderValue),
}

pub(super) struct Destination {
    pub(super) url: BrowserUrl,
    pub(super) mode: RequestMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestMode {
    Http,
    WebSocket,
    Connect,
}

pub(super) fn destination<B>(request: &Request<B>) -> Result<Destination, InvalidRequest> {
    if request.method() == Method::CONNECT {
        let uri = request.uri();
        if uri.scheme().is_some() || uri.path_and_query().is_some() {
            return Err(InvalidRequest::Form);
        }
        let authority = uri.authority().ok_or(InvalidRequest::Form)?;
        let url = format!("https://{authority}/")
            .parse()
            .map_err(InvalidRequest::Url)?;
        return Ok(Destination {
            url,
            mode: RequestMode::Connect,
        });
    }
    // A proxy accepts absolute-form HTTP (and WebSocket's equivalent),
    // never origin-form plus an unchecked Host or plaintext HTTPS forwarding.
    let input = request.uri().to_string();
    let input = input
        .strip_prefix("ws://")
        .map_or_else(|| input.clone(), |tail| format!("http://{tail}"));
    let url: BrowserUrl = input.parse().map_err(InvalidRequest::Url)?;
    if url.parsed().scheme() != "http" {
        return Err(InvalidRequest::Form);
    }
    let mode = match request.headers().get(header::UPGRADE) {
        Some(value) if value.as_bytes().eq_ignore_ascii_case(b"websocket") => {
            RequestMode::WebSocket
        }
        Some(_) => return Err(InvalidRequest::Form),
        None => RequestMode::Http,
    };
    Ok(Destination { url, mode })
}

pub(super) fn prepare<B>(
    request: &mut Request<B>,
    destination: &Destination,
) -> Result<(), InvalidRequest> {
    let url = destination.url.parsed();
    let path = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };
    *request.uri_mut() = path.parse::<Uri>().map_err(InvalidRequest::Uri)?;
    let host = url.host_str().ok_or(InvalidRequest::Form)?;
    let authority = url
        .port()
        .map_or_else(|| host.to_owned(), |port| format!("{host}:{port}"));
    sanitize(request.headers_mut(), destination.mode)?;
    let _old = request.headers_mut().insert(
        header::HOST,
        HeaderValue::from_str(&authority).map_err(InvalidRequest::HeaderValue)?,
    );
    Ok(())
}

pub(super) fn sanitize(headers: &mut HeaderMap, mode: RequestMode) -> Result<(), InvalidRequest> {
    let mut named = Vec::new();
    for value in headers.get_all(header::CONNECTION) {
        for token in value
            .to_str()
            .map_err(InvalidRequest::HeaderText)?
            .split(',')
        {
            if named.len() >= MAX_CONNECTION_TOKENS {
                return Err(InvalidRequest::Form);
            }
            let name = HeaderName::from_bytes(token.trim().as_bytes())
                .map_err(InvalidRequest::HeaderName)?;
            named.push(name);
        }
    }
    for name in named {
        let _old = headers.remove(name);
    }
    for name in [
        header::CONNECTION,
        header::PROXY_AUTHORIZATION,
        header::PROXY_AUTHENTICATE,
        header::TE,
        header::TRAILER,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
        HeaderName::from_static("keep-alive"),
        HeaderName::from_static("proxy-connection"),
    ] {
        let _old = headers.remove(name);
    }
    if mode == RequestMode::WebSocket {
        let _old = headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
        let _old = headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_and_absolute_form_are_distinct_and_host_is_rewritten() {
        for invalid in [
            "/private",
            "https://example.test/path",
            "http://person:secret@example.test/",
        ] {
            let request = Request::builder().uri(invalid).body(()).unwrap();
            assert!(destination(&request).is_err());
        }
        let mut request = Request::builder()
            .uri("http://example.test:8080/a?b=1")
            .header(header::HOST, "different.test")
            .body(())
            .unwrap();
        let selected = destination(&request).unwrap();
        prepare(&mut request, &selected).unwrap();
        assert_eq!(request.uri(), "/a?b=1");
        assert_eq!(
            request.headers().get(header::HOST),
            Some(&HeaderValue::from_static("example.test:8080"))
        );
        let request = Request::builder()
            .method(Method::CONNECT)
            .uri("example.test:443")
            .body(())
            .unwrap();
        let selected = destination(&request).unwrap();
        assert_eq!(selected.mode, RequestMode::Connect);
        assert_eq!(selected.url.as_str(), "https://example.test/");
    }

    #[test]
    fn connection_named_headers_and_proxy_credentials_are_never_forwarded() {
        let mut headers = HeaderMap::new();
        let _old = headers.insert(
            header::CONNECTION,
            HeaderValue::from_static("x-internal, keep-alive"),
        );
        let _old = headers.insert("x-internal", HeaderValue::from_static("canary"));
        let _old = headers.insert(
            header::PROXY_AUTHORIZATION,
            HeaderValue::from_static("Basic canary"),
        );
        sanitize(&mut headers, RequestMode::Http).unwrap();
        assert!(headers.is_empty());
        sanitize(&mut headers, RequestMode::WebSocket).unwrap();
        assert_eq!(
            headers.get(header::UPGRADE),
            Some(&HeaderValue::from_static("websocket"))
        );
    }
}
