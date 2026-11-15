//! Construct the upstream client; calls and response handling stay in the parent.

use super::{ClientError, Limits};
use std::sync::Arc;

/// Calls providers. Cheap to clone (one connection pool).
#[derive(Debug, Clone)]
pub struct Client {
    /// Parent call/response code uses the same private HTTP pool.
    pub(super) http: reqwest::Client,
    /// Parent reads the policy fixed at construction.
    pub(super) limits: Limits,
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
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(limits.connect)
            .build()
            .map_err(ClientError::Http)?;
        Ok(Self { http, limits })
    }
}
