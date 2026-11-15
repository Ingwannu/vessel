//! Defaults, explicit resets and provider-scoped changes share one typed contract.

use super::*;

fn patch(json: &str) -> SetDictation {
    serde_json::from_str(json).unwrap()
}

fn endpoint() -> SetDictation {
    patch(
        r#"{"switch":"off","provider":"endpoint","language":"ko","endpoint_model":"custom-model","endpoint_base":"https://example.com/api","key":"DICTATION_TEST_KEY"}"#,
    )
}

#[test]
fn defaults_match_paseo_and_readers_never_receive_a_secret_name() {
    let defaults = DictationSettings::default();
    assert!(defaults.is_default());
    assert_eq!(defaults.switch, DictationSwitch::On);
    assert_eq!(defaults.provider, DictationProvider::Local);
    assert_eq!(defaults.language, Language::english());
    assert_eq!(defaults.local_model, LocalModelId::DEFAULT);
    assert_eq!(defaults.endpoint_model, EndpointModel::whisper());
    assert_eq!(defaults.endpoint_base, EndpointBase::openai());
    let view = endpoint().resolve(KeyPresence::Present);
    let wire = serde_json::to_string(&view).unwrap();
    assert!(!wire.contains("DICTATION_TEST_KEY"));
    assert!(wire.contains("\"key\":\"present\""));
    assert_eq!(
        serde_json::from_str::<DictationSettings>(&wire).unwrap(),
        view
    );
}

#[test]
fn changes_within_a_provider_keep_unspecified_fields() {
    let merged = patch(r#"{"provider":"endpoint","endpoint_model":"next-model"}"#)
        .over(endpoint())
        .unwrap();
    let view = merged.resolve(KeyPresence::Present);
    assert_eq!(view.switch, DictationSwitch::Off);
    assert_eq!(view.provider, DictationProvider::Endpoint);
    assert_eq!(view.language.as_str(), "ko");
    assert_eq!(view.endpoint_model.as_str(), "next-model");
    assert!(merged.key_name().is_some());
}

#[test]
fn provider_changes_reset_site_fields_before_applying_the_patch() {
    let merged = patch(r#"{"provider":"local","language":"ja"}"#)
        .over(endpoint())
        .unwrap();
    let view = merged.resolve(KeyPresence::Absent);
    assert_eq!(view.switch, DictationSwitch::Off);
    assert_eq!(view.provider, DictationProvider::Local);
    assert_eq!(view.language.as_str(), "ja");
    assert_eq!(view.endpoint_model, EndpointModel::whisper());
    assert_eq!(view.endpoint_base, EndpointBase::openai());
    assert!(merged.key_name().is_none());
}

#[test]
fn resetting_an_already_local_provider_still_clears_its_site() {
    let saved = patch(
        r#"{"switch":"off","provider":"local","language":"ko","local_model":"whisper-base"}"#,
    );
    let kept = patch(r#"{"provider":"local"}"#)
        .over(saved.clone())
        .unwrap();
    assert_eq!(kept.resolve(KeyPresence::Absent).language.as_str(), "ko");
    let reset = patch(r#"{"provider":null}"#).over(saved).unwrap();
    let view = reset.resolve(KeyPresence::Absent);
    assert_eq!(view.switch, DictationSwitch::Off);
    assert_eq!(view.provider, DictationProvider::Local);
    assert_eq!(view.language, Language::english());
    assert_eq!(view.local_model, LocalModelId::DEFAULT);
}

#[test]
fn omission_keeps_a_key_and_null_clears_the_reference_only() {
    let selected = endpoint();
    assert_eq!(
        SetDictation::default().over(selected.clone()).unwrap(),
        selected
    );
    let clear = patch(r#"{"key":null}"#);
    assert_eq!(serde_json::to_string(&clear).unwrap(), r#"{"key":null}"#);
    let cleared = clear.over(selected).unwrap();
    assert!(cleared.key_name().is_none());
    assert!(
        !serde_json::to_string(&cleared)
            .unwrap()
            .contains("DICTATION_TEST_KEY")
    );
}

#[test]
fn every_present_null_is_a_reset_and_is_normalized_out_of_saved_overrides() {
    for field in [
        "switch",
        "provider",
        "language",
        "endpoint_model",
        "endpoint_base",
        "key",
    ] {
        let reset = patch(&format!(r#"{{"{field}":null}}"#));
        let wire = serde_json::to_value(&reset).unwrap();
        assert_eq!(wire.get(field), Some(&serde_json::Value::Null));
        let after = reset.over(endpoint()).unwrap();
        assert!(serde_json::to_value(after).unwrap().get(field).is_none());
    }
    let local = patch(r#"{"local_model":"whisper-base"}"#);
    let reset = patch(r#"{"local_model":null}"#);
    assert_eq!(
        serde_json::to_string(&reset).unwrap(),
        r#"{"local_model":null}"#
    );
    assert!(reset.over(local).unwrap().local_model.is_none());
}

#[test]
fn future_choices_remain_readable_but_are_refused_when_applied() {
    for json in [
        r#"{"switch":"future"}"#,
        r#"{"provider":"future"}"#,
        r#"{"local_model":"future-model"}"#,
    ] {
        let change = patch(json);
        assert!(change.check().is_err());
        assert!(change.over(SetDictation::default()).is_err());
    }
    for model in LocalModelId::KNOWN {
        assert!(
            SetDictation {
                local_model: Some(DictationChange::Set(model.clone())),
                ..SetDictation::default()
            }
            .over(SetDictation::default())
            .is_ok()
        );
    }
    assert!(serde_json::from_str::<SetDictation>(r#"{"future_field":true}"#).is_err());
}

#[test]
fn provider_inapplicable_fields_are_refused_instead_of_kept_unused() {
    let change = patch(r#"{"endpoint_base":"https://example.com"}"#);
    assert!(matches!(
        change.clone().over(SetDictation::default()),
        Err(DictationDecodeError::EndpointFields)
    ));
    assert!(change.over(patch(r#"{"provider":"endpoint"}"#)).is_ok());
    assert!(matches!(
        patch(r#"{"local_model":"whisper-base"}"#).over(endpoint()),
        Err(DictationDecodeError::LocalFields)
    ));
}

#[test]
fn secret_reference_diagnostics_hide_invalid_and_valid_looking_pastes() {
    let pasted = "synthetic-inline-credential-not-a-name";
    let wire = format!(r#"{{"key":"{pasted}"}}"#);
    let error = serde_json::from_str::<SetDictation>(&wire).unwrap_err();
    assert!(!error.to_string().contains(pasted));
    assert!(!format!("{error:?}").contains(pasted));
    for scalar in [
        "1234567890123456789",
        "-1234567890123456789",
        "123456789.012345",
        "true",
    ] {
        let wire = format!(r#"{{"key":{scalar}}}"#);
        let error = serde_json::from_str::<SetDictation>(&wire).unwrap_err();
        assert!(!error.to_string().contains(scalar));
        assert!(!format!("{error:?}").contains(scalar));
    }
    let pasted = "SYNTHETIC_PASTED_CREDENTIAL";
    let change = patch(&format!(r#"{{"key":"{pasted}"}}"#));
    assert!(!format!("{change:?}").contains(pasted));
}

#[cfg(feature = "ts")]
#[test]
fn typescript_changes_keep_the_nullable_reset_representation() {
    use ts_rs::TS as _;
    assert_eq!(
        DictationChange::<String>::inline(&ts_rs::Config::new()),
        "string | null"
    );
}
