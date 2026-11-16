//! Validated inputs and bounded, non-cached application results.

use super::{Failure, FailureKind, MAX_HOST_BYTES, MAX_NATIVE_RESOLUTIONS};
use std::{fmt, net::SocketAddr, num::NonZeroUsize, sync::Arc};

/// A hostname or numeric IP, never an URL. Debug deliberately omits its input.
#[derive(Clone)]
pub struct HostName(pub(super) Box<str>);
impl HostName {
    /// Validate the bounded host input. Work admission precedes the additional
    /// worker-owned copy; this caller-owned value grants no work or capacity.
    ///
    /// # Errors
    /// Rejects empty/oversized input and URL, whitespace or control components.
    pub fn new(host: &str) -> Result<Self, Failure> {
        if host.is_empty()
            || host.len() > MAX_HOST_BYTES
            || host.chars().any(|c| {
                c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '?' | '#' | '@')
            })
        {
            return Err(Failure::new(FailureKind::InvalidHost));
        }
        Ok(Self(host.into()))
    }
}
impl fmt::Debug for HostName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HostName(<redacted>)")
    }
}

/// Port for this lookup; zero is the reqwest resolver-hook convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionPort(pub u16);

/// A consumer's finite share, supplied from its existing source constant.
#[derive(Debug, Clone, Copy)]
pub struct ResolutionQuota(pub(super) NonZeroUsize);
impl ResolutionQuota {
    /// Admit between one slot and the root's fixed maximum.
    ///
    /// # Errors
    /// Zero and values above `MAX_NATIVE_RESOLUTIONS` are refused.
    pub fn new(count: usize) -> Result<Self, Failure> {
        match NonZeroUsize::new(count) {
            Some(count) if count.get() <= MAX_NATIVE_RESOLUTIONS => Ok(Self(count)),
            Some(_) | None => Err(Failure::new(FailureKind::InvalidQuota)),
        }
    }
}

/// Resolver-order unique addresses for this result only; never a cache.
#[derive(Clone)]
pub struct Addresses(pub(super) Arc<[SocketAddr]>);
impl Addresses {
    /// Exact socket addresses to apply the consumer's Reach policy to and dial.
    pub fn iter(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.0.iter().copied()
    }
}
impl fmt::Debug for Addresses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Addresses")
            .field("count", &self.0.len())
            .finish_non_exhaustive()
    }
}
