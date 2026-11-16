//! Persisted execution correlation is evidence, never a runtime authority object.

use crate::{ExecutionIncarnationId, NodeId, SessionId};
use serde::{Deserialize, Serialize};

/// Durable correlation cannot construct the node's private live admission fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ParentExecutionStamp {
    /// Actual local owner; foreign/delegated capture is unsupported.
    pub node: NodeId,
    /// Parent whose execution was captured.
    pub session: SessionId,
    /// Original incarnation; a later execution cannot replace it in saved intent.
    pub incarnation: ExecutionIncarnationId,
}

/// Saved authority is at most Read/Operate and never grows during recovery.
/// Finish registration/admission requires `ReadOperate` and the same current grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FinishGrantCeiling {
    /// Read alone cannot enqueue a notice.
    Read,
    /// Operate alone cannot observe a child.
    Operate,
    /// Both remain necessary at actual atomic queue admission.
    ReadOperate,
}

/// Private transition diagnostics; no variant grants a new execution capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ExecutionInvalidation {
    /// Explicit authorized stop, before its first effect. This diagnostic cause
    /// also covers already-allowed agent requests; it proves no human principal
    /// and changes neither authorization nor the stop operation's permissions.
    PersonStop,
    /// Explicit resume never restores old intent authority.
    Resume,
    /// Archiving invalidates outstanding asynchronous work.
    Archive,
    /// Detach changes the current relationship/execution boundary.
    Detach,
    /// A replacement driver is another execution even under the same session id.
    DriverReplaced,
    /// Current grant changed; a saved ceiling cannot preserve the prior grant.
    GrantChanged,
    /// Natural actor end/retirement also closes the live incarnation.
    Ended,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_stamp_roundtrips_without_accepting_a_malformed_incarnation_or_admin() {
        let stamp = ParentExecutionStamp {
            node: NodeId::random(),
            session: SessionId::random(),
            incarnation: ExecutionIncarnationId::random(),
        };
        let encoded = serde_json::to_string(&stamp).unwrap();
        assert_eq!(
            serde_json::from_str::<ParentExecutionStamp>(&encoded).unwrap(),
            stamp
        );
        assert!(
            serde_json::from_str::<ParentExecutionStamp>(
                &encoded.replace(&stamp.incarnation.to_string(), "invalid")
            )
            .is_err()
        );
        assert!(serde_json::from_str::<FinishGrantCeiling>("\"admin\"").is_err());
    }
}
