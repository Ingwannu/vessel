//! Operating-system trust and explicit ring for the node's HTTPS clients.
//!
//! No destinations, credentials or HTTP policy. Callers that initialise after
//! startup must run this synchronous certificate loading on a blocking worker.

use std::sync::Arc;

/// Reuse the node's explicit TLS provider instead of a process-global install.
pub(crate) fn config() -> Result<rustls::ClientConfig, rustls::Error> {
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    let (_added, _ignored) = roots.add_parsable_certificates(native.certs);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(tls)
}
