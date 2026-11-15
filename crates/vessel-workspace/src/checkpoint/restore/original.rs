//! Fresh admission uses full observations; recovery uses persisted stable objects.
use crate::checkpoint::rooted::Identity;
use vessel_proto::checkpoint::ObjectFingerprint;

#[derive(Debug, Clone, Copy)]
pub(super) enum Original {
    Absent,
    Fresh(Identity),
    Recovered(ObjectFingerprint),
}
impl From<Option<Identity>> for Original {
    fn from(value: Option<Identity>) -> Self {
        value.map_or(Self::Absent, Self::Fresh)
    }
}
impl Original {
    pub(super) fn fresh_matches(self, current: Option<Identity>) -> bool {
        match self {
            Self::Absent => current.is_none(),
            Self::Fresh(expected) => current == Some(expected),
            Self::Recovered(_) => false,
        }
    }
    pub(super) fn object(self) -> Option<ObjectFingerprint> {
        match self {
            Self::Absent => None,
            Self::Fresh(identity) => Some(object(identity)),
            Self::Recovered(object) => Some(object),
        }
    }
}
pub(super) fn object(identity: Identity) -> ObjectFingerprint {
    identity.object_fingerprint()
}
