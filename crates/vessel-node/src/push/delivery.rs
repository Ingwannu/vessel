//! HTTPS egress for a browser-issued capability: resolve once, reject every
//! private answer and pin the accepted addresses for the actual connection.
//! Never log reqwest's error (it includes the capability URL).
use axum::http::{HeaderMap, Request};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use vessel_proto::push::PushEndpoint;

const TOTAL: Duration = Duration::from_secs(10);
#[derive(Debug, thiserror::Error)]
pub(crate) enum DeliveryError {
    #[error("the push endpoint must resolve only to public HTTPS addresses")]
    Endpoint,
    #[error("the push endpoint could not be resolved")]
    Resolve(#[source] std::io::Error),
    #[error("the push endpoint did not answer in time")]
    Timeout,
    #[error("the push request could not be sent")]
    Http(#[source] reqwest::Error),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delivered {
    Accepted,
    Gone,
    Refused,
}

/// TLS roots are prepared on the blocking pool once per node's push setup.
#[derive(Debug, Clone)]
pub(crate) struct Delivery {
    tls: Arc<rustls::ClientConfig>,
    #[cfg(test)]
    test_origin: Option<url::Url>,
}
impl Delivery {
    pub(crate) fn new() -> Result<Self, rustls::Error> {
        let mut roots = rustls::RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        let (_added, _ignored) = roots.add_parsable_certificates(native.certs);
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            tls: Arc::new(tls),
            #[cfg(test)]
            test_origin: None,
        })
    }
    #[cfg(test)]
    pub(crate) fn test_at(mut self, origin: url::Url) -> Self {
        self.test_origin = Some(origin);
        self
    }

    pub(crate) async fn validate(&self, endpoint: &PushEndpoint) -> Result<(), DeliveryError> {
        #[cfg(test)]
        if self.test_origin.is_some() {
            return Ok(());
        }
        let _addresses = resolved(endpoint).await?;
        Ok(())
    }
    pub(crate) async fn prepare(
        &self,
        endpoint: &PushEndpoint,
        request: Request<Vec<u8>>,
    ) -> Result<Prepared, DeliveryError> {
        let url = url::Url::parse(endpoint.expose()).map_err(|_invalid| DeliveryError::Endpoint)?;
        #[cfg(test)]
        let url = self.test_origin.clone().unwrap_or(url);
        let mut builder = reqwest::Client::builder()
            .use_preconfigured_tls((*self.tls).clone())
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(TOTAL)
            .pool_max_idle_per_host(0)
            .http1_max_headers(32)
            .http2_max_header_list_size(32 * 1024);
        if url.scheme() == "https" {
            let (host, addresses) = resolved(endpoint).await?;
            builder = builder.resolve_to_addrs(&host, &addresses);
        } else {
            // The only non-HTTPS route is compiled into unit tests, selected
            // by an injected transport; no wire endpoint can enable it.
            #[cfg(not(test))]
            return Err(DeliveryError::Endpoint);
            #[cfg(test)]
            if self.test_origin.is_none() {
                return Err(DeliveryError::Endpoint);
            }
        }
        let http = builder.build().map_err(DeliveryError::Http)?;
        let (parts, bytes) = request.into_parts();
        let mut headers: HeaderMap = parts.headers;
        if let Some(authorization) = headers.get_mut(axum::http::header::AUTHORIZATION) {
            authorization.set_sensitive(true);
        }
        let request = http
            .post(url)
            .headers(headers)
            .header("Topic", "vessel-attention")
            .body(bytes)
            .build()
            .map_err(DeliveryError::Http)?;
        Ok(Prepared { http, request })
    }
}
/// DNS resolution and encryption have finished before authority is admitted.
/// `send` performs no application-level preparation await before HTTP starts.
pub(crate) struct Prepared {
    http: reqwest::Client,
    request: reqwest::Request,
}
impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedPush(<redacted>)")
    }
}
impl Prepared {
    pub(crate) async fn send(self) -> Result<Delivered, DeliveryError> {
        let response = self
            .http
            .execute(self.request)
            .await
            .map_err(DeliveryError::Http)?;
        // Status is sufficient: no consumer collects, decodes or reads the
        // response body. HTTP may prefetch bytes in its bounded transport
        // buffers, so this does not claim zero bytes received on the socket.
        let status = response.status();
        if status.is_success() {
            Ok(Delivered::Accepted)
        } else if status.as_u16() == 404 || status.as_u16() == 410 {
            Ok(Delivered::Gone)
        } else {
            Ok(Delivered::Refused)
        }
    }
}
async fn resolved(endpoint: &PushEndpoint) -> Result<(String, Vec<SocketAddr>), DeliveryError> {
    let url = url::Url::parse(endpoint.expose()).map_err(|_invalid| DeliveryError::Endpoint)?;
    let host = url.host_str().ok_or(DeliveryError::Endpoint)?;
    let found = tokio::time::timeout(Duration::from_secs(5), tokio::net::lookup_host((host, 443)))
        .await
        .map_err(|_elapsed| DeliveryError::Timeout)?
        .map_err(DeliveryError::Resolve)?;
    let mut addresses = Vec::new();
    for address in found {
        if addresses.len() >= 16 || !vessel_relay::is_public(address.ip()) {
            return Err(DeliveryError::Endpoint);
        }
        addresses.push(address);
    }
    if addresses.is_empty() {
        return Err(DeliveryError::Endpoint);
    }
    Ok((host.to_owned(), addresses))
}
