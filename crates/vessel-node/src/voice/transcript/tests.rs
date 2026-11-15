use super::*;

#[test]
fn logprob_request_policy_and_low_confidence_never_leak_rejected_text() {
    assert_eq!(
        Scores::for_model(&"whisper-1".parse().unwrap()),
        Scores::Unavailable
    );
    for name in ["gpt-4o-transcribe", "gpt-4o-mini-transcribe"] {
        assert_eq!(Scores::for_model(&name.parse().unwrap()), Scores::Requested);
    }
    let body = br#"{"text":"private utterance","logprobs":[{"logprob":-4.0,"token":"private"},{"token":"x","logprob":-3.0}]}"#;
    let rejected = decode(body, Scores::Requested).unwrap();
    assert_eq!(rejected, VoiceTranscript::Rejected);
    assert!(!format!("{rejected:?}").contains("private"));
    assert!(matches!(
        decode(body, Scores::Unavailable).unwrap(),
        VoiceTranscript::Accepted {
            confidence: Confidence::Unspecified,
            ..
        }
    ));
}

#[test]
fn threshold_is_strict_and_missing_or_malformed_arrays_do_not_invent_confidence() {
    let exact = br#"{"text":"hello","logprobs":[{"token":"x","logprob":-3.0}]}"#;
    assert!(matches!(
        decode(exact, Scores::Requested).unwrap(),
        VoiceTranscript::Accepted {
            confidence: Confidence::Accepted,
            ..
        }
    ));
    for logprobs in [
        "null",
        "true",
        "-17",
        "\"private metadata\"",
        "[{\"token\":\"x\",\"logprob\":-8,\"bytes\":null}]",
        "[{\"token\":false,\"logprob\":-8}]",
        "[]",
        "[{}]",
        "[{\"logprob\":-8}]",
        "[{\"token\":\"x\",\"logprob\":-8,\"bytes\":[\"bad\"]}]",
        "[{\"logprob\":\"private\"}]",
        "{}",
    ] {
        let body = format!("{{\"text\":\"hello\",\"logprobs\":{logprobs}}}");
        assert!(matches!(
            decode(body.as_bytes(), Scores::Requested).unwrap(),
            VoiceTranscript::Accepted {
                confidence: Confidence::Unspecified,
                ..
            }
        ));
    }
    assert!(matches!(
        decode(br#"{"text":"hello"}"#, Scores::Requested).unwrap(),
        VoiceTranscript::Accepted {
            confidence: Confidence::Unspecified,
            ..
        }
    ));
}

#[test]
fn empty_invalid_large_or_overflowing_responses_fail_without_foreign_diagnostics() {
    assert_eq!(
        decode(br#"{"text":"\ufeff\t "}"#, Scores::Requested).unwrap(),
        VoiceTranscript::Empty
    );
    for body in [br#"{"text":17}"#.as_slice(), br#"{"text":"private""#] {
        let error = decode(body, Scores::Requested).unwrap_err();
        assert_eq!(error, TranscriptError::Invalid);
        assert!(!format!("{error:?}").contains("private"));
    }
    assert_eq!(
        decode(
            &vec![b'x'; MAX_RESPONSE_BYTES.saturating_add(1)],
            Scores::Requested
        ),
        Err(TranscriptError::TooLarge)
    );
    assert_eq!(
        decode(
            br#"{"text":"secret","logprobs":[{"token":"x","logprob":-1e308},{"token":"x","logprob":-1e308}]}"#,
            Scores::Requested
        ),
        Err(TranscriptError::Invalid)
    );
    let body = serde_json::to_vec(&serde_json::json!({"text":"x".repeat(vessel_proto::voice::MAX_SPEECH_BYTES.saturating_add(1))})).unwrap();
    assert_eq!(
        decode(&body, Scores::Unavailable),
        Err(TranscriptError::Text)
    );
}

#[test]
fn typed_response_ignores_unknown_metadata_and_rejects_duplicate_response_fields() {
    let body = br#"{"text":"hello\nworld","future":{"private":[true,17]},"logprobs":[{"token":"x","logprob":-1,"bytes":[120],"future":[{}]}]}"#;
    let VoiceTranscript::Accepted { text, confidence } = decode(body, Scores::Requested).unwrap()
    else {
        panic!("accepted known fields")
    };
    assert_eq!(text.as_str(), "hello\nworld");
    assert_eq!(confidence, Confidence::Accepted);
    for body in [
        br#"{"text":"private one","text":"private two"}"#.as_slice(),
        br#"{"text":"private","logprobs":[],"logprobs":null}"#,
    ] {
        let error = decode(body, Scores::Requested).unwrap_err();
        assert_eq!(error, TranscriptError::Invalid);
        assert!(!format!("{error:?} {error}").contains("private"));
    }
}
