//! Settings-wire tests: no provider, secret lookup, persistence or model I/O.

use super::*;
use crate::dictation::{DictationProvider, EndpointBase, EndpointModel, Language, LocalModelId};

fn patch(json: &str) -> SetVoice {
    serde_json::from_str(json).unwrap()
}
fn endpoint() -> SetVoice {
    patch(
        r#"{"switch":"on","stt":{"provider":"endpoint","language":"ko","endpoint_model":"recognizer-custom","key":"VOICE_TEST_STT"},"tts":{"provider":"endpoint","endpoint_model":"speaker-custom","voice":"voice-custom","endpoint_base":"https://example.com/api","key":"VOICE_TEST_TTS"}}"#,
    )
}
fn read(value: &SetVoice) -> VoiceSettings {
    value.resolve(KeyPresence::Absent, KeyPresence::Absent)
}

#[test]
fn opt_in_and_independent_site_defaults_do_not_imply_model_readiness() {
    let defaults = VoiceSettings::default();
    assert_eq!(defaults.switch, DictationSwitch::Off);
    assert_eq!(defaults.stt.provider, DictationProvider::Local);
    assert_eq!(defaults.tts.provider, DictationProvider::Local);
    assert_eq!(defaults.stt.language, Language::english());
    assert_eq!(defaults.stt.local_model, LocalModelId::DEFAULT);
    assert_eq!(defaults.stt.endpoint_model, EndpointModel::whisper());
    assert_eq!(defaults.tts.local_model, LocalTtsModelId::DEFAULT);
    assert_eq!(defaults.tts.endpoint_model.as_str(), "tts-1");
    assert_eq!(defaults.tts.voice.as_str(), "alloy");
    assert_eq!(defaults.tts.endpoint_base, EndpointBase::openai());
    assert_eq!(defaults.stt.key, KeyPresence::Absent);
    assert_eq!(defaults.tts.key, KeyPresence::Absent);
    assert_eq!(serde_json::to_string(&SetVoice::default()).unwrap(), "{}");
    let wire = serde_json::to_string(&defaults).unwrap();
    assert_eq!(
        serde_json::from_str::<VoiceSettings>(&wire).unwrap(),
        defaults
    );
}

#[test]
fn deep_site_edits_preserve_the_other_site_and_omitted_references() {
    let base = endpoint();
    let same = SetVoice::default().over(base.clone()).unwrap();
    assert_eq!(same, base);
    let edited = patch(r#"{"stt":{"language":"ja"},"tts":{"voice":"next-voice"}}"#)
        .over(base.clone())
        .unwrap();
    let view = read(&edited);
    assert_eq!(view.stt.language.as_str(), "ja");
    assert_eq!(view.stt.endpoint_model.as_str(), "recognizer-custom");
    assert_eq!(view.tts.endpoint_model.as_str(), "speaker-custom");
    assert_eq!(view.tts.voice.as_str(), "next-voice");
    assert_eq!(edited.stt_key_name(), base.stt_key_name());
    assert_eq!(edited.tts_key_name(), base.tts_key_name());
    let stt_only = patch(r#"{"stt":{"language":"fr"}}"#)
        .over(base.clone())
        .unwrap();
    assert_eq!(stt_only.tts, base.tts);
    let tts_only = patch(r#"{"tts":{"voice":"other"}}"#)
        .over(base.clone())
        .unwrap();
    assert_eq!(tts_only.stt, base.stt);
}

#[test]
fn provider_change_clears_only_its_site_before_applying_new_options() {
    let changed = patch(r#"{"stt":{"provider":"local","language":"ja"},"tts":{"provider":"local","local_model":"piper-fr-siwis"}}"#)
        .over(endpoint()).unwrap();
    let view = read(&changed);
    assert_eq!(view.switch, DictationSwitch::On);
    assert_eq!(view.stt.provider, DictationProvider::Local);
    assert_eq!(view.stt.language.as_str(), "ja");
    assert_eq!(view.stt.endpoint_model, EndpointModel::whisper());
    assert_eq!(view.tts.local_model, LocalTtsModelId::PiperFrench);
    assert_eq!(view.tts.endpoint_model, TtsModel::default());
    assert!(changed.stt_key_name().is_none());
    assert!(changed.tts_key_name().is_none());
    let saved = patch(
        r#"{"stt":{"language":"ko","local_model":"whisper-base"},"tts":{"local_model":"piper-fr-siwis"}}"#,
    );
    let same = patch(r#"{"stt":{"provider":"local"},"tts":{"provider":"local"}}"#)
        .over(saved.clone())
        .unwrap();
    assert_eq!(read(&same).stt.language.as_str(), "ko");
    assert_eq!(read(&same).tts.local_model, LocalTtsModelId::PiperFrench);
    let reset = patch(r#"{"stt":{"provider":null},"tts":{"provider":null}}"#)
        .over(saved)
        .unwrap();
    assert_eq!(read(&reset), VoiceSettings::default());
}

#[test]
fn null_and_omission_are_distinct_at_every_settings_level() {
    let resets = patch(r#"{"switch":null,"stt":null,"tts":null}"#);
    assert_eq!(
        serde_json::to_string(&resets).unwrap(),
        r#"{"switch":null,"stt":null,"tts":null}"#
    );
    let cleared = resets.over(endpoint()).unwrap();
    assert_eq!(cleared, SetVoice::default());
    let reset = patch(r#"{"stt":{"key":null},"tts":{"key":null,"voice":null}}"#);
    assert!(
        serde_json::to_string(&reset)
            .unwrap()
            .contains("\"key\":null")
    );
    let cleared = reset.over(endpoint()).unwrap();
    assert!(cleared.stt_key_name().is_none());
    assert!(cleared.tts_key_name().is_none());
    assert_eq!(read(&cleared).tts.voice, TtsVoice::default());
    assert_eq!(
        read(&cleared).stt.endpoint_model.as_str(),
        "recognizer-custom"
    );
    assert!(!serde_json::to_string(&cleared).unwrap().contains("null"));
}

#[test]
fn unknown_sites_survive_unrelated_disable_but_cannot_be_newly_selected() {
    let future = patch(
        r#"{"switch":"on","stt":{"provider":"future-stt","local_model":"future-asr"},"tts":{"provider":"future-tts","local_model":"future-tts"}}"#,
    );
    let disabled = patch(r#"{"switch":"off"}"#).over(future.clone()).unwrap();
    assert_eq!(disabled.stt, future.stt);
    assert_eq!(disabled.tts, future.tts);
    let view = read(&disabled);
    assert_eq!(view.switch, DictationSwitch::Off);
    assert_eq!(view.stt.provider, DictationProvider::Unknown);
    assert_eq!(view.tts.provider, DictationProvider::Unknown);
    assert!(!view.tts.local_model.known());
    for json in [
        r#"{"switch":"future"}"#,
        r#"{"stt":{"provider":"future"}}"#,
        r#"{"tts":{"provider":"future"}}"#,
        r#"{"tts":{"local_model":"future-model"}}"#,
    ] {
        assert!(patch(json).over(SetVoice::default()).is_err());
    }
    assert!(
        patch(r#"{"tts":{"voice":"change"}}"#)
            .over(future.clone())
            .is_err()
    );
    let recovered = patch(r#"{"stt":null,"tts":{"provider":"local"}}"#)
        .over(future)
        .unwrap();
    assert_eq!(read(&recovered).tts.provider, DictationProvider::Local);
    assert_eq!(read(&recovered).stt.provider, DictationProvider::Local);
}

#[test]
fn inactive_fields_and_independent_stt_switch_are_refused() {
    for json in [
        r#"{"tts":{"voice":"other"}}"#,
        r#"{"tts":{"endpoint_model":"other"}}"#,
        r#"{"tts":{"key":"VOICE_TEST_TTS"}}"#,
        r#"{"tts":{"endpoint_base":"https://example.com"}}"#,
        r#"{"tts":{"provider":"endpoint","local_model":"piper-fr-siwis"}}"#,
        r#"{"stt":{"endpoint_model":"other"}}"#,
    ] {
        assert!(patch(json).over(SetVoice::default()).is_err());
    }
    assert!(serde_json::from_str::<SetVoice>(r#"{"stt":{"switch":"on"}}"#).is_err());
    assert!(serde_json::from_str::<SetVoice>(r#"{"extra":true}"#).is_err());
}

#[test]
fn reads_expose_presence_only_and_diagnostics_hide_configuration() {
    let selected = endpoint();
    let view = selected.resolve(KeyPresence::Present, KeyPresence::Absent);
    let wire = serde_json::to_string(&view).unwrap();
    assert!(wire.contains("\"key\":\"present\""));
    assert!(wire.contains("\"key\":\"absent\""));
    assert!(!wire.contains("VOICE_TEST_"));
    let diagnostics = format!(
        "{selected:?} {view:?} {:?} {:?} {:?} {:?}",
        view.stt, view.tts, view.tts.endpoint_model, view.tts.voice
    );
    for private in [
        "VOICE_TEST_",
        "recognizer-custom",
        "speaker-custom",
        "voice-custom",
        "example.com",
    ] {
        assert!(!diagnostics.contains(private));
    }
    for site in ["stt", "tts"] {
        for raw in [
            "\"inline-synthetic-credential\"",
            "1234567890123456789",
            "true",
        ] {
            let wire = format!(r#"{{"{site}":{{"key":{raw}}}}}"#);
            let error = serde_json::from_str::<SetVoice>(&wire).unwrap_err();
            assert!(!error.to_string().contains(raw));
            assert!(!format!("{error:?}").contains(raw));
        }
    }
}

#[test]
fn distinct_endpoint_identifiers_reuse_the_bounded_validator() {
    for text in ["custom-identifier", "alloy", "tts-1", &"a".repeat(64)] {
        let model = text.parse::<TtsModel>().unwrap();
        let voice = text.parse::<TtsVoice>().unwrap();
        assert_eq!(model.as_str(), text);
        assert_eq!(voice.as_str(), text);
        let model_wire = serde_json::to_string(&model).unwrap();
        assert_eq!(
            serde_json::from_str::<TtsModel>(&model_wire).unwrap(),
            model
        );
        let voice_wire = serde_json::to_string(&voice).unwrap();
        assert_eq!(
            serde_json::from_str::<TtsVoice>(&voice_wire).unwrap(),
            voice
        );
    }
    for bad in ["", "two words", "private\nvalue", &"a".repeat(65)] {
        for error in [
            bad.parse::<TtsModel>().unwrap_err(),
            bad.parse::<TtsVoice>().unwrap_err(),
        ] {
            if !bad.is_empty() {
                assert!(!error.to_string().contains(bad));
                assert!(!format!("{error:?}").contains(bad));
            }
        }
    }
}

#[test]
fn local_tts_catalogue_preserves_future_tokens_without_asr_role_confusion() {
    assert_eq!(LocalTtsModelId::KNOWN.len(), 9);
    for model in LocalTtsModelId::KNOWN {
        assert!(model.known());
        assert_eq!(model.as_str().parse::<LocalTtsModelId>().unwrap(), *model);
        assert!(
            patch(&format!(r#"{{"tts":{{"local_model":"{model}"}}}}"#))
                .over(SetVoice::default())
                .is_ok()
        );
    }
    for future in ["future-voice", "whisper-base"] {
        let model = future.parse::<LocalTtsModelId>().unwrap();
        assert!(!model.known());
        assert_eq!(model.as_str(), future);
        assert!(!format!("{model:?}").contains(future));
        assert!(
            patch(&format!(r#"{{"tts":{{"local_model":"{future}"}}}}"#))
                .over(SetVoice::default())
                .is_err()
        );
    }
    for bad in ["", "two words", "한글", &"a".repeat(65)] {
        assert!(bad.parse::<LocalTtsModelId>().is_err());
    }
}

#[cfg(feature = "ts")]
#[test]
fn typescript_settings_keep_nullable_patches_and_string_identifiers() {
    use ts_rs::TS as _;
    // These DTOs are not in the wire graph until NodeSettings wiring. Do not
    // add alias classifications for undeclared values merely for this local test.
    let config = ts_rs::Config::new();
    assert_eq!(
        DictationChange::<TtsVoice>::inline(&config),
        "TtsVoice | null"
    );
    assert_eq!(TtsVoice::inline(&config), "string");
    assert_eq!(TtsModel::inline(&config), "string");
    assert_eq!(LocalTtsModelId::inline(&config), "string");
}
