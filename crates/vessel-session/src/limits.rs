//! Local launch association for quota reads. It contains no credential and
//! never enters the event log or protocol. A profile reload must not redirect
//! a still-running session to another home; only a successful launch/move
//! changes this watch value. Existing per-session ownership avoids a second
//! mutable registry and deliberately forgets unavailable stored associations.
use vessel_harness::LimitBinding;
use vessel_proto::limits::LimitFailure;

/// Nonsecret saved-sign-in location selected for this actual launch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LimitAssociation {
    /// No trustworthy launch association is available; never guess a home.
    #[default]
    Unavailable,
    /// The locally resolved binding, including its captured account label.
    Bound(LimitBinding),
    /// This launch explicitly uses an unsupported or different route.
    Refused(LimitFailure),
}
impl From<Result<LimitBinding, LimitFailure>> for LimitAssociation {
    fn from(binding: Result<LimitBinding, LimitFailure>) -> Self {
        match binding {
            Ok(binding) => Self::Bound(binding),
            Err(reason) => Self::Refused(reason),
        }
    }
}
