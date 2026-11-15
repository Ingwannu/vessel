//! Report identity is independent of an account label, credential, or local path.
use std::{fmt, str::FromStr, sync::Arc};

/// Paseo's account-key alphabet also bounds source namespaces; neither is a path.
pub const MAX_USAGE_ID_COMPONENT_BYTES: usize = 128;

/// No rejected value is retained, so errors cannot echo credential-like input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidUsageIdentity {
    /// Only nonempty ASCII alphanumeric, dot, dash and underscore components fit.
    #[error("usage identity components must contain 1 to {max} ASCII letters, digits, dots, dashes or underscores", max = MAX_USAGE_ID_COMPONENT_BYTES)]
    Component,
    /// The report id must have exactly one namespace separator.
    #[error("a usage report id must contain one source and one opaque account key")]
    Report,
}

fn component(text: &str) -> Result<Arc<str>, InvalidUsageIdentity> {
    if text.is_empty()
        || text.len() > MAX_USAGE_ID_COMPONENT_BYTES
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(InvalidUsageIdentity::Component);
    }
    Ok(Arc::from(text))
}

macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);
        impl $name {
            /// Borrow validated identity without another parse or allocation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl FromStr for $name {
            type Err = InvalidUsageIdentity;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                component(text).map(Self)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
                output.write_str(&self.0)
            }
        }
        crate::key::serde_via_text!($name);
    };
}
identity!(
    UsageSourceId,
    "A source namespace; account/window pins apply to every account of this source."
);
identity!(
    UsageAccountKey,
    "A source-provided opaque account key; never an email, token, login path or human label."
);

/// One canonical report identity. Consumers resolve it through discovery, not labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct UsageReportId {
    source: UsageSourceId,
    account: UsageAccountKey,
}
impl UsageReportId {
    /// Join already validated parts; no secret or display text participates.
    #[must_use]
    pub const fn new(source: UsageSourceId, account: UsageAccountKey) -> Self {
        Self { source, account }
    }
    /// The namespace used to resolve the report's source.
    #[must_use]
    pub const fn source(&self) -> &UsageSourceId {
        &self.source
    }
    /// Opaque source-owned identity, never interpreted by a client.
    #[must_use]
    pub const fn account(&self) -> &UsageAccountKey {
        &self.account
    }
}
impl fmt::Display for UsageReportId {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}:{}", self.source, self.account)
    }
}
impl FromStr for UsageReportId {
    type Err = InvalidUsageIdentity;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (source, account) = text.split_once(':').ok_or(InvalidUsageIdentity::Report)?;
        Ok(Self::new(source.parse()?, account.parse()?))
    }
}
crate::key::serde_via_text!(UsageReportId);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_report_ids_round_trip_without_human_account_labels() {
        let id: UsageReportId = "codex-usage-source:a83f01".parse().unwrap();
        assert_eq!(id.source().as_str(), "codex-usage-source");
        assert_eq!(id.account().as_str(), "a83f01");
        assert_eq!(id.to_string(), "codex-usage-source:a83f01");
        assert_eq!(
            serde_json::from_str::<UsageReportId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }
    #[test]
    fn identity_edges_reject_extra_separators_unicode_and_credential_like_text() {
        for text in [
            "",
            ":account",
            "source:",
            "source:key:extra",
            "source:user@example.test",
            "source:a/b",
            "source:한글",
            "source: key",
            "source:key\n",
        ] {
            assert!(text.parse::<UsageReportId>().is_err());
        }
        let maximum = "a".repeat(MAX_USAGE_ID_COMPONENT_BYTES);
        assert!(maximum.parse::<UsageSourceId>().is_ok());
        assert!(format!("{maximum}a").parse::<UsageSourceId>().is_err());
        let rejected = "not-a-key@example.test";
        let error = rejected.parse::<UsageAccountKey>().unwrap_err();
        assert!(!error.to_string().contains(rejected));
    }
}
