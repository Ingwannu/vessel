//! Session discovery request encoding and label selection (ADR 0063).
//! The default remains null so an unfiltered request reaches old peers.

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::SessionLabels;

/// `session.list`: every session the node knows, newest first, including
/// ended sessions restored from disk. `labels`, when set, keeps the
/// sessions whose current labels hold every pair (ADR 0063). No filter is
/// `null` on the wire, so a peer that predates labels still decodes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(type = "null | { labels?: SessionLabels }")
)]
#[non_exhaustive]
pub struct ListSessions {
    /// Every pair a session must carry. Empty means the whole list.
    pub labels: SessionLabels,
}

impl ListSessions {
    /// Require every pair while leaving any other discovery option at its default.
    /// A constructor keeps callers independent of future optional fields.
    #[must_use]
    pub fn matching(labels: SessionLabels) -> Self {
        Self { labels }
    }
}

impl Serialize for ListSessions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.labels.is_empty() {
            return serializer.serialize_unit();
        }
        #[derive(Serialize)]
        struct Body<'a> {
            labels: &'a SessionLabels,
        }
        Body {
            labels: &self.labels,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ListSessions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Body {
            #[serde(default)]
            labels: SessionLabels,
        }
        Ok(match Option::<Body>::deserialize(deserializer)? {
            None => Self::default(),
            Some(body) => Self {
                labels: body.labels,
            },
        })
    }
}

#[cfg(test)]
mod label_listing_tests {
    #[test]
    fn an_unfiltered_list_is_null_and_a_filter_needs_the_label_version() {
        use crate::lifecycle_version::SINCE;
        use crate::{Call, ListSessions, ProtocolVersion, Request};

        let all = ListSessions::default().into_call();
        let text = serde_json::to_string(&all).unwrap();
        assert_eq!(text, r#"{"method":"session.list","params":null}"#);
        assert_eq!(serde_json::from_str::<Call>(&text).unwrap(), all);
        assert!(Call::since(&all) < ProtocolVersion::new(SINCE));

        let filtered = ListSessions::matching(serde_json::from_str(r#"{"env":"prod"}"#).unwrap());
        let call = filtered.clone().into_call();
        let again = serde_json::to_string(&call).unwrap();
        assert_eq!(serde_json::from_str::<Call>(&again).unwrap(), call);
        assert_eq!(Call::since(&call), ProtocolVersion::new(SINCE));
        assert_eq!(
            serde_json::from_str::<Call>(r#"{"method":"session.list","params":{}}"#).unwrap(),
            ListSessions::default().into_call()
        );
    }
    #[test]
    fn null_defaults_but_a_present_invalid_map_is_refused() {
        use super::ListSessions;
        for text in ["null", "{}", r#"{"labels":{}}"#] {
            assert_eq!(
                serde_json::from_str::<ListSessions>(text).unwrap(),
                ListSessions::default()
            );
        }
        for text in [
            r#"{"labels":null}"#,
            r#"{"labels":{"":"v"}}"#,
            "[]",
            r#""bad""#,
        ] {
            assert!(serde_json::from_str::<ListSessions>(text).is_err());
        }
    }
}
