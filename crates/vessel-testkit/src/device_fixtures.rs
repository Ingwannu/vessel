//! Self-inspection shapes shared by real-node tests and fixture consumers.
use vessel_proto::{DeviceSelf, Scope};

/// Independent grants: a reader and two callers without Read or Admin.
pub const SELF_GRANTS: [&[Scope]; 3] = [&[Scope::Read], &[Scope::Operate], &[Scope::Sync]];

/// A local owner's expected response: every scope, no paired identity.
#[must_use]
pub fn owner_self() -> DeviceSelf {
    DeviceSelf {
        device: None,
        scopes: Scope::ALL.to_vec(),
    }
}

/// The full refusal when no advertised route can carry an invitation.
#[must_use]
pub fn unreachable_invitation() -> vessel_proto::WireError {
    vessel_proto::WireError {
        code: vessel_proto::ErrorCode::Conflict,
        message: "this node has no pairing route; configure --public with a reachable address or --relay, then create a new invitation".to_owned(),
        detail: Some(vessel_proto::ErrorDetail::DeviceInvite {
            reason: vessel_proto::DeviceInviteError::NoRoutes,
        }),
    }
}
