//! The new field is strict on patches, readable after a downgrade, and hidden before assignment.

use super::*;
use crate::{Forwarded, PROTOCOL_VERSION, ProtocolVersion, Reply};

fn settings() -> NodeSettings {
    NodeSettings {
        park_after: ParkAfter::Never,
        drivers: AgentDrivers::default(),
        restart_recovery: RestartRecovery::default(),
        metadata: Some(
            MetadataSettings::new(vec![MetadataProvider {
                route: "fast".parse().unwrap(),
                model: None,
                effort: None,
            }])
            .unwrap(),
        ),
    }
}

#[test]
fn missing_metadata_means_unsupported_and_reply_views_ignore_later_nested_keys() {
    let older: NodeSettings = serde_json::from_value(serde_json::json!({
        "park_after": ParkAfter::Never,
    }))
    .unwrap();
    assert!(older.metadata.is_none());
    let raw = serde_json::json!({"park_after":ParkAfter::Never,"metadata":{
        "future":true,"providers":[{"route":"fast","future":true,"effort":"future"}]
    }});
    let read: NodeSettings = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(
        read.metadata.as_ref().unwrap().providers()[0].effort,
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
            .is_none()
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
            .is_none()
    );
}

#[test]
fn metadata_masking_keeps_the_heads_recovery_projection_even_when_metadata_is_default() {
    let mut original = settings();
    original.restart_recovery = RestartRecovery {
        enabled: true,
        prompt: "synthetic recovery instruction".parse().unwrap(),
    };
    for metadata in [
        original.metadata.clone(),
        Some(MetadataSettings::default()),
        None,
    ] {
        original.metadata = metadata;
        let projected = original
            .for_protocol(PROTOCOL_VERSION)
            .unwrap_or_else(|| original.clone());
        let json = serde_json::to_string(&projected).unwrap();
        let recovery_visible = crate::restart::RESTART_RECOVERY_PROTOCOL
            .is_some_and(|required| PROTOCOL_VERSION >= required);
        assert_eq!(json.contains("restart_recovery"), recovery_visible);
        assert_eq!(
            json.contains("metadata"),
            crate::speaks_generated_names() && original.metadata.is_some()
        );
    }
}

#[test]
fn assigned_automatic_is_present_while_unassigned_peers_receive_no_metadata() {
    let mut automatic = settings();
    automatic.metadata = Some(MetadataSettings::default());
    let required = PROTOCOL_VERSION;
    let supported = automatic
        .for_protocol_with_names(required, Some(required))
        .unwrap_or_else(|| automatic.clone());
    let wire = serde_json::to_value(&supported).unwrap();
    assert_eq!(
        wire.get("metadata"),
        Some(&serde_json::json!({"providers":[]}))
    );
    let hidden = automatic.for_protocol_with_names(required, None).unwrap();
    assert!(hidden.metadata.is_none());
    assert!(
        serde_json::to_value(&hidden)
            .unwrap()
            .get("metadata")
            .is_none()
    );
}
