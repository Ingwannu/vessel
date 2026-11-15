//! Welcome identity owns catalogs; a local route alias never manufactures one.

use super::{Catalog, Failure, Host, HostState};
use crate::{Fleet, Reach, machine_book::Name};

/// A fresh projection retains offline caches without attributing them to a new peer.
#[derive(Debug, Default)]
pub struct Bindings {
    /// Deduplicated answered identities, including their retained offline colors.
    pub hosts: Vec<Host>,
    /// Book aliases that have never answered with a known identity.
    pub unresolved: Vec<Name>,
    /// A malformed legacy alias is a visible failure, never silently accepted.
    pub error: Option<Failure>,
}

/// Preserve colors across alias renames, but never across a different welcome id.
#[must_use]
pub fn bind_hosts(previous: &[Host], fleet: &Fleet) -> Bindings {
    let mut result = Bindings::default();
    for machine in &fleet.machines {
        let route = match Name::parse(&machine.name) {
            Ok(route) => route,
            Err(_invalid_book_alias) => {
                result.error = Some(Failure::Load);
                continue;
            }
        };
        match machine.node {
            Some(key) => {
                let availability = match machine.reach {
                    Reach::Online => HostState::Unsupported,
                    Reach::Unreachable(_) => HostState::Offline,
                };
                if let Some(known) = result.hosts.iter_mut().find(|host| host.key == key) {
                    if known.state == HostState::Offline {
                        known.state = availability;
                        known.route = route;
                    }
                    continue;
                }
                let catalog = previous
                    .iter()
                    .find(|host| host.key == key)
                    .map_or_else(Catalog::default, |host| host.catalog.clone());
                result.hosts.push(Host {
                    key,
                    route,
                    state: availability,
                    catalog,
                });
            }
            None => {
                if let Some(cached) = previous.iter().find(|host| host.route == route) {
                    let mut cached = cached.clone();
                    cached.state = HostState::Offline;
                    result.hosts.push(cached);
                } else {
                    result.unresolved.push(route);
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests;
