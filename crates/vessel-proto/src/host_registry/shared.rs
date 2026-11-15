//! Public inventory is a deliberate allowlist, never serialization of a private record.
use super::{HostDestination, HostLabel, HostMetadata, HostRegistrationId, MAX_HOST_REGISTRATIONS};
use crate::NodeId;
use serde::{Deserialize, Serialize, de};
use std::fmt;

/// Read public inventory across sources, without selecting or inheriting a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListSharedHostRegistrations;

/// Approved display fields only; a destination reference grants no route or authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SharedHostRegistration {
    /// Stable inventory identity, never a credential.
    pub registration_id: HostRegistrationId,
    /// Existing node identity and authority kind, without connection material.
    pub destination: HostDestination,
    /// Optional bounded name observed through the existing destination authority.
    pub destination_name: Option<HostLabel>,
    /// Explicitly public administrator-authored label.
    pub label: Option<HostLabel>,
    /// Explicitly public annotations readable by this node's local agents.
    pub annotations: HostMetadata,
}

/// Bounded public rows, including registrations that are disabled or unreachable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SharedHostRegistrations(Vec<SharedHostRegistration>);

/// Refuse excess rows rather than publishing a silently truncated inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("host inventory exceeds {MAX_HOST_REGISTRATIONS} registrations")]
pub struct TooManySharedHosts;

impl TryFrom<Vec<SharedHostRegistration>> for SharedHostRegistrations {
    type Error = TooManySharedHosts;
    fn try_from(rows: Vec<SharedHostRegistration>) -> Result<Self, Self::Error> {
        if rows.len() > MAX_HOST_REGISTRATIONS {
            Err(TooManySharedHosts)
        } else {
            Ok(Self(rows))
        }
    }
}
impl SharedHostRegistrations {
    /// Borrow the validated public inventory without exposing unchecked insertion.
    #[must_use]
    pub fn entries(&self) -> &[SharedHostRegistration] {
        &self.0
    }
    /// Enrich public names using existing observations, without adding connectivity claims.
    pub fn observe_names(&mut self, mut name: impl FnMut(NodeId) -> Option<HostLabel>) {
        for row in &mut self.0 {
            row.destination_name = name(row.destination.node());
        }
    }
}
impl<'de> Deserialize<'de> for SharedHostRegistrations {
    fn deserialize<D: de::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Rows;
        impl<'de> de::Visitor<'de> for Rows {
            type Value = SharedHostRegistrations;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "at most {MAX_HOST_REGISTRATIONS} public host rows"
                )
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut rows = Vec::new();
                while let Some(row) = seq.next_element()? {
                    if rows.len() == MAX_HOST_REGISTRATIONS {
                        return Err(de::Error::custom(TooManySharedHosts));
                    }
                    rows.push(row);
                }
                Ok(SharedHostRegistrations(rows))
            }
        }
        decoder.deserialize_seq(Rows)
    }
}

/// Node-wide public inventory, not a connection resolver or a full node-info response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SharedHostDirectory {
    /// Identity of the authenticated node supplying this inventory.
    pub node: NodeId,
    /// Only the approved public fields, across authenticated management sources.
    pub hosts: SharedHostRegistrations,
}

#[cfg(test)]
mod tests;
