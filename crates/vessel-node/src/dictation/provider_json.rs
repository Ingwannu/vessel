//! The bounded foreign JSON edge of the transcription API.
//!
//! A local Value is deliberate (T6): serde's typed data errors can quote an
//! unexpected string, which may be an echoed key. Only the validated Transcript
//! or a fixed typed failure leaves this module. No Value or response text survives.

use reqwest::StatusCode;
use serde_json::Value;
use vessel_llm::client::Redactor;
use vessel_proto::dictation::Transcript;

/// A foreign response cannot carry its payload into a diagnostic.
#[derive(Debug, thiserror::Error)]
pub(super) enum DecodeError {
    #[error("the dictation endpoint returned invalid JSON")]
    Json(#[source] serde_json::Error),
    #[error("the dictation endpoint returned no text field")]
    MissingText,
    #[error("the dictation endpoint refused the audio")]
    Status(StatusCode),
    #[error("the dictation endpoint refused the audio with an invalid error body")]
    StatusBody {
        status: StatusCode,
        #[source]
        source: serde_json::Error,
    },
    #[error("the dictation endpoint requires a larger audio segment")]
    TooSmall,
    #[error("the dictation endpoint echoed a credential")]
    CredentialEcho,
    #[error("the dictation endpoint returned an invalid transcript")]
    Transcript(#[source] vessel_proto::dictation::DictationDecodeError),
}

/// Decode only after the HTTP reader has enforced its response byte cap.
pub(super) fn decode(
    status: StatusCode,
    body: &[u8],
    redactor: &Redactor,
) -> Result<Transcript, DecodeError> {
    let value: Value = serde_json::from_slice(body).map_err(|source| {
        if status.is_success() {
            DecodeError::Json(source)
        } else {
            DecodeError::StatusBody { status, source }
        }
    })?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str);
        // Classification belongs at the foreign decoder edge. Decode escapes
        // first, and never let an unrelated field classify an error as TooSmall.
        return Err(match message {
            Some(text) if redactor.finds(text) => DecodeError::CredentialEcho,
            Some(text) if text.to_ascii_lowercase().contains("buffer too small") => {
                DecodeError::TooSmall
            }
            Some(_) | None => DecodeError::Status(status),
        });
    }
    let Value::Object(mut object) = value else {
        return Err(DecodeError::MissingText);
    };
    let Some(Value::String(text)) = object.remove("text") else {
        return Err(DecodeError::MissingText);
    };
    // JSON escapes have already been decoded. holds() would reinterpret
    // escape-looking speech; finds() checks this decoded text literally.
    if redactor.finds(&text) {
        return Err(DecodeError::CredentialEcho);
    }
    Transcript::try_from(text).map_err(DecodeError::Transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_llm::client::ApiKey;

    fn redactor() -> Redactor {
        Redactor::new(Some(&ApiKey::new(String::from(
            "synthetic-private-credential",
        ))))
    }

    #[test]
    fn malformed_types_never_quote_submitted_text() {
        for body in [
            br#""synthetic-private-credential""#.as_slice(),
            br#"{"text":{"synthetic-private-credential":true}}"#.as_slice(),
            br#"{"text":["synthetic-private-credential"]}"#.as_slice(),
            br#"{"text":false}"#.as_slice(),
        ] {
            let error = decode(StatusCode::OK, body, &redactor()).unwrap_err();
            assert!(matches!(error, DecodeError::MissingText));
            assert!(!error.to_string().contains("synthetic-private-credential"));
            assert!(!format!("{error:?}").contains("synthetic-private-credential"));
        }
    }

    #[test]
    fn echoed_credentials_are_refused_even_after_json_escape_decoding() {
        for body in [
            br#"{"text":"synthetic-private-credential"}"#.as_slice(),
            br#"{"text":"\u0073ynthetic-private-credential"}"#.as_slice(),
        ] {
            let error = decode(StatusCode::OK, body, &redactor()).unwrap_err();
            assert!(matches!(error, DecodeError::CredentialEcho));
        }
    }

    #[test]
    fn only_the_decoded_error_message_can_classify_a_small_segment() {
        let status = StatusCode::BAD_REQUEST;
        let small = br#"{"error":{"message":"BUFFER \u0074oo small"}}"#;
        assert!(matches!(
            decode(status, small, &redactor()),
            Err(DecodeError::TooSmall)
        ));
        let unrelated = br#"{"extra":"buffer too small","error":{"message":"other failure"}}"#;
        assert!(
            matches!(decode(status, unrelated, &redactor()), Err(DecodeError::Status(code)) if code == status)
        );
        let echoed = br#"{"error":{"message":"synthetic-private-credential buffer too small"}}"#;
        assert!(matches!(
            decode(status, echoed, &redactor()),
            Err(DecodeError::CredentialEcho)
        ));
    }

    #[test]
    fn decoded_speech_with_backslashes_is_not_parsed_as_json_again() {
        // Build the fixture through JSON so the backslashes represent ordinary
        // speech, rather than relying on hand-escaped invalid syntax.
        for speech in [
            r#"a path C:\work and a "quote""#,
            r"\u0073ynthetic-private-credential",
        ] {
            let body = serde_json::to_vec(&serde_json::json!({"text":speech})).unwrap();
            let text = decode(StatusCode::OK, &body, &redactor()).unwrap();
            assert!(text.as_str() == speech, "decoded speech was changed");
        }
    }

    #[test]
    fn malformed_error_json_keeps_its_status_and_syntax_cause_without_payload() {
        let expected = StatusCode::SERVICE_UNAVAILABLE;
        let error = decode(
            expected,
            br#"{"error":"synthetic-private-credential""#,
            &redactor(),
        )
        .unwrap_err();
        match &error {
            DecodeError::StatusBody { status, source } => {
                assert_eq!(*status, expected);
                assert!(source.is_eof());
            }
            DecodeError::Json(_)
            | DecodeError::MissingText
            | DecodeError::Status(_)
            | DecodeError::TooSmall
            | DecodeError::CredentialEcho
            | DecodeError::Transcript(_) => panic!("unexpected error category"),
        }
        assert!(std::error::Error::source(&error).is_some());
        assert!(!error.to_string().contains("synthetic-private-credential"));
        assert!(!format!("{error:?}").contains("synthetic-private-credential"));
    }

    #[test]
    fn empty_text_is_a_real_transcript_and_json_syntax_keeps_its_cause() {
        let text = decode(StatusCode::OK, br#"{"text":""}"#, &redactor()).unwrap();
        assert!(text.as_str().is_empty());
        let malformed = decode(StatusCode::OK, b"{", &redactor()).unwrap_err();
        assert!(matches!(malformed, DecodeError::Json(_)));
        assert!(std::error::Error::source(&malformed).is_some());
    }
}
