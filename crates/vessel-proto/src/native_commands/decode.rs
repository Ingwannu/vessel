//! Serde internally tagged unit variants ignore payload fields even with
//! deny_unknown_fields. Empty struct variants validate the identical wire shape
//! while preserving the public unit variants used by the actor and transport.
use super::{GoalIntent, GoalObjective};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CheckedGoal {
    Usage {},
    Set { objective: GoalObjective },
    Pause {},
    Resume {},
    Clear {},
}

impl<'de> Deserialize<'de> for GoalIntent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match CheckedGoal::deserialize(deserializer)? {
            CheckedGoal::Usage {} => Self::Usage,
            CheckedGoal::Set { objective } => Self::Set { objective },
            CheckedGoal::Pause {} => Self::Pause,
            CheckedGoal::Resume {} => Self::Resume,
            CheckedGoal::Clear {} => Self::Clear,
        })
    }
}
