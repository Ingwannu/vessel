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
