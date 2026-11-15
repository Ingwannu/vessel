//! Bounded observation uses a revision cursor; missing is never interrupted.

use super::{CreationError, Key, Kind, Revision};
use serde::{Deserialize, Serialize};

/// Resolve a caller's durable identity even when its first reply was lost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ObserveCreation {
    /// Operation kind namespaces the opaque caller key.
    pub kind: Kind,
    /// Retain the original key across reconnects; never generate it on retry.
    pub key: Key,
}

/// One bounded long poll, reissued with the latest committed revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubscribeCreation {
    /// The same identity as initial admission, not an inferred resource path.
    pub identity: ObserveCreation,
    /// None requests the initial snapshot immediately.
    pub after: Option<Revision>,
    /// A saturated wait share answers immediately with the current snapshot.
    #[serde(default)]
    pub wait: ObservationWait,
}

/// Seconds of an observation wait, bounded before it can hold a server call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[cfg_attr(feature = "ts", ts(rename = "CreationObservationWait"))]
pub struct ObservationWait(u16);

impl ObservationWait {
    /// A creation wait never monopolizes a connection indefinitely.
    pub const MAX_SECONDS: u16 = 30;

    /// Validate at the wire boundary. Zero requests an immediate snapshot.
    /// # Errors
    /// Waiting longer than the shared finite call budget is refused.
    pub const fn new(seconds: u16) -> Result<Self, CreationError> {
        if seconds > Self::MAX_SECONDS {
            Err(CreationError::WaitTooLong)
        } else {
            Ok(Self(seconds))
        }
    }

    /// Reuse the node's existing per-origin and per-connection wait shares.
    #[must_use]
    pub fn seconds(self) -> Option<u32> {
        (self.0 != 0).then_some(u32::from(self.0))
    }
}
impl TryFrom<u16> for ObservationWait {
    type Error = CreationError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ObservationWait> for u16 {
    fn from(value: ObservationWait) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::CREATION_PROTOCOL;
    use crate::{Call, Method, MethodShape, Request as _, Scope};

    #[test]
    fn wait_is_bounded_and_initial_cursor_is_distinct_from_revision_zero() {
        assert_eq!(ObservationWait::default().seconds(), None);
        assert_eq!(
            ObservationWait::new(ObservationWait::MAX_SECONDS)
                .unwrap()
                .seconds(),
            Some(u32::from(ObservationWait::MAX_SECONDS))
        );
        assert!(serde_json::from_str::<ObservationWait>("31").is_err());
        let request = SubscribeCreation {
            identity: ObserveCreation {
                kind: Kind::Workspace,
                key: Key::parse("opaque key").unwrap(),
            },
            after: None,
            wait: ObservationWait::default(),
        };
        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: SubscribeCreation = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(request, decoded);
        assert_eq!(decoded.after, None);
    }

    #[test]
    fn observation_methods_use_the_shared_feature_gate_and_read_scope() {
        for method in [Method::ObserveCreation, Method::SubscribeCreation] {
            assert_eq!(method.since(), CREATION_PROTOCOL);
            assert_eq!(method.scope(), Some(Scope::Read));
            assert_eq!(method.shape(), MethodShape::Unary);
            let call = crate::rpc::samples::call(method).unwrap();
            assert_eq!(call.since(), CREATION_PROTOCOL);
            let encoded = serde_json::to_vec(&call).unwrap();
            let decoded: Call = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded.method(), method);
        }
        let observe = ObserveCreation {
            kind: Kind::Workspace,
            key: Key::parse("secret-shaped-fixture").unwrap(),
        };
        assert!(!format!("{observe:?}").contains("secret-shaped-fixture"));
        assert!(matches!(observe.into_call(), Call::ObserveCreation(_)));
    }
}
