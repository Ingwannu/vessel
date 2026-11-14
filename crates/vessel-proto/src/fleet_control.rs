//! Fleet controls and a credential-free state projection (protocol 15, ADR 0044).
use crate::{BundlePath, FleetMember, Generation, NodeId, NodeInfo, PairingLink, Scope};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Join this node to the home named by one single-use invitation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct JoinFleet {
    /// Never logged or returned; the node redeems it on the caller's behalf.
    pub link: PairingLink,
    /// What this node grants its home; never inferred from the invitation.
    pub grant: Vec<Scope>,
}

/// The authenticated home after the membership was persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct JoinedFleet {
    /// The home, without credentials or connection routes.
    pub home: NodeInfo,
}

/// Leave this node's fleet. Applied files stay; already left is success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LeaveFleet {}

/// Ask this node to sync now. Ack means accepted, not already applied.
/// Concurrent requests coalesce behind the node's current apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SyncFleet {}

/// Read this node's role and last apply, without resolving any secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetFleetStatus {}

/// This node's current configured role, independently of connectivity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FleetRole {
    /// No fleet is configured.
    Standalone,
    /// This node publishes its configured sources.
    Home,
    /// This node follows a home; files may still be from the previous pull.
    Member {
        /// The configured home.
        home: NodeId,
        /// When the current membership was persisted.
        joined_at: DateTime<Utc>,
    },
    /// A future role; clients must not infer controls from it.
    #[serde(other)]
    Unknown,
}

/// Actionable issue classes; arbitrary file errors remain in the local log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FleetSyncProblem {
    /// The latest fleet operation failed; inspect this machine's logs.
    OperationFailed,
    /// Some destinations could not be written.
    ApplyFailed,
    /// Some source files were left out of the published bundle.
    IncompleteSources,
    /// The home could not acquire the authority granted by this member.
    GrantFailed,
    /// A future problem; show a general failure without discarding it.
    #[serde(other)]
    Unknown,
}

/// Safe for any caller with Read; no file text, credentials or secret names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FleetApplied {
    /// The last generation applied here, possibly before the latest join.
    /// It has no current-home association: never compare its bare number
    /// with the current home's generation to claim this node is up to date.
    pub generation: Option<Generation>,
    /// When that apply completed.
    pub applied_at: Option<DateTime<Utc>>,
    /// When the member's home last answered.
    pub contacted_at: Option<DateTime<Utc>>,
    /// Bundle-relative paths kept as edited, deleted or not owned by Vessel.
    pub conflicts: Vec<BundlePath>,
    /// Issues at the last recorded operation.
    pub problems: Vec<FleetSyncProblem>,
}

/// Local state plus the home's per-member reports when the caller is Admin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FleetSyncStatus {
    /// The configured role.
    pub role: FleetRole,
    /// Last apply here, not a promise that the current sources were applied.
    pub local: FleetApplied,
    /// Present only on a home for an Admin caller; read-only sees none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<FleetMember>>,
}

/// Why a fleet control was refused, without invitation or filesystem text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FleetControlError {
    /// Another membership operation is in progress.
    #[error("another fleet operation is in progress; try again shortly")]
    Busy,
    /// A publishing home cannot follow another home.
    #[error("this machine publishes a fleet; remove its home configuration before joining another")]
    AlreadyHome,
    /// Leaving is explicit; a link cannot silently replace a home.
    #[error("this machine already follows a fleet; leave it before joining another")]
    AlreadyMember,
    /// A node never follows itself, even under another claimed id.
    #[error("choose another machine as the fleet's home")]
    SelfHome,
    /// The link cannot be decoded.
    #[error("use a fresh fleet invitation from the home machine")]
    InvalidInvitation,
    /// The authenticated home could not be reached or refused redemption.
    #[error("could not join the home; check its connection and use a fresh invitation")]
    CannotJoin,
    /// The data directory could not commit or remove membership.
    #[error("could not save the fleet change; check this machine's data directory and logs")]
    CannotStore,
    /// Pairing completed, but commit admission or persistence failed.
    #[error(
        "the invitation was redeemed but membership could not be saved; inspect this machine's fleet status before making another invitation"
    )]
    JoinUncertain,
    /// There is no home or membership to sync.
    #[error("join a fleet or configure this machine as its home before syncing")]
    NotConfigured,
    /// Fleet management stays on the selected machine's own connection.
    #[error("manage the fleet through this machine's own authenticated connection")]
    DirectOnly,
    /// The status file could not be read.
    #[error("could not read fleet state; check this machine's logs")]
    CannotRead,
    /// No truncated state or member list is returned.
    #[error("fleet status is too large to send; inspect this machine's local fleet status")]
    StatusTooLarge,
    /// What this machine shares is longer than a reply carries (protocol
    /// 16, `fleet.published`); none of it is sent.
    #[error("the shared list is longer than one reply carries; none of it was sent")]
    PublishedTooLarge,
    /// A future reason; retain the message and offer no guessed recovery.
    #[serde(other)]
    #[error("update this client to read the fleet refusal")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, ErrorCode, ErrorDetail, Request as _, WireError};

    #[test]
    fn fleet_controls_have_independent_authority_and_round_trip() {
        for (call, scope, forwarded) in [
            (LeaveFleet {}.into_call(), Scope::Admin, false),
            (SyncFleet {}.into_call(), Scope::Admin, false),
            (GetFleetStatus {}.into_call(), Scope::Read, true),
        ] {
            let method = call.method();
            let encoded = serde_json::to_string(&call).unwrap();
            assert_eq!(serde_json::from_str::<Call>(&encoded).unwrap(), call);
            assert_eq!(method.scope(), scope);
            assert_eq!(method.forwardable(), forwarded);
        }
    }

    #[test]
    fn future_fleet_roles_and_problems_are_retained_as_unknown() {
        assert_eq!(
            serde_json::from_str::<FleetRole>(r#"{"kind":"future_role","extra":true}"#).unwrap(),
            FleetRole::Unknown
        );
        assert_eq!(
            serde_json::from_str::<FleetSyncProblem>(r#""future_problem""#).unwrap(),
            FleetSyncProblem::Unknown
        );
        let mut error = WireError::new(ErrorCode::Conflict, "choose another home");
        error.detail = Some(ErrorDetail::FleetControl {
            reason: FleetControlError::AlreadyHome,
        });
        assert_eq!(
            serde_json::from_str::<WireError>(&serde_json::to_string(&error).unwrap()).unwrap(),
            error
        );
    }
}
