//! The new field is strict on patches, readable after a downgrade, and hidden before assignment.

use super::*;
use crate::{Forwarded, PROTOCOL_VERSION, ProtocolVersion, Reply};

fn settings() -> NodeSettings {
    NodeSettings {
        park_after: ParkAfter::Never,
        drivers: AgentDrivers::default(),
        metadata: MetadataSettings::new(vec![MetadataProvider {
            route: "fast".parse().unwrap(),
            model: None,
            effort: None,
        }])
        .unwrap(),
    }
}

#[test]
fn missing_metadata_defaults_to_automatic_and_reply_views_ignore_later_nested_keys() {
    let older: NodeSettings = serde_json::from_value(serde_json::json!({
        "park_after": ParkAfter::Never,
    }))
    .unwrap();
    assert!(older.metadata.is_automatic());
    let raw = serde_json::json!({"park_after":ParkAfter::Never,"metadata":{
        "future":true,"providers":[{"route":"fast","future":true,"effort":"future"}]
    }});
    let read: NodeSettings = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(
        read.metadata.providers()[0].effort,
        Some(MetadataEffort::Unknown)
    );
    assert!(serde_json::from_value::<SetNodeSettings>(raw).is_err());
}

#[test]
fn settings_fields_are_hidden_on_both_replies_and_all_envelope_paths() {
    let wrappers: [fn(Forwarded) -> Reply; 3] = [Reply::Forward, Reply::Relay, Reply::Deliver];
    for reply in [
        Reply::GetNodeSettings(settings()),
        Reply::SetNodeSettings(settings()),
    ] {
        let projected = reply
            .for_protocol(PROTOCOL_VERSION)
            .unwrap_or_else(|| reply.clone());
        let json = serde_json::to_string(&projected).unwrap();
        assert_eq!(json.contains("metadata"), crate::speaks_generated_names());
        for wrap in wrappers {
            let nested = wrap(Forwarded {
                reply: Box::new(reply.clone()),
            });
            let projected = nested
                .for_protocol(PROTOCOL_VERSION)
                .unwrap_or_else(|| nested.clone());
            assert_eq!(
                serde_json::to_string(&projected)
                    .unwrap()
                    .contains("metadata"),
                crate::speaks_generated_names()
            );
        }
    }
}

#[test]
fn an_assigned_threshold_keeps_metadata_only_for_readers_of_that_threshold() {
    let threshold = PROTOCOL_VERSION;
    let settings = settings();
    assert!(
        settings
            .for_protocol_with_names(threshold, None)
            .unwrap()
            .metadata
            .is_automatic()
    );
    assert!(
        settings
            .for_protocol_with_names(threshold, Some(threshold))
            .is_none()
    );
    let old = ProtocolVersion::new(threshold.get().checked_sub(1).unwrap());
    assert!(
        settings
            .for_protocol_with_names(old, Some(threshold))
            .unwrap()
            .metadata
            .is_automatic()
    );
}
