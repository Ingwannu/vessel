//! Exact origin metadata has no ability to authorize a connection.

use std::net::{IpAddr, SocketAddr};

use vessel_proto::workspace::ServicePort;

use super::PreviewError;

/// The adapter has already decoded a supported browser URL scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewScheme {
    Http,
    Https,
}

/// Match a literal backend origin; only the trusted source can issue a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviewOrigin {
    scheme: PreviewScheme,
    address: IpAddr,
    port: ServicePort,
}

impl PreviewOrigin {
    /// This constructor validates matcher data, never ownership or a TLS proof.
    pub(crate) fn new(scheme: PreviewScheme, endpoint: SocketAddr) -> Result<Self, PreviewError> {
        if !endpoint.ip().is_loopback() {
            return Err(PreviewError::NotLoopback);
        }
        let port = ServicePort::try_from(endpoint.port()).map_err(PreviewError::Port)?;
        Ok(Self {
            scheme,
            address: endpoint.ip(),
            port,
        })
    }

    /// The exact scheme is part of equality, not a reason to try a fallback.
    pub(crate) const fn scheme(self) -> PreviewScheme {
        self.scheme
    }

    /// A node adapter may compare metadata; dialing still requires the lease.
    pub(crate) const fn endpoint(self) -> SocketAddr {
        SocketAddr::new(self.address, self.port.get())
    }
}
