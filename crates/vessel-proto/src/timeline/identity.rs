//! Validated external identities; none of these values confers node authority.

use std::{fmt, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

/// The shared byte bound keeps external labels and ids below frame budgets.
pub const MAX_TIMELINE_IDENTITY_BYTES: usize = 1024;

/// Refusing a malformed identity does not echo provider or user data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TimelineIdentityError {
    /// Identity-bearing text must fit the same rule on wire and in storage.
    #[error(
        "timeline identity must be nonempty, control-free and at most {MAX_TIMELINE_IDENTITY_BYTES} bytes"
    )]
    Invalid,
}

macro_rules! identity {
    ($name:ident, $why:literal) => {
        #[doc = $why]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// Borrow the validated spelling without allocating for each reader.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = TimelineIdentityError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.trim().is_empty()
                    || value.len() > MAX_TIMELINE_IDENTITY_BYTES
                    || value.chars().any(char::is_control)
                {
                    return Err(TimelineIdentityError::Invalid);
                }
                Ok(Self(Arc::from(value)))
            }
        }

        impl TryFrom<String> for $name {
            type Error = TimelineIdentityError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0.to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

identity!(
    ClientMessageId,
    "A caller-retained identity for retrying one prompt without duplication."
);
identity!(
    ExternalAgentId,
    "Provenance from another agent system, never a local session capability."
);
identity!(
    ExternalHostId,
    "A provenance host id which need not be a Vessel node id."
);
identity!(
    PluginItemId,
    "A plugin's stable item id, scoped by its plugin identity."
);
identity!(
    TimelinePluginId,
    "The extension owning a timeline item, distinct from its local item id."
);
identity!(
    PluginKind,
    "An extension contribution kind, interpreted only by its plugin runtime."
);
identity!(
    TimelineName,
    "A display name or tool label whose spelling must remain intact."
);

impl ClientMessageId {
    /// Mint once before prompt preparation; retain this identity across delivery retries.
    #[must_use]
    pub fn random() -> Self {
        // A canonical UUID has no controls and fits the identity bound by construction.
        Self(Arc::from(uuid::Uuid::new_v4().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deserialization_uses_the_validating_constructor() {
        for value in ["", " ", "line\nbreak", "tab\tkey", "\0"] {
            assert!(value.parse::<ClientMessageId>().is_err());
            let json = serde_json::to_string(value).unwrap();
            assert!(serde_json::from_str::<ClientMessageId>(&json).is_err());
        }
        let at_bound = "a".repeat(MAX_TIMELINE_IDENTITY_BYTES);
        assert!(at_bound.parse::<ClientMessageId>().is_ok());
        assert!(format!("{at_bound}a").parse::<ClientMessageId>().is_err());
        let unicode = "한".repeat(MAX_TIMELINE_IDENTITY_BYTES);
        assert!(unicode.parse::<ClientMessageId>().is_err());
        assert_eq!(
            "external/agent"
                .parse::<ExternalAgentId>()
                .unwrap()
                .as_str(),
            "external/agent"
        );
    }
}
