//! Provider metadata is accepted once and carried as a validated value.

use super::settings_values::{MAX_LANGUAGE_CHARS, MAX_MODEL_CHARS, MAX_URL_BYTES};
use super::*;

#[test]
fn defaults_and_catalogue_match_paseo_without_linking_a_model() {
    assert_eq!(Language::english().as_str(), "en");
    assert_eq!(EndpointModel::whisper().as_str(), "whisper-1");
    assert_eq!(EndpointBase::openai().as_str(), "https://api.openai.com/v1");
    assert_eq!(LocalModelId::DEFAULT.as_str(), "parakeet-tdt-0.6b-v2-int8");
    assert_eq!(LocalModelId::KNOWN.len(), 10);
    for model in LocalModelId::KNOWN {
        let decoded: LocalModelId = model.as_str().parse().unwrap();
        assert_eq!(&decoded, model);
        assert!(decoded.known());
    }
    let future: LocalModelId = "future-model".parse().unwrap();
    assert!(!future.known());
    let encoded = serde_json::to_string(&future).unwrap();
    assert_eq!(
        serde_json::from_str::<LocalModelId>(&encoded).unwrap(),
        future
    );
}

#[test]
fn endpoint_authorities_are_unambiguous_and_cleartext_is_loopback_only() {
    for valid in [
        "https://api.openai.com/v1",
        "https://example.com:443/v1",
        "http://localhost:1234/v1",
        "http://127.1.2.3/v1",
        "http://[::1]:1234/v1",
    ] {
        assert!(valid.parse::<EndpointBase>().is_ok());
    }
    for bad in [
        "http://example.com/v1",
        "http://127.0.0.1.example.com",
        "http://127.0.0.1@evil.example",
        "https://user:credential@example.com",
        "https://@example.com",
        "https://example.com#fragment",
        "https://example.com?query",
        "https://example.com\\evil",
        "https://",
        "https://:443/v1",
        "https://example.com:0",
        "https://example.com:65536",
        "https://example.com:word",
        "http://[::2]",
        "https://example.com\n",
    ] {
        let error = bad.parse::<EndpointBase>().unwrap_err();
        assert!(!error.to_string().contains("credential"));
        assert!(!format!("{error:?}").contains("credential"));
    }
    let base: EndpointBase = "HTTPS://EXAMPLE.COM:443/v1/".parse().unwrap();
    assert_eq!(base.as_str(), "https://example.com/v1");
    assert_eq!(
        base.transcription_url().unwrap().as_str(),
        "https://example.com/v1/audio/transcriptions"
    );
    let escaped: EndpointBase = "https://example.com/a%20b/".parse().unwrap();
    assert_eq!(
        escaped.transcription_url().unwrap().as_str(),
        "https://example.com/a%20b/audio/transcriptions"
    );
    let encoded = serde_json::to_string(&base).unwrap();
    assert_eq!(
        serde_json::from_str::<EndpointBase>(&encoded).unwrap(),
        base
    );
}

#[test]
fn metadata_bounds_use_characters_and_canonical_url_bytes() {
    assert!("a".repeat(MAX_LANGUAGE_CHARS).parse::<Language>().is_ok());
    for language in ["", "two words", "123", &"a".repeat(MAX_LANGUAGE_CHARS + 1)] {
        assert!(language.parse::<Language>().is_err());
    }
    assert!(
        "모"
            .repeat(MAX_MODEL_CHARS)
            .parse::<EndpointModel>()
            .is_ok()
    );
    for model in [
        "",
        "two words",
        "model\u{0000}",
        &"모".repeat(MAX_MODEL_CHARS + 1),
    ] {
        assert!(model.parse::<EndpointModel>().is_err());
    }
    for model in [
        "",
        "two words",
        "../model",
        &"a".repeat(MAX_MODEL_CHARS + 1),
    ] {
        assert!(model.parse::<LocalModelId>().is_err());
    }
    let prefix = "https://example.com/";
    let max = format!("{prefix}{}", "a".repeat(MAX_URL_BYTES - prefix.len()));
    assert!(max.parse::<EndpointBase>().is_ok());
    assert!(format!("{max}a").parse::<EndpointBase>().is_err());
    let escaped_too_long = format!("{prefix}{}", "한".repeat(MAX_URL_BYTES / "한".len()));
    assert!(escaped_too_long.parse::<EndpointBase>().is_err());
}
