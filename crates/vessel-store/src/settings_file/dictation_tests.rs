//! Future fields must not erase known settings; the old file is copied first.
use super::*;
use vessel_proto::dictation::{DictationChange, DictationProvider, LocalModelId, SetDictation};
use vessel_proto::parking::{IdleMinutes, ParkAfter};
use vessel_proto::settings::{AgentDriver, SetAgentDrivers};

#[test]
fn a_later_dictation_file_keeps_every_known_value_and_is_copied_before_rewrite() {
    let endpoint: SetDictation = serde_json::from_str(
        r#"{
        "switch":"off", "provider":"endpoint", "language":"ko",
        "endpoint_model":"whisper-1", "endpoint_base":"https://example.test/v1",
        "key":"TRANSCRIPTION"
    }"#,
    )
    .unwrap();
    let local = SetDictation {
        local_model: Some(DictationChange::Set(LocalModelId::default_model())),
        provider: Some(DictationChange::Set(DictationProvider::Local)),
        ..SetDictation::default()
    };
    for dictation in [endpoint, local] {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let expected = SetNodeSettings {
            park_after: Some(ParkAfter::Idle {
                minutes: IdleMinutes::new(15).unwrap(),
            }),
            drivers: Some(SetAgentDrivers {
                codex: Some(AgentDriver::Direct),
                ..SetAgentDrivers::default()
            }),
            dictation: Some(dictation),
        };
        let mut future = serde_json::to_value(&expected).unwrap();
        let previous = future
            .get_mut("dictation")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future".to_owned(), serde_json::json!({"nested":true}));
        assert!(previous.is_none());
        let original = serde_json::to_vec(&future).unwrap();
        std::fs::write(data.settings_path(), &original).unwrap();
        let saved = data.read_settings().unwrap().unwrap();
        assert_eq!(
            saved, expected,
            "a known override was lost while reading a future field"
        );
        assert!(data.settings_hold_more_than(&saved).unwrap());
        let copy = data
            .set_aside_settings(Utc::now(), SetAside::Later)
            .unwrap();
        data.write_settings(&saved).unwrap();
        assert_eq!(std::fs::read(copy).unwrap(), original);
        assert_eq!(data.read_settings().unwrap().unwrap(), expected);
        assert!(!data.settings_hold_more_than(&expected).unwrap());
    }
}

#[test]
fn empty_and_future_only_dictation_blocks_do_not_add_saved_overrides() {
    for block in [serde_json::json!({}), serde_json::json!({"future":true})] {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let original = serde_json::to_vec(&serde_json::json!({
            "park_after":{"kind":"never"}, "dictation":block
        }))
        .unwrap();
        std::fs::write(data.settings_path(), &original).unwrap();
        let saved = data.read_settings().unwrap().unwrap();
        assert_eq!(saved.park_after, Some(ParkAfter::Never));
        assert!(saved.dictation.is_none());
        assert!(data.settings_hold_more_than(&saved).unwrap());
        let copy = data
            .set_aside_settings(Utc::now(), SetAside::Later)
            .unwrap();
        data.write_settings(&saved).unwrap();
        assert_eq!(std::fs::read(copy).unwrap(), original);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(data.settings_path()).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({"park_after":{"kind":"never"}}));
    }
}
