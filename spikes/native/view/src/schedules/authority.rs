//! Authority evidence for one schedule row on one connection. An absent scope
//! report is not a denial. Keep a refusal on its row, not on unrelated rows;
//! replace this evidence when the connection changes.

use vessel_proto::{ErrorCode, Scope};

/// Row/connection-local evidence, using the wire's actual scope type.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Authority {
    /// The connection does not report its scopes. Offer actions and let the
    /// node decide, as the rest of the app does.
    #[default]
    Unknown,
    /// Scopes actually reported for this device, never inferred from defaults.
    Scopes(Vec<Scope>),
    /// The node answered Forbidden to an operation on this connection. This
    /// records the refusal without inventing a list of the device's scopes.
    OperationForbidden,
}

impl Authority {
    /// Known insufficient or actually refused means no mutation actions.
    #[must_use]
    pub fn allows_actions(&self) -> bool {
        match self {
            Self::Unknown => true,
            Self::Scopes(scopes) => scopes.contains(&Scope::Operate),
            Self::OperationForbidden => false,
        }
    }

    /// A successful self-query can update a grant, but cannot erase a resource
    /// refusal already observed on this connection. Missing reports keep the
    /// last known grant rather than inventing an empty one.
    pub fn reported(&mut self, scopes: Option<&[Scope]>) {
        if !matches!(self, Self::OperationForbidden)
            && let Some(scopes) = scopes
        {
            *self = Self::Scopes(scopes.to_vec());
        }
    }

    /// A node refusal is evidence for this connection only. Starting a new
    /// connection constructs a new Authority; other failures never narrow it.
    pub fn refused(&mut self, code: ErrorCode) {
        match code {
            ErrorCode::Forbidden => *self = Self::OperationForbidden,
            ErrorCode::InvalidRequest
            | ErrorCode::Unauthorized
            | ErrorCode::ProtocolMismatch
            | ErrorCode::NotFound
            | ErrorCode::Conflict
            | ErrorCode::Unsupported
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Unavailable
            | ErrorCode::Internal => {}
        }
    }
}
