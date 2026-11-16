use super::*;
use base64::Engine as _;

#[test]
fn one_pending_feature_cannot_be_sent_or_forwarded() {
    use crate::{Method, MethodShape, Scope};
    for method in [
        Method::StartVoice,
        Method::CaptureVoice,
        Method::NextVoice,
        Method::PlayedVoice,
        Method::AbortVoice,
        Method::StopVoice,
        Method::SpeakVoice,
    ] {
        assert_eq!(method.since(), SINCE);
        assert_eq!(method.scope(), Some(Scope::Operate));
        assert_eq!(method.shape(), MethodShape::Unary);
        assert!(!method.forwardable());
    }
    assert_eq!(SINCE, None);
}

#[test]
fn pcm_caps_apply_to_constructors_and_untrusted_base64() {
    assert!(CapturePcm::new(Vec::new()).is_err());
    assert!(CapturePcm::new(vec![0]).is_err());
    let exact = CapturePcm::new(vec![1; MAX_CAPTURE_BYTES]).unwrap();
    let encoded = serde_json::to_string(&exact).unwrap();
    assert_eq!(serde_json::from_str::<CapturePcm>(&encoded).unwrap(), exact);
    assert!(CapturePcm::new(vec![0; MAX_CAPTURE_BYTES + 2]).is_err());
    let too_large =
        base64::engine::general_purpose::STANDARD.encode(vec![0; MAX_CAPTURE_BYTES + 2]);
    let json = serde_json::to_string(&too_large).unwrap();
    assert!(serde_json::from_str::<CapturePcm>(&json).is_err());
    // An invalid oversized encoding is refused by size before any decode allocation.
    let oversized = serde_json::to_string(&"!".repeat(MAX_CAPTURE_BYTES * 2)).unwrap();
    assert!(
        serde_json::from_str::<CapturePcm>(&oversized)
            .unwrap_err()
            .to_string()
            .contains("byte limit")
    );
    assert!(PlaybackPcm::new(vec![0; MAX_PLAYBACK_BYTES + 2]).is_err());
}

#[test]
fn invalid_audio_encoding_diagnostics_do_not_report_submitted_bytes() {
    for encoded in [r#""AAAA~AAA""#, r#""AAAA====""#, r#""AAA""#] {
        let errors = [
            serde_json::from_str::<CapturePcm>(encoded).unwrap_err(),
            serde_json::from_str::<PlaybackPcm>(encoded).unwrap_err(),
        ];
        for error in errors {
            assert!(
                error
                    .to_string()
                    .starts_with("base64 audio payload is invalid")
            );
            let debug = format!("{error:?}");
            assert!(debug.contains("base64 audio payload is invalid"));
            assert!(!debug.contains("Invalid byte"));
            assert!(!debug.contains("AAAA"));
        }
    }
}

#[test]
fn speech_is_nonblank_bounded_and_diagnostic_safe() {
    assert!(
        SpeechText::try_from(
            " 
	"
            .to_owned()
        )
        .is_err()
    );
    assert!(SpeechText::try_from("\u{feff}".to_owned()).is_err());
    assert!(SpeechText::try_from("\u{0085}".to_owned()).is_ok());
    assert!(SpeechText::try_from("a".repeat(MAX_SPEECH_BYTES)).is_ok());
    assert!(SpeechText::try_from("a".repeat(MAX_SPEECH_BYTES + 1)).is_err());
    let text = SpeechText::try_from("private spoken text".to_owned()).unwrap();
    let request = SpeakVoice { text };
    assert!(!format!("{request:?}").contains("private"));
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<SpeakVoice>(&encoded).unwrap(),
        request
    );
    let capture = CaptureVoice {
        lease: lease(),
        seq: VoiceSeq::ZERO,
        pcm: CapturePcm::new(vec![97; 100]).unwrap(),
    };
    assert!(!format!("{capture:?}").contains("97, 97"));
}

fn lease() -> VoiceLease {
    VoiceLease {
        id: VoiceId::random(),
        epoch: VoiceEpoch::random(),
    }
}

#[test]
fn low_confidence_has_no_text_field_and_future_reason_is_readable() {
    let update = VoiceUpdate::Transcription {
        result: VoiceTranscript::Rejected,
    };
    assert_eq!(
        serde_json::to_value(update).unwrap(),
        serde_json::json!({
            "kind":"transcription", "result":{"kind":"rejected"}
        })
    );
    assert_eq!(
        serde_json::from_str::<VoiceFailure>(r#""future_reason""#).unwrap(),
        VoiceFailure::Unknown
    );
}

#[test]
fn epochs_and_receipts_roundtrip_as_distinct_identifiers() {
    let first = lease();
    let second = VoiceLease {
        id: first.id,
        epoch: VoiceEpoch::random(),
    };
    assert_ne!(first, second);
    let request = PlayedVoice {
        lease: first,
        playback: PlaybackId::random(),
        outcome: PlaybackOutcome::Interrupted,
    };
    let encoded = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        serde_json::from_slice::<PlayedVoice>(&encoded).unwrap(),
        request
    );
}

#[test]
fn pending_stop_roundtrips_without_fabricating_a_server_epoch() {
    let id = VoiceId::random();
    let pending = StopVoice {
        target: VoiceStopTarget::Starting { id },
    };
    let json = serde_json::to_string(&pending).unwrap();
    assert!(!json.contains("epoch"));
    assert_eq!(serde_json::from_str::<StopVoice>(&json).unwrap(), pending);
    let active = StopVoice {
        target: VoiceStopTarget::Active { lease: lease() },
    };
    let json = serde_json::to_string(&active).unwrap();
    assert!(json.contains("epoch"));
    assert_eq!(serde_json::from_str::<StopVoice>(&json).unwrap(), active);
    let mixed = format!(
        r#"{{"target":{{"kind":"starting","id":"{id}","epoch":"{}"}}}}"#,
        VoiceEpoch::random()
    );
    assert!(serde_json::from_str::<StopVoice>(&mixed).is_err());
}
