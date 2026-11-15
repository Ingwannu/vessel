//! The checked route collection for ADR 0097, without a decoder or I/O.
//!
//! [Decision Log]
//! - Purpose: route identity, preference and allocation history must survive
//!   endpoint changes without becoming an authenticated connection epoch.
//! - Existing assets: typed URLs, SSH targets, secure route kinds and file
//!   references; reuse them rather than parsing another address language.
//! - Choice: one immutable, nonempty checked collection. A future bounded
//!   decoder supplies records; malformed references refuse the entire value.
//! - Limits: this does not authenticate a route, authorize association,
//!   allocate a new id, perform failover or change the legacy book format.
//!   The decoder must bound allocation before constructing its input vector.

use std::collections::BTreeSet;
use std::fmt;

use vessel_proto::{BookRouteId, NodePublicKey};

use super::TokenSource;
use crate::address::SshTarget;
use crate::credential::CredentialSource;
use crate::secure::Route;
use crate::url::NodeUrl;

mod legacy;

/// ADR 0097's per-host route budget, shared by model and future file decoder.
pub const MAX_BOOK_ROUTES_PER_HOST: usize = 64;

/// A configured transport specification, never evidence that it was contacted.
///
/// Credential/token values remain file references. A secure route's configured
/// key cannot mint a pinned-channel receipt; only the actual secure wrapper can.
#[derive(Clone, PartialEq, Eq)]
pub enum ConnectionSpec {
    /// The existing bearer websocket transport.
    WebSocket {
        /// Parsed transport URL, including the existing protocol path rules.
        url: NodeUrl,
        /// The existing private bearer file reference, if authentication needs it.
        token: Option<TokenSource>,
    },
    /// The existing typed SSH transport; this adds no native SSH implementation.
    Ssh(SshTarget),
    /// A single secure direct or relay route with its configured binding.
    Secure {
        /// One route, retaining the distinction between direct and relay.
        route: Route,
        /// The key the secure handshake must actually verify before use.
        key: NodePublicKey,
        /// The device credential reference, checked before dialing by its owner.
        credential: CredentialSource,
    },
}

impl ConnectionSpec {
    fn secure_binding(&self) -> Option<(&NodePublicKey, &CredentialSource)> {
        match self {
            Self::WebSocket { .. } | Self::Ssh(_) => None,
            Self::Secure {
                key, credential, ..
            } => Some((key, credential)),
        }
    }
}

impl fmt::Debug for ConnectionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Even a failed assertion must not dump file references or endpoints.
        f.write_str(match self {
            Self::WebSocket { .. } => "WebSocket(..)",
            Self::Ssh(_) => "Ssh(..)",
            Self::Secure { .. } => "Secure(..)",
        })
    }
}

/// One entry-local persisted route identity and its complete specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    id: BookRouteId,
    spec: ConnectionSpec,
}

impl Connection {
    /// Associate already parsed values; collection-wide checks follow in `new`.
    #[must_use]
    pub const fn new(id: BookRouteId, spec: ConnectionSpec) -> Self {
        Self { id, spec }
    }

    /// The local identity; it is neither an epoch nor remote authority.
    #[must_use]
    pub const fn id(&self) -> BookRouteId {
        self.id
    }

    /// Borrow the exact specification for binding checks by its owning layer.
    #[must_use]
    pub const fn specification(&self) -> &ConnectionSpec {
        &self.spec
    }
}

/// A nonempty route list with a valid preference and retained allocation history.
///
/// This stores configuration only. Mixed transport classes do not imply any
/// automatic association/failover permission; the ADR's evidence matrix still
/// applies at mutation and connection time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connections {
    records: Vec<Connection>,
    preferred: BookRouteId,
    last_allocated: BookRouteId,
}

impl Connections {
    /// Check all references before a decoder/mutation can publish this value.
    ///
    /// # Errors
    /// Empty/oversized input, duplicate ids/specifications, an absent preference,
    /// a regressed counter, or contradictory secure credential/key bindings.
    pub fn new(
        records: Vec<Connection>,
        preferred: BookRouteId,
        last_allocated: BookRouteId,
    ) -> Result<Self, ConnectionProblem> {
        check(&records, preferred, last_allocated)?;
        Ok(Self {
            records,
            preferred,
            last_allocated,
        })
    }

    /// Preserve file order; selection is explicit, never first-alias resolution.
    pub fn iter(&self) -> impl Iterator<Item = &Connection> {
        self.records.iter()
    }

    /// The preferred id is guaranteed to name one of this collection's records.
    #[must_use]
    pub const fn preferred(&self) -> BookRouteId {
        self.preferred
    }

    /// Retain this even after removing a route, so future allocation cannot wrap.
    #[must_use]
    pub const fn last_allocated(&self) -> BookRouteId {
        self.last_allocated
    }

    /// Lookup within this exact profile; it does not resolve a node or client.
    #[must_use]
    pub fn get(&self, id: BookRouteId) -> Option<&Connection> {
        self.records.iter().find(|record| record.id == id)
    }
}

fn check(
    records: &[Connection],
    preferred: BookRouteId,
    last: BookRouteId,
) -> Result<(), ConnectionProblem> {
    if records.is_empty() {
        return Err(ConnectionProblem::Empty);
    }
    if records.len() > MAX_BOOK_ROUTES_PER_HOST {
        return Err(ConnectionProblem::TooMany);
    }
    let mut ids = BTreeSet::new();
    let mut secure = None;
    for (index, record) in records.iter().enumerate() {
        if !ids.insert(record.id) {
            return Err(ConnectionProblem::DuplicateId(record.id));
        }
        if record.id > last {
            return Err(ConnectionProblem::CounterBehind);
        }
        if records
            .iter()
            .take(index)
            .any(|prior| prior.spec == record.spec)
        {
            return Err(ConnectionProblem::DuplicateSpecification);
        }
        if let Some(binding) = record.spec.secure_binding() {
            if secure.is_some_and(|prior| prior != binding) {
                return Err(ConnectionProblem::SecureBindingMismatch);
            }
            secure = Some(binding);
        }
    }
    if !ids.contains(&preferred) {
        return Err(ConnectionProblem::UnknownPreferred);
    }
    Ok(())
}

/// Safe collection faults; no endpoint, path, key or parser text is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConnectionProblem {
    /// A host with no route must be removed instead of stored as usable.
    #[error("a host needs at least one connection")]
    Empty,
    /// Refuse overflow rather than silently losing saved routes.
    #[error("the host has too many saved connections")]
    TooMany,
    /// Route ids are unique inside a host, independent of endpoints.
    #[error("the host repeats connection id {0}")]
    DuplicateId(BookRouteId),
    /// Distinct ids must not hide duplicate specifications.
    #[error("the host repeats a connection specification")]
    DuplicateSpecification,
    /// Removal never lowers the retained last allocated id.
    #[error("the host's connection allocation counter has regressed")]
    CounterBehind,
    /// A preference is an existing reference, not an out-of-band destination.
    #[error("the preferred connection does not exist in this host")]
    UnknownPreferred,
    /// Secure routes in a host must share their configured identity/credential.
    #[error("the host's secure connections have different bindings")]
    SecureBindingMismatch,
    /// Refuse contradictory public legacy fields instead of dropping one.
    #[error("the legacy host has inconsistent transport bindings")]
    InvalidLegacyBinding,
}

#[cfg(test)]
mod tests;
