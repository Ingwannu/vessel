//! Explicit session archive admission; a client preflight cannot authorize a stop.
use crate::SessionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The actor checks this condition before it stops accepting new turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCondition {
    /// Refuse an active turn without changing runtime or stored state.
    IfIdle,
    /// Stop the session even while a turn runs.
    Force,
}

/// Archive one session with an explicit, atomically admitted stop condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArchiveSession {
    /// Exact node-local session identity.
    pub session: SessionId,
    /// Required; an omitted field never silently becomes Force.
    pub condition: ArchiveCondition,
}

/// Acknowledged durable archive identity and timestamp, supplied by the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[must_use]
pub struct ArchivedSession {
    /// The session that was archived, never a replacement conversation.
    pub session: SessionId,
    /// The committed store mark; the CLI must not invent its own time.
    pub archived_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, Reply, Request as _};
    #[test]
    fn an_archive_condition_is_required_and_never_defaults_to_force() {
        let session = SessionId::random();
        let missing = format!(r#"{{"session":"{session}"}}"#);
        assert!(serde_json::from_str::<ArchiveSession>(&missing).is_err());
        let unknown = format!(r#"{{"session":"{session}","condition":"future_condition"}}"#);
        assert!(serde_json::from_str::<ArchiveSession>(&unknown).is_err());
        for condition in [ArchiveCondition::IfIdle, ArchiveCondition::Force] {
            let request = ArchiveSession { session, condition };
            let call = request.into_call();
            assert_eq!(
                serde_json::from_slice::<Call>(&serde_json::to_vec(&call).unwrap()).unwrap(),
                call
            );
            assert_eq!(
                ArchiveSession::METHOD.since(),
                crate::workspace::WORKSPACE_LIFECYCLE
            );
        }
    }
    #[test]
    fn acknowledged_identity_and_timestamp_survive_typed_reply_roundtrip() {
        let expected = ArchivedSession {
            session: SessionId::random(),
            archived_at: DateTime::UNIX_EPOCH,
        };
        let reply = Reply::ArchiveSession(expected);
        let decoded: Reply = serde_json::from_slice(&serde_json::to_vec(&reply).unwrap()).unwrap();
        assert_eq!(ArchiveSession::response_from(decoded).unwrap(), expected);
    }
}
