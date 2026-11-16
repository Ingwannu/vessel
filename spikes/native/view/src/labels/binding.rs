//! Welcome identity owns catalogs; a local route alias never manufactures one.

use super::{Catalog, Failure, Host, HostState};
use crate::{Fleet, machine_book::Name};
use vessel_proto::{ProtocolVersion, workspace_labels::WORKSPACE_LABELS};

/// A fresh projection retains offline caches without attributing them to a new peer.
#[derive(Debug, Default)]
pub struct Bindings {
    /// Explicit book routes, sharing cached colors by answered node identity.
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
                let connection = machine.connection_context();
                let cached = previous.iter().find(|host| host.key == key);
                let same_admission = previous.iter().find(|host| {
                    host.key == key && connection.is_some() && host.connection == connection
                });
                let state = availability(
                    machine,
                    same_admission.map(|host| host.state),
                    WORKSPACE_LABELS,
                );
                let catalog = cached.map_or_else(Catalog::default, |host| host.catalog.clone());
                result.hosts.push(Host {
                    key,
                    route,
                    connection,
                    state,
                    catalog,
                });
            }
            None => {
                if let Some(cached) = previous.iter().find(|host| host.route == route) {
                    let mut cached = cached.clone();
                    cached.connection = None;
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

fn availability(
    machine: &crate::Machine,
    previous: Option<HostState>,
    since: Option<ProtocolVersion>,
) -> HostState {
    if machine.connection_context().is_none() {
        return HostState::Offline;
    }
    if since.is_none_or(|since| machine.protocol.is_none_or(|peer| peer < since)) {
        return HostState::Unsupported;
    }
    match previous {
        Some(state @ (HostState::Online | HostState::Failed(_))) => state,
        None | Some(HostState::Offline | HostState::Loading | HostState::Unsupported) => {
            HostState::Loading
        }
    }
}

#[cfg(test)]
mod tests;
