//! Fetching the ACP registry (`agent.catalogue` with `refresh`, ADR 0032):
//! one bounded GET, only when a person asks.
//!
//! [Decision Log]
//! - Purpose: the catalogue lists what the registry offers, so a person
//!   can add an agent without writing its command.
//! - Constraint: the answer comes from the network and names programs the
//!   node may later run; a slow or huge answer must not hold the node.
//! - Choice: `reqwest` with the workspace's rustls (ring) and the
//!   operating system's roots (the model proxy's client is built the same
//!   way, `vessel_llm::Client`), no redirect, no proxy, 10 s to connect,
//!   30 s in all, the body read up to 2 MiB and refused past it. https
//!   only, except plain http to a loopback address, which is how a test
//!   serves its fixture (the URL is the node's `Config` value; the
//!   published one otherwise).
//! - Not reused: `vessel_llm::Client`, whose retries, error classes and
//!   streaming are a model provider's (the contract's review).

use std::time::Duration;

use reqwest::Url;

/// The registry's published index.
pub(crate) const REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

/// The most bytes read.
pub(crate) const MAX_BYTES: usize = 2 * 1024 * 1024;
const CONNECT: Duration = Duration::from_secs(10);
const TOTAL: Duration = Duration::from_secs(30);

/// Why the registry could not be fetched.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error("the registry's address is not an https URL (or http on this machine)")]
    Address,
    #[error("the registry could not be reached")]
    Unreachable(#[source] reqwest::Error),
    #[error("the registry answered {0}")]
    Status(u16),
    #[error("the registry did not answer within {} s", TOTAL.as_secs())]
    TimedOut,
    #[error("the registry's answer is larger than 2 MiB")]
    TooLarge,
    #[error("the TLS configuration was refused")]
    Tls(#[source] rustls::Error),
}

/// The client the node fetches with.
#[derive(Debug, Clone)]
pub(crate) struct Fetcher {
    http: reqwest::Client,
    url: Url,
}

impl Fetcher {
    /// A fetcher of `url`, trusting the operating system's roots.
    ///
    /// # Errors
    /// The URL is not one the node fetches, or TLS cannot be set up.
    pub(crate) fn new(url: &str) -> Result<Self, FetchError> {
        let url = Url::parse(url).map_err(|_error| FetchError::Address)?;
        let loopback = url.scheme() == "http"
            && url.host_str().is_some_and(|host| {
                host == "localhost"
                    || host
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            });
        if !(url.scheme() == "https" || loopback)
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(FetchError::Address);
        }
        let tls = crate::http_tls::config().map_err(FetchError::Tls)?;
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(CONNECT)
            .build()
            .map_err(FetchError::Unreachable)?;
        Ok(Self { http, url })
    }

    /// The registry's bytes.
    ///
    /// # Errors
    /// See [`FetchError`].
    pub(crate) async fn fetch(&self) -> Result<Vec<u8>, FetchError> {
        tokio::time::timeout(TOTAL, self.read())
            .await
            .unwrap_or(Err(FetchError::TimedOut))
    }

    async fn read(&self) -> Result<Vec<u8>, FetchError> {
        let response = self
            .http
            .get(self.url.clone())
            .send()
            .await
            .map_err(FetchError::Unreachable)?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Status(status.as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| usize::try_from(length).map_or(true, |length| length > MAX_BYTES))
        {
            return Err(FetchError::TooLarge);
        }
        let mut response = response;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(FetchError::Unreachable)? {
            if body.len().saturating_add(chunk.len()) > MAX_BYTES {
                return Err(FetchError::TooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}
