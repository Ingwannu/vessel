//! Pure legacy expansion, called only by an eventual explicit format upgrade.
//!
//! Legacy reads do not call this or rewrite anything. Stable ids follow the
//! existing connector order: direct before relay. Check the budget before
//! allocating a new vector; overflow refuses the original entry intact.

use vessel_proto::BookRouteId;

use super::{Connection, ConnectionProblem, ConnectionSpec, Connections, MAX_BOOK_ROUTES_PER_HOST};
use crate::address::NodeAddress;
use crate::book::BookEntry;
use crate::secure::Route;

impl TryFrom<&BookEntry> for Connections {
    type Error = ConnectionProblem;

    /// Prepare deterministic route identities without changing the old entry.
    ///
    /// This does not verify its configured `NodeId`, authenticate a connection,
    /// save format2 or grant a transport permission to inherit another's trust.
    fn try_from(entry: &BookEntry) -> Result<Self, Self::Error> {
        match &entry.address {
            NodeAddress::Ws(url) => {
                if entry.credential.is_some() {
                    return Err(ConnectionProblem::InvalidLegacyBinding);
                }
                single(ConnectionSpec::WebSocket {
                    url: url.clone(),
                    token: entry.token.clone(),
                })
            }
            NodeAddress::Ssh(target) => {
                if entry.token.is_some() || entry.credential.is_some() {
                    return Err(ConnectionProblem::InvalidLegacyBinding);
                }
                single(ConnectionSpec::Ssh(target.clone()))
            }
            NodeAddress::Secure(target) => {
                if entry.token.is_some() {
                    return Err(ConnectionProblem::InvalidLegacyBinding);
                }
                let credential = entry
                    .credential
                    .as_ref()
                    .ok_or(ConnectionProblem::InvalidLegacyBinding)?;
                let count = target
                    .direct
                    .len()
                    .checked_add(target.relays.len())
                    .filter(|count| *count <= MAX_BOOK_ROUTES_PER_HOST)
                    .ok_or(ConnectionProblem::TooMany)?;
                let routes = target
                    .direct
                    .iter()
                    .cloned()
                    .map(Route::Direct)
                    .chain(target.relays.iter().cloned().map(Route::Relay));
                let mut records = Vec::with_capacity(count);
                for (index, route) in routes.enumerate() {
                    let id = route_id(index)?;
                    records.push(Connection::new(
                        id,
                        ConnectionSpec::Secure {
                            route,
                            key: target.key,
                            credential: credential.clone(),
                        },
                    ));
                }
                let last = records.last().ok_or(ConnectionProblem::Empty)?.id();
                Self::new(records, BookRouteId::FIRST, last)
            }
        }
    }
}

fn single(spec: ConnectionSpec) -> Result<Connections, ConnectionProblem> {
    Connections::new(
        vec![Connection::new(BookRouteId::FIRST, spec)],
        BookRouteId::FIRST,
        BookRouteId::FIRST,
    )
}

fn route_id(index: usize) -> Result<BookRouteId, ConnectionProblem> {
    index
        .checked_add(1)
        .and_then(|number| u32::try_from(number).ok())
        .and_then(BookRouteId::new)
        .ok_or(ConnectionProblem::TooMany)
}

#[cfg(test)]
mod tests;
