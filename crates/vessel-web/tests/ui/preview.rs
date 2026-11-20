//! The HTML preview route (ADR 0122): a document the page posts with its
//! session key, framed from `/preview/<id>` under Paseo's sandboxed policy.

use crate::server::{get, http};
use crate::support::{Source, Ui};

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(header, _)| header == name)
        .map(|(_, value)| value.as_str())
}

fn post(host: &str, extra: &str, body: &str) -> String {
    format!(
        "POST /api/preview HTTP/1.1\r\nHost: {host}\r\n{extra}Content-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

const DOCUMENT: &str = "<!doctype html><p id=x></p><script>x.textContent='ran'</script>";

#[tokio::test]
async fn a_posted_document_is_framed_under_its_own_sandboxed_policy() {
    let ui = Ui::start(Source::default()).await;
    let _tab = ui.tab().await;
    let bearer = format!(
        "Authorization: Bearer {}\r\nOrigin: {}\r\n",
        ui.session.lock().await.as_ref().unwrap(),
        ui.origin()
    );
    let slot = "X-Vessel-Preview-Slot: panel-1\r\n";
    // Without the session key, or from another origin: refused.
    let (code, _headers, _body) = http(
        ui.address,
        &post(
            &ui.host(),
            &format!("Origin: {}\r\n{slot}", ui.origin()),
            DOCUMENT,
        ),
    )
    .await;
    assert_eq!(code, 401);
    let elsewhere = format!(
        "Authorization: Bearer {}\r\nOrigin: http://evil.example\r\n{slot}",
        ui.session.lock().await.as_ref().unwrap()
    );
    let (code, _headers, _body) = http(ui.address, &post(&ui.host(), &elsewhere, DOCUMENT)).await;
    assert_eq!(code, 403);
    // No panel named: refused.
    let (code, _headers, _body) = http(ui.address, &post(&ui.host(), &bearer, DOCUMENT)).await;
    assert_eq!(code, 400);

    let (code, headers, body) = http(
        ui.address,
        &post(&ui.host(), &format!("{bearer}{slot}"), DOCUMENT),
    )
    .await;
    assert_eq!(code, 200, "{body}");
    assert_eq!(header(&headers, "content-type"), Some("application/json"));
    let placed: serde_json::Value = serde_json::from_str(&body).unwrap();
    let url = placed
        .get("url")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    let id = url.strip_prefix("/preview/").unwrap();
    assert_eq!(id.len(), 32);
    assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()));

    // The frame's load carries no header: the id alone opens it.
    let (code, headers, body) = http(ui.address, &get(url, &ui.host(), "")).await;
    assert_eq!(code, 200);
    assert_eq!(body, DOCUMENT);
    assert_eq!(
        header(&headers, "content-type"),
        Some("text/html; charset=utf-8")
    );
    let csp = header(&headers, "content-security-policy").unwrap();
    for directive in [
        "default-src 'none'",
        "script-src 'unsafe-inline' 'unsafe-eval' blob:",
        "connect-src 'none'",
        "form-action 'none'",
        // Opaque even when opened outside the frame.
        "sandbox allow-scripts",
        "frame-ancestors 'self'",
    ] {
        assert!(csp.contains(directive), "{directive} in {csp}");
    }
    assert!(!csp.contains("script-src 'self'"), "{csp}");
    assert_eq!(header(&headers, "x-frame-options"), Some("SAMEORIGIN"));
    assert_eq!(header(&headers, "referrer-policy"), Some("no-referrer"));
    assert_eq!(header(&headers, "x-content-type-options"), Some("nosniff"));
    assert_eq!(header(&headers, "cache-control"), Some("no-store"));
    assert!(header(&headers, "set-cookie").is_none());
    assert!(header(&headers, "access-control-allow-origin").is_none());

    // The panel's next document replaces it.
    let (code, _headers, body) = http(
        ui.address,
        &post(&ui.host(), &format!("{bearer}{slot}"), "<p>next</p>"),
    )
    .await;
    assert_eq!(code, 200, "{body}");
    let (code, headers, _body) = http(ui.address, &get(url, &ui.host(), "")).await;
    assert_eq!(code, 404);
    // A missing document is an ordinary answer, under the page's policy.
    let csp = header(&headers, "content-security-policy").unwrap();
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert_eq!(header(&headers, "x-frame-options"), Some("DENY"));
}
