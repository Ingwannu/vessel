//! Native booleans decode once into their independent presentation meanings.
use serde::{Deserialize, Deserializer};
use vessel_proto::timeline::EmptyAnswerPolicy;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(from = "bool")]
pub(in crate::codex) enum NativeSelection {
    #[default]
    Single,
    Multiple,
}
impl From<bool> for NativeSelection {
    fn from(multiple: bool) -> Self {
        if multiple {
            Self::Multiple
        } else {
            Self::Single
        }
    }
}
impl NativeSelection {
    pub(in crate::codex) const fn is_multiple(self) -> bool {
        matches!(self, Self::Multiple)
    }
}

pub(super) fn empty_policy<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<EmptyAnswerPolicy, D::Error> {
    bool::deserialize(deserializer).map(|allowed| {
        if allowed {
            EmptyAnswerPolicy::AllowEmpty
        } else {
            EmptyAnswerPolicy::RequireText
        }
    })
}
