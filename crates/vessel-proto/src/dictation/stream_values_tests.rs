//! Nested values keep validation, forward decoding and diagnostic privacy.

use super::*;

#[test]
fn acknowledgements_use_absence_instead_of_a_negative_sequence() {
    let empty = DictationAck { through: None };
    assert_eq!(serde_json::to_string(&empty).unwrap(), "{}");
    assert_eq!(serde_json::from_str::<DictationAck>("{}").unwrap(), empty);
    let ack = DictationAck {
        through: Some(ChunkSeq::new(0)),
    };
    assert_eq!(serde_json::to_string(&ack).unwrap(), r#"{"through":0}"#);
    assert!(serde_json::from_str::<DictationAck>(r#"{"through":-1}"#).is_err());
}

#[test]
fn future_updates_and_failure_reasons_remain_decodable() {
    let update: DictationUpdate =
        serde_json::from_str(r#"{"kind":"future","new_field":{}}"#).unwrap();
    assert_eq!(update, DictationUpdate::Unknown);
    let update: DictationUpdate =
        serde_json::from_str(r#"{"kind":"failed","reason":{"kind":"future","new_field":{}}}"#)
            .unwrap();
    assert_eq!(
        update,
        DictationUpdate::Failed {
            reason: DictationReason::Unknown
        }
    );
}

#[test]
fn a_parent_debug_cannot_expose_audio_or_transcript() {
    let transcript = "synthetic-private-transcript";
    for update in [
        DictationUpdate::Partial {
            text: Transcript::try_from(transcript.to_owned()).unwrap(),
        },
        DictationUpdate::Final {
            text: Transcript::try_from(transcript.to_owned()).unwrap(),
        },
    ] {
        assert!(!format!("{update:?}").contains(transcript));
        let wire = serde_json::to_string(&update).unwrap();
        assert_eq!(
            serde_json::from_str::<DictationUpdate>(&wire).unwrap(),
            update
        );
    }
    let request = ChunkDictation {
        id: "synthetic".parse().unwrap(),
        seq: ChunkSeq::new(0),
        pcm: DictationPcm::new(vec![42; super::super::PCM_SAMPLE_BYTES]).unwrap(),
    };
    let diagnostic = format!("{request:?}");
    assert!(diagnostic.contains("DictationPcm([redacted])"));
    let wire = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<ChunkDictation>(&wire).unwrap(),
        request
    );
    assert!(
        serde_json::from_str::<ChunkDictation>(r#"{"id":"synthetic","seq":0,"pcm":"AQ=="}"#)
            .is_err()
    );
}

#[test]
fn a_local_refusal_carries_its_model_in_the_reason_instead_of_an_optional_sibling() {
    let reason = DictationReason::LocalNotBuilt {
        model: LocalModelId::default_model(),
    };
    let encoded = serde_json::to_string(&reason).unwrap();
    let decoded: DictationReason = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, reason);
    assert!(serde_json::from_str::<DictationReason>(r#"{"kind":"local_not_built"}"#).is_err());
    let unknown: DictationReason = serde_json::from_str(r#"{"kind":"later","model":{}}"#).unwrap();
    assert_eq!(unknown, DictationReason::Unknown);
}
