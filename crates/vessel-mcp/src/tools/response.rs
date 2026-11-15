//! MCP result serialization for the actual request's negotiated revision.
//!
//! The SDK models 2026's arbitrary structured JSON. Older revisions allow only
//! objects (or have no structured field), while Vessel's legacy text can be an
//! array. Preserve that text rather than wrapping it into a different API.
use rmcp::model::{CallToolResult, ProtocolVersion};
use serde_json::Value;

pub(super) fn success(value: Value, protocol: Option<&ProtocolVersion>) -> CallToolResult {
    let object = value.is_object();
    let mut result = CallToolResult::structured(value);
    let supports = protocol.is_some_and(|version| {
        version >= &ProtocolVersion::V_2025_06_18
            && (object || version >= &ProtocolVersion::V_2026_07_28)
    });
    if !supports {
        result.structured_content = None;
    }
    // rmcp's server handler already strips resultType for legacy peers.
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ContentBlock;
    use serde_json::json;

    #[derive(Clone, Copy)]
    enum Shape {
        Text,
        Object,
        Any,
    }

    #[test]
    fn each_revision_keeps_legacy_text_and_only_its_allowed_structured_shape() {
        for (protocol, shape) in [
            (None, Shape::Text),
            (Some(ProtocolVersion::V_2024_11_05), Shape::Text),
            (Some(ProtocolVersion::V_2025_03_26), Shape::Text),
            (Some(ProtocolVersion::V_2025_06_18), Shape::Object),
            (Some(ProtocolVersion::V_2025_11_25), Shape::Object),
            (Some(ProtocolVersion::V_2026_07_28), Shape::Any),
        ] {
            for value in [
                json!({"items": []}),
                json!([]),
                json!("text"),
                json!(7),
                json!(true),
                Value::Null,
            ] {
                let result = success(value.clone(), protocol.as_ref());
                assert_eq!(result.content.len(), 1);
                let ContentBlock::Text(text) = &result.content[0] else {
                    panic!("JSON text fallback is missing");
                };
                assert_eq!(serde_json::from_str::<Value>(&text.text).unwrap(), value);
                assert_eq!(result.is_error, Some(false));
                match shape {
                    Shape::Text => assert!(result.structured_content.is_none()),
                    Shape::Object if !value.is_object() => {
                        assert!(result.structured_content.is_none());
                    }
                    Shape::Object | Shape::Any => {
                        assert_eq!(result.structured_content, Some(value));
                    }
                }
            }
        }
    }
}
