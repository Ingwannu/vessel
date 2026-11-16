//! The original work lease stays inside the actual standard-native closure.

use super::{Addresses, Failure, FailureKind, HostName, MAX_RESOLVED_ADDRESSES, ResolutionPort};
use std::{
    net::{SocketAddr, ToSocketAddrs as _},
    sync::Arc,
};

pub(super) fn resolve(host: &HostName, port: ResolutionPort) -> Result<Addresses, Failure> {
    let records = (host.0.as_ref(), port.0)
        .to_socket_addrs()
        .map_err(Failure::native)?;
    collect(records)
}

pub(super) fn collect(records: impl Iterator<Item = SocketAddr>) -> Result<Addresses, Failure> {
    let mut unique = Vec::new();
    for (index, address) in records.enumerate() {
        // Count raw records, including duplicates, before retaining an item.
        if index >= MAX_RESOLVED_ADDRESSES {
            return Err(Failure::new(FailureKind::TooManyAddresses));
        }
        if !unique.contains(&address) {
            unique.push(address);
        }
    }
    if unique.is_empty() {
        return Err(Failure::new(FailureKind::Empty));
    }
    Ok(Addresses(Arc::from(unique)))
}
