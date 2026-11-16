//! Construct the upstream client; calls and response handling stay in the parent.

use super::{ClientError, Limits};
use std::sync::Arc;

/// Calls providers. Cheap to clone (one connection pool).
#[derive(Clone)]
pub struct Client {
    /// Parent call/response code uses the same private HTTP pool.
    pub(super) http: reqwest::Client,
    /// Parent reads the policy fixed at construction.
    pub(super) limits: Limits,
    tls: Arc<rustls::ClientConfig>,
}

impl Client {
    /// A client trusting the operating system's root certificates.
    ///
    /// # Errors
    /// No root certificate could be loaded, or the TLS or HTTP
    /// configuration was refused.
    pub fn new(limits: Limits) -> Result<Self, ClientError> {
        let found = rustls_native_certs::load_native_certs();
        let mut roots = rustls::RootCertStore::empty();
        let (_added, _ignored) = roots.add_parsable_certificates(found.certs);
        if roots.is_empty() {
            return Err(ClientError::NoRoots);
        }
        Self::with_roots(roots, limits)
    }

    /// A client trusting exactly `roots`.
    ///
    /// # Errors
    /// The TLS or HTTP configuration was refused.
    pub fn with_roots(roots: rustls::RootCertStore, limits: Limits) -> Result<Self, ClientError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(ClientError::Tls)?
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Self::from_config(Arc::new(tls), limits)
    }

    /// Build a separate pool with different bounds and the same initialized trust.
    /// Cloning an existing HTTP pool would retain its original connect timeout;
    /// this factory deliberately rebuilds it without rereading system roots.
    ///
    /// # Errors
    /// The HTTP client builder refuses the requested configuration.
    pub fn with_limits(&self, limits: Limits) -> Result<Self, ClientError> {
        Self::from_config(Arc::clone(&self.tls), limits)
    }

    fn from_config(tls: Arc<rustls::ClientConfig>, limits: Limits) -> Result<Self, ClientError> {
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls.as_ref().clone())
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(limits.connect)
            .build()
            .map_err(ClientError::Http)?;
        Ok(Self { http, limits, tls })
    }
}

// TLS configuration may own resumption state; it is not diagnostic output.
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_separate_policy_reuses_trust_without_mutating_the_original() {
        let original =
            Client::with_roots(rustls::RootCertStore::empty(), Limits::default()).unwrap();
        let original_limits = original.limits();
        let mut reduced = original_limits;
        reduced.connect = reduced.connect.checked_div(2).unwrap();
        reduced.max_body = reduced.max_body.saturating_sub(1);
        reduced.retries = reduced.retries.saturating_sub(1);
        let bounded = original.with_limits(reduced).unwrap();
        assert!(Arc::ptr_eq(&original.tls, &bounded.tls));
        assert_eq!(bounded.limits(), reduced);
        assert_eq!(original.limits(), original_limits);
        let rebuilt = bounded.with_limits(original_limits).unwrap();
        assert!(Arc::ptr_eq(&original.tls, &rebuilt.tls));
        assert_eq!(bounded.limits(), reduced);
        assert_eq!(rebuilt.limits(), original_limits);
    }
}
