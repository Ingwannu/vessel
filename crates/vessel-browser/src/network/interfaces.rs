//! Current OS interface observations, never a caller/workspace-provided list.

use std::collections::BTreeSet;
use std::net::IpAddr;

/// Bound both observation work and retained local addresses.
#[cfg(target_os = "linux")]
const MAX_INTERFACE_RECORDS: usize = 512;

/// Interface-observation failures do not weaken the destination rule.
#[derive(Debug, thiserror::Error)]
pub(crate) enum InterfaceError {
    /// No qualified implementation is available on this platform yet.
    #[error("browser interface observation is unsupported on this platform")]
    #[cfg(not(target_os = "linux"))]
    Unsupported,
    /// The system call must complete successfully.
    #[cfg(target_os = "linux")]
    #[error("browser interface observation failed")]
    Observe(#[source] nix::errno::Errno),
    /// A truncated snapshot cannot prove this host's address set.
    #[error("browser interface observation exceeded its bound or was empty")]
    Incomplete,
}

pub(super) struct LocalInterfaces(BTreeSet<IpAddr>);
impl LocalInterfaces {
    pub(super) fn contains(&self, address: IpAddr) -> bool {
        self.0.contains(&super::canonical(address))
    }

    #[cfg(test)]
    pub(super) fn fixture(addresses: impl IntoIterator<Item = IpAddr>) -> Self {
        Self(addresses.into_iter().map(super::canonical).collect())
    }
}

#[cfg(target_os = "linux")]
pub(super) fn observe() -> Result<LocalInterfaces, InterfaceError> {
    let records = nix::ifaddrs::getifaddrs().map_err(InterfaceError::Observe)?;
    let mut addresses = BTreeSet::new();
    for (index, interface) in records.enumerate() {
        if index >= MAX_INTERFACE_RECORDS {
            return Err(InterfaceError::Incomplete);
        }
        if let Some(address) = interface.address {
            if let Some(v4) = address.as_sockaddr_in() {
                let _new = addresses.insert(IpAddr::V4(v4.ip()));
            } else if let Some(v6) = address.as_sockaddr_in6() {
                let _new = addresses.insert(super::canonical(IpAddr::V6(v6.ip())));
            }
            // Link-layer/unspecified interface records have no dialable IP.
        }
    }
    if addresses.is_empty() {
        return Err(InterfaceError::Incomplete);
    }
    Ok(LocalInterfaces(addresses))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn observe() -> Result<LocalInterfaces, InterfaceError> {
    Err(InterfaceError::Unsupported)
}
