//! MCP JSON decoding for session metadata and discovery. A tree exists only
//! at this external argument/schema edge; requests carry validated wire types.

use rmcp::model::JsonObject;
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use vessel_proto::{
    ErrorCode, LabelKey, LabelValue, ListSessions, ListingScope, ProtocolVersion, SessionLabels,
    SessionSelection, SessionSpec,
};

use super::{ToolError, VesselTools};

#[derive(Default, Deserialize)]
struct Creation {
    #[serde(default)]
    labels: SessionLabels,
    #[serde(default, deserialize_with = "present")]
    listing: Option<vessel_proto::SessionListing>,
}

#[derive(Default, Deserialize)]
struct Selection {
    #[serde(default, deserialize_with = "present")]
    scope: Option<ListingScope>,
    #[serde(default, deserialize_with = "present")]
    labels: Option<SessionLabels>,
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(decoder).map(Some)
}

fn decode<T: serde::de::DeserializeOwned>(args: &JsonObject) -> Result<T, ToolError> {
    let entries = args.iter().map(|(key, value)| (key.as_str(), value));
    T::deserialize(serde::de::value::MapDeserializer::<_, serde_json::Error>::new(entries))
        .map_err(ToolError::MetadataJson)
}

/// Creation metadata stays typed before any target lookup, write or spawn.
pub(super) fn creation(args: &JsonObject, spec: &mut SessionSpec) -> Result<(), ToolError> {
    let metadata: Creation = decode(args)?;
    spec.labels = metadata.labels;
    spec.listing = metadata.listing;
    Ok(())
}

/// A schedule targeting an existing session cannot silently discard overrides.
pub(super) fn existing_target(args: &JsonObject) -> Result<(), ToolError> {
    if args.contains_key("labels") || args.contains_key("listing") {
        return Err(ToolError::MetadataTarget);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyFallback {
    Allowed,
    Forbidden,
}

impl Selection {
    fn request(
        self,
        assigned: Option<ProtocolVersion>,
        peer: ProtocolVersion,
    ) -> (ListSessions, LegacyFallback) {
        let implicit = self.scope.is_none() && self.labels.is_none();
        if implicit && !assigned.is_some_and(|since| peer >= since) {
            return (ListSessions::default(), LegacyFallback::Forbidden);
        }
        let selection = SessionSelection {
            scope: self.scope.unwrap_or(ListingScope::Shown),
            labels: self.labels.unwrap_or_default(),
        };
        let fallback = if implicit {
            LegacyFallback::Allowed
        } else {
            LegacyFallback::Forbidden
        };
        (ListSessions::selected(selection), fallback)
    }
}

impl LegacyFallback {
    fn after(self, error: &ToolError) -> bool {
        match self {
            Self::Allowed => {
                matches!(error, ToolError::Node(error) if error.code() == Some(ErrorCode::Unsupported))
            }
            Self::Forbidden => false,
        }
    }
}

impl VesselTools {
    pub(super) async fn list_sessions(&self, args: &JsonObject) -> Result<Value, ToolError> {
        // Decode first: even a name-resolution RPC is work for invalid selectors.
        let selected: Selection = decode(args)?;
        let (request, fallback) = selected.request(
            vessel_proto::lifecycle_version::SINCE,
            self.client.protocol(),
        );
        let node = self.target(args).await?;
        let result = self.on(node, request).await;
        let sessions = if let Err(error) = &result
            && fallback.after(error)
        {
            // Only an implicit default may use legacy discovery through an older
            // fleet hop. Explicit scope/labels never get discarded; no other
            // refusal or transport error is retried.
            self.on(node, ListSessions::default()).await?
        } else {
            result?
        };
        Ok(json!(sessions))
    }
}

/// Schema limits come from the same validators, not another set of bounds.
pub(super) fn labels_property() -> Value {
    json!({"type":"object",
        "additionalProperties":{"type":"string", "maxLength":LabelValue::MAX_CHARS},
        "description":format!("Validated metadata: at most {} canonical keys; keys are trimmed then limited to {} Unicode scalars. Values are preserved. Every requested pair must match on discovery.", SessionLabels::MAX, LabelKey::MAX_CHARS)})
}

/// Session metadata is only a legacy/unbound override; canonical workspaces win.
pub(super) fn listing_property() -> Value {
    json!({"type":"string","enum":["shown","background"],
        "description":"Legacy/unbound creation visibility; omit to inherit. Bound workspace visibility remains authoritative. Set background only when asked."})
}

/// Default inclusion and inclusive discovery are different from archival.
pub(super) fn scope_property() -> Value {
    json!({"type":"string","enum":["shown","all"],
        "description":"Shown by default; all also includes background sessions. Does not change archival."})
}

#[cfg(test)]
mod tests;
