//! Draft identity belongs to local composition, independent of node session lifetime.
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use vessel_proto::SessionId;

/// A client-minted draft UUID; no public conversion to a node `SessionId` exists.
/// The private backing reuses the protocol's UUID generator without a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DraftId(SessionId);
impl DraftId {
    /// Mint before accepting a seed; layout itself never creates an agent.
    #[must_use]
    pub fn random() -> Self {
        Self(SessionId::random())
    }
}
impl fmt::Display for DraftId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
impl FromStr for DraftId {
    type Err = <SessionId as FromStr>::Err;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn draft_survives_storage_without_becoming_a_new_instance() {
        let draft = DraftId::random();
        assert_eq!(draft.to_string().parse::<DraftId>().unwrap(), draft);
        assert_eq!(
            serde_json::from_str::<DraftId>(&serde_json::to_string(&draft).unwrap()).unwrap(),
            draft
        );
        assert_ne!(DraftId::random(), draft);
    }
}
