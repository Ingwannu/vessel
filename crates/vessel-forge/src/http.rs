//! Requests to a forge's API: one client, its slots, conditional reads and
//! bounds (ADR 0029; the plan's "How the node watches"). A forge's limits
//! are `limits`'; which endpoint and what its answer means is the
//! provider's.
//!
//! [Decision Log]
//! - Purpose: every request any provider makes goes through here, so
//!   each gets the same bounds, the token only in its header, and the
//!   same reading of a forge's limits.
//! - Constraint: a 304 to a conditional request costs nothing; a 403
//!   means a rate limit or a refusal, told apart only by its headers and
//!   body (agent-orchestrator's trap).
//! - Choice: `reqwest` as `vessel-llm` builds it (rustls with `ring`, the
//!   platform's roots, no system proxy), redirects followed only to the
//!   same origin and at most three (a renamed repository answers 301), a
//!   connect and a whole-request timeout, bodies read bounded; 4 slots,
//!   one of them kept for a person; an `ETag` kept per URL, at most
//!   [`MAX_ETAGS`], the oldest dropped first.
//! - Must stay true: nothing here logs a header, a URL's query or a body
//!   (an error body may quote a token); the token is a sensitive header.

mod limits;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use tokio::sync::Semaphore;
use vessel_proto::forge::{ForgeHost, RefusedReason};

pub(crate) use self::limits::Resource;
use self::limits::{Admit, Hold, Limits, is_limit, sso};
use crate::provider::{ForgeError, Priority};
use crate::token::Token;

/// Requests at once per node.
const SLOTS: usize = 4;
/// Of those, the background may take this many: one stays a person's.
const BACKGROUND_SLOTS: usize = 3;
/// A request's whole time, and its connection's.
const TIME: Duration = Duration::from_secs(30);
const CONNECT: Duration = Duration::from_secs(10);
/// The most of a body read.
const MAX_BODY: usize = 4 * 1024 * 1024;
/// The longest the background waits for its bucket before giving up.
const BUCKET_WAIT: Duration = Duration::from_secs(2);
/// How many `ETag`s are kept: one per conditional URL.
const MAX_ETAGS: usize = 1024;

/// One request.
#[derive(Debug)]
pub(crate) struct Call<'a> {
    pub(crate) method: Method,
    pub(crate) url: reqwest::Url,
    pub(crate) host: &'a ForgeHost,
    pub(crate) token: &'a Token,
    pub(crate) resource: Resource,
    pub(crate) priority: Priority,
    /// Sent with `If-None-Match` of its last `ETag`.
    pub(crate) conditional: bool,
    /// A JSON body.
    pub(crate) json: Option<Vec<u8>>,
}

/// What a request answered.
#[derive(Debug)]
pub(crate) enum Reply {
    /// A body, with its status (2xx, or a 4xx the provider reads).
    Body { status: StatusCode, body: Vec<u8> },
    /// 304: nothing changed since the last look.
    NotModified,
}

#[derive(Debug, Default)]
struct State {
    limits: Limits,
    etags: HashMap<String, HeaderValue>,
    etag_order: VecDeque<String>,
}

/// The client every provider's request goes through.
#[derive(Debug, Clone)]
pub(crate) struct Http {
    client: reqwest::Client,
    slots: Arc<Semaphore>,
    background: Arc<Semaphore>,
    state: Arc<Mutex<State>>,
}

impl Http {
    /// A client trusting the platform's roots.
    pub(crate) fn new() -> Result<Self, ForgeError> {
        let found = rustls_native_certs::load_native_certs();
        let mut roots = rustls::RootCertStore::empty();
        let (_added, _ignored) = roots.add_parsable_certificates(found.certs);
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_refused| ForgeError::Unexpected { what: "TLS setup" })?
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::custom(same_origin))
            .no_proxy()
            .connect_timeout(CONNECT)
            .timeout(TIME)
            .user_agent("vessel")
            .build()
            .map_err(ForgeError::Unreachable)?;
        Ok(Self {
            client,
            slots: Arc::new(Semaphore::new(SLOTS)),
            background: Arc::new(Semaphore::new(BACKGROUND_SLOTS)),
            state: Arc::new(Mutex::new(State::default())),
        })
    }

    /// Send `call`, within every bound.
    ///
    /// # Errors
    /// [`ForgeError::RateLimited`] before sending when a cooldown or the
    /// background's floor holds; after it, for a limit answer.
    /// [`ForgeError::Refused`] for 401 and a 403 that is not a limit;
    /// [`ForgeError::Server`] for 5xx; [`ForgeError::Unreachable`].
    pub(crate) async fn send(&self, call: Call<'_>) -> Result<Reply, ForgeError> {
        self.admit(&call).await?;
        let _background = match call.priority {
            Priority::Background => Some(
                Arc::clone(&self.background)
                    .acquire_owned()
                    .await
                    .map_err(|_closed| ForgeError::Busy)?,
            ),
            Priority::Person => None,
        };
        let _slot = Arc::clone(&self.slots)
            .acquire_owned()
            .await
            .map_err(|_closed| ForgeError::Busy)?;
        let key = call.url.as_str().to_owned();
        let mut request = self
            .client
            .request(call.method.clone(), call.url.clone())
            .header("accept", "application/vnd.github+json")
            .header("x-github-api-version", "2022-11-28");
        if let Some(header) = call.token.header() {
            request = request.header("authorization", header);
        }
        if call.conditional
            && let Some(etag) = self.state().etags.get(&key).cloned()
        {
            request = request.header("if-none-match", etag);
        }
        if let Some(json) = call.json {
            request = request
                .header("content-type", "application/json")
                .body(json);
        }
        let response = request.send().await.map_err(ForgeError::Unreachable)?;
        let status = response.status();
        let headers = response.headers().clone();
        self.state().limits.note(call.host, call.resource, &headers);
        if status == StatusCode::NOT_MODIFIED {
            return Ok(Reply::NotModified);
        }
        let body = read_bounded(response).await?;
        if status.is_success() {
            self.state().limits.cooled(call.host, Utc::now());
            if call.conditional
                && let Some(etag) = headers.get("etag")
            {
                self.keep_etag(key, etag.clone());
            }
            return Ok(Reply::Body { status, body });
        }
        if is_limit(status, &headers, &body) {
            let hold = Hold::of(call.resource, &headers);
            let until = self
                .state()
                .limits
                .cool_down_for(call.host, hold, &headers, Utc::now());
            return Err(ForgeError::RateLimited { until });
        }
        match status {
            StatusCode::UNAUTHORIZED => Err(ForgeError::Refused {
                reason: RefusedReason::Unauthorized,
            }),
            StatusCode::FORBIDDEN => Err(ForgeError::Refused {
                reason: sso(&headers).unwrap_or(RefusedReason::Forbidden),
            }),
            status if status.is_server_error() => Err(ForgeError::Server {
                status: status.as_u16(),
            }),
            status => Ok(Reply::Body { status, body }),
        }
    }

    /// A limit answer seen in a body (GraphQL's own, which comes with a
    /// 200): cool `host` down as for a 429.
    pub(crate) fn limited(&self, host: &ForgeHost) -> DateTime<Utc> {
        self.state()
            .limits
            .cool_down(host, &HeaderMap::new(), Utc::now())
    }

    /// Whether `call` may be sent now; the background waits a moment for
    /// its bucket.
    async fn admit(&self, call: &Call<'_>) -> Result<(), ForgeError> {
        let admit = self
            .state()
            .limits
            .admit(call.host, call.resource, call.priority, Utc::now());
        match admit {
            Admit::Go => Ok(()),
            Admit::Held(until) => Err(ForgeError::RateLimited { until }),
            Admit::After(wait) if wait > BUCKET_WAIT => Err(ForgeError::Busy),
            Admit::After(wait) => {
                tokio::time::sleep(wait).await;
                Ok(())
            }
        }
    }

    fn keep_etag(&self, key: String, etag: HeaderValue) {
        let mut state = self.state();
        if state.etags.insert(key.clone(), etag).is_none() {
            state.etag_order.push_back(key);
            while state.etag_order.len() > MAX_ETAGS {
                if let Some(oldest) = state.etag_order.pop_front() {
                    let _dropped = state.etags.remove(&oldest);
                }
            }
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Redirects: to the same origin only, at most three.
fn same_origin(attempt: reqwest::redirect::Attempt<'_>) -> reqwest::redirect::Action {
    let first = attempt.previous().first().map(reqwest::Url::origin);
    if attempt.previous().len() > 3 || first.as_ref() != Some(&attempt.url().origin()) {
        attempt.stop()
    } else {
        attempt.follow()
    }
}

/// The body, at most [`MAX_BODY`].
async fn read_bounded(mut response: reqwest::Response) -> Result<Vec<u8>, ForgeError> {
    let too_large = ForgeError::Unexpected {
        what: "an answer over 4 MiB",
    };
    if response
        .content_length()
        .is_some_and(|length| length > u64::try_from(MAX_BODY).unwrap_or(u64::MAX))
    {
        return Err(too_large);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(ForgeError::Unreachable)? {
        if body.len().saturating_add(chunk.len()) > MAX_BODY {
            return Err(too_large);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
