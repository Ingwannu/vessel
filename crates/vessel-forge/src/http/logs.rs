//! A job's log tail (ADR 0088, part 4): the one exception to this client's
//! same-origin rule.
//!
//! [Decision Log]
//! - Purpose: "Add logs to chat" sends a failed job's last lines (Paseo's
//!   `getCheckDetails`, 200 lines or 16 KiB).
//! - Constraint: GitHub answers `…/actions/jobs/{id}/logs` with a 302 to
//!   signed blob storage on another origin; the token must never go
//!   there, and a log may be hundreds of megabytes.
//! - Choice: the API request goes through [`Http::exchange`] (token, slots,
//!   limits), which keeps a redirect's `Location` instead of following
//!   it. The node then makes exactly ONE request to that address, only if
//!   it is `https`, with the separate `bare` client: no `Authorization`,
//!   no further redirect, its own [`TIME`]. It asks the last
//!   [`TAIL`] bytes (`Range: bytes=-16384`); a server that ignores the
//!   range is read through with a rolling tail and stopped at
//!   [`READ_MAX`]. GitHub Enterprise Server may answer the log itself
//!   (200): read as any answer, at most 4 MiB.
//! - Must stay true: the signed address is never logged, kept or sent to
//!   a client (errors are made without their URL).
//! - Trust boundary (ADR 0088): the address comes from the configured
//!   forge host, which is trusted; a `localhost` name or a literal address
//!   in a loopback, private, link-local, unique-local, shared or
//!   unspecified range is refused before any request ([`public_host`]).
//!   A name that only resolves to such an address is not caught here (a
//!   stated gap; DNS-level filtering would ride ADR 0104's resolver).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{CONTENT_RANGE, RANGE};

use super::{Call, Http, Reply};
use crate::provider::ForgeError;

/// The log request's own time.
pub(super) const TIME: Duration = Duration::from_secs(20);
/// The bytes of a log kept: the tail a log's last 200 lines are cut from.
const TAIL: usize = 16 * 1024;
/// The most read of a log whose server ignored the range.
const READ_MAX: usize = 32 * 1024 * 1024;

/// The end of a log as read, and whether bytes before it were left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tail {
    pub(crate) bytes: Vec<u8>,
    pub(crate) cut: bool,
}

impl Http {
    /// The tail of the log `call` (GitHub's `…/actions/jobs/{id}/logs`)
    /// points to.
    ///
    /// # Errors
    /// [`ForgeError::NotFound`] when GitHub keeps no log (expired);
    /// [`ForgeError::Unexpected`] for a redirect that is not one `https`
    /// address; the errors of [`Http::send`] and of the one request.
    pub(crate) async fn log_tail(&self, call: Call<'_>) -> Result<Tail, ForgeError> {
        let answered = self.exchange(call).await?;
        let status = match answered.reply {
            Reply::Body { status, body } if status.is_success() => {
                return Ok(tail_of(&body, false));
            }
            Reply::Body { status, .. } => status,
            Reply::NotModified => StatusCode::NOT_MODIFIED,
        };
        if matches!(status.as_u16(), 404 | 410) {
            return Err(ForgeError::NotFound);
        }
        let Some(location) = answered.location.filter(|_| status.is_redirection()) else {
            return Err(ForgeError::Unexpected {
                what: "a job's log",
            });
        };
        let url = reqwest::Url::parse(&location)
            .ok()
            .filter(|url| url.scheme() == "https" && public_host(url))
            .ok_or(ForgeError::Unexpected {
                what: "a job's log address (https, a public host)",
            })?;
        self.signed(url).await
    }

    /// The one request to the signed address, without the token.
    async fn signed(&self, url: reqwest::Url) -> Result<Tail, ForgeError> {
        let mut response = self
            .bare
            .get(url)
            .header(RANGE, format!("bytes=-{TAIL}"))
            .send()
            .await
            .map_err(unreachable)?;
        let status = response.status();
        if status == StatusCode::PARTIAL_CONTENT {
            let total = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(range_total);
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(unreachable)? {
                bytes.extend_from_slice(&chunk);
                if bytes.len() > TAIL {
                    break;
                }
            }
            let held = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            let cut = total.is_none_or(|total| total > held);
            return Ok(tail_of(&bytes, cut));
        }
        if !status.is_success() {
            return Err(if status.is_server_error() {
                ForgeError::Server {
                    status: status.as_u16(),
                }
            } else {
                ForgeError::Unexpected {
                    what: "a job's log",
                }
            });
        }
        // The whole log: keep a rolling tail, stop past `READ_MAX`.
        let mut kept: Vec<u8> = Vec::new();
        let mut read: usize = 0;
        let mut stopped = false;
        while let Some(chunk) = response.chunk().await.map_err(unreachable)? {
            read = read.saturating_add(chunk.len());
            kept.extend_from_slice(&chunk);
            if kept.len() > TAIL.saturating_mul(2) {
                let _old = kept.drain(..kept.len().saturating_sub(TAIL));
            }
            if read > READ_MAX {
                stopped = true;
                break;
            }
        }
        let cut = stopped || read > kept.len();
        Ok(tail_of(&kept, cut))
    }
}

/// A failed read, without the signed address in it.
fn unreachable(error: reqwest::Error) -> ForgeError {
    ForgeError::Unreachable(error.without_url())
}

/// The last [`TAIL`] bytes of `bytes`; `cut` when anything before them is
/// missing.
/// Whether `url`'s host may be asked for a log: not `localhost` (or a
/// name under it) and not a literal address outside the public internet.
fn public_host(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    match bare.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => public_v4(address),
        Ok(IpAddr::V6(address)) => address
            .to_ipv4_mapped()
            .map_or_else(|| public_v6(address), public_v4),
        Err(_) => {
            let name = bare.trim_end_matches('.').to_ascii_lowercase();
            name != "localhost" && !name.ends_with(".localhost")
        }
    }
}

fn public_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    // 100.64.0.0/10, carrier-grade shared space.
    let shared = first == 100 && (second & 0xc0) == 64;
    !(address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_broadcast()
        || shared)
}

fn public_v6(address: Ipv6Addr) -> bool {
    let [first, ..] = address.segments();
    // fc00::/7 unique local, fe80::/10 link local.
    let unique_local = (first & 0xfe00) == 0xfc00;
    let link_local = (first & 0xffc0) == 0xfe80;
    !(address.is_loopback() || address.is_unspecified() || unique_local || link_local)
}

fn tail_of(bytes: &[u8], cut: bool) -> Tail {
    let start = bytes.len().saturating_sub(TAIL);
    Tail {
        bytes: bytes.get(start..).unwrap_or_default().to_vec(),
        cut: cut || start > 0,
    }
}

/// The whole length in `bytes 100-200/5000`; `None` for `*` or a bad one.
fn range_total(value: &str) -> Option<u64> {
    value.rsplit_once('/')?.1.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::http::HeaderMap;
    use axum::response::IntoResponse as _;

    use super::*;

    #[test]
    fn a_tail_keeps_the_last_sixteen_kib_and_says_it_cut() {
        let tail = tail_of(&vec![b'x'; TAIL + 5], false);
        assert_eq!(tail.bytes.len(), TAIL);
        assert!(tail.cut);
        assert!(!tail_of(b"short", false).cut);
        assert!(tail_of(b"short", true).cut);
    }

    /// A stand-in blob store on loopback: the last bytes for a suffix
    /// range (206), or the whole log when told to ignore ranges (200); it
    /// records whether a request carried a token.
    async fn store(log: Vec<u8>, ranges: bool) -> (reqwest::Url, Arc<Mutex<Vec<bool>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&seen);
        let app = Router::new().fallback(move |headers: HeaderMap| {
            let log = log.clone();
            let kept = Arc::clone(&kept);
            async move {
                kept.lock()
                    .unwrap()
                    .push(headers.contains_key("authorization"));
                let asked = headers.get("range").and_then(|value| value.to_str().ok());
                match asked.and_then(|range| range.strip_prefix("bytes=-")) {
                    Some(last) if ranges => {
                        let last: usize = last.parse().unwrap();
                        let start = log.len().saturating_sub(last);
                        let end = log.len().saturating_sub(1);
                        let range = format!("bytes {start}-{end}/{}", log.len());
                        (
                            StatusCode::PARTIAL_CONTENT,
                            [("content-range", range)],
                            log[start..].to_vec(),
                        )
                            .into_response()
                    }
                    _ => log.into_response(),
                }
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        }));
        (format!("http://{address}/signed").parse().unwrap(), seen)
    }

    #[tokio::test]
    async fn the_signed_address_gets_a_suffix_range_and_never_the_token() {
        let http = Http::new().unwrap();
        let log: Vec<u8> = b"abcdefghij\n"
            .iter()
            .copied()
            .cycle()
            .take(40_000)
            .collect();
        let (url, seen) = store(log.clone(), true).await;
        let tail = http.signed(url).await.unwrap();
        assert_eq!(tail.bytes, log[log.len() - TAIL..]);
        assert!(tail.cut);
        let (url, seen_whole) = store(log.clone(), false).await;
        let whole = http.signed(url).await.unwrap();
        assert_eq!(
            whole.bytes,
            log[log.len() - TAIL..],
            "a rolling tail of a whole log"
        );
        assert!(whole.cut);
        for seen in [seen, seen_whole] {
            assert_eq!(*seen.lock().unwrap(), [false], "one request, no token");
        }
    }

    #[test]
    fn a_log_address_on_a_private_or_local_host_is_refused() {
        let public = |address: &str| public_host(&address.parse().unwrap());
        assert!(public("https://productionresultssa1.blob.core.windows.net/log"));
        assert!(public("https://93.184.216.34/log"));
        assert!(public("https://[2606:4700::1111]/log"));
        for refused in [
            "https://localhost/log",
            "https://LOCALHOST./log",
            "https://api.localhost/log",
            "https://127.0.0.1/log",
            "https://10.1.2.3/log",
            "https://172.16.0.1/log",
            "https://192.168.1.1/log",
            "https://169.254.169.254/latest/meta-data",
            "https://100.64.0.1/log",
            "https://0.0.0.0/log",
            "https://[::1]/log",
            "https://[fd00::1]/log",
            "https://[fe80::1]/log",
            "https://[::ffff:127.0.0.1]/log",
        ] {
            assert!(!public(refused), "{refused}");
        }
    }

    #[test]
    fn a_content_range_names_its_whole_length() {
        assert_eq!(range_total("bytes 100-200/5000"), Some(5000));
        assert_eq!(range_total("bytes 0-10/*"), None);
        assert_eq!(range_total("nonsense"), None);
    }
}
