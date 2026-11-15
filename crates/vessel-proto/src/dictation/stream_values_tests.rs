//! Nested values keep validation, forward decoding and diagnostic privacy.

use super::*;
use crate::dictation::SampleRate;

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

#[test]
fn registered_dictation_calls_share_pending_admission_operate_and_node_local_policy() {
    use crate::{Method, MethodShape, Request as _, Scope};
    let id: DictationId = "admission".parse().unwrap();
    let calls = [
        StartDictation {
            id: id.clone(),
            format: Pcm16Mono::new(SampleRate::MIN),
        }
        .into_call(),
        ChunkDictation {
            id: id.clone(),
            seq: ChunkSeq::new(0),
            pcm: DictationPcm::new(vec![0; super::super::PCM_SAMPLE_BYTES]).unwrap(),
        }
        .into_call(),
        FinishDictation {
            id: id.clone(),
            final_seq: ChunkSeq::new(0),
        }
        .into_call(),
        NextDictation { id: id.clone() }.into_call(),
        CancelDictation { id }.into_call(),
    ];
    let expected = [
        Method::StartDictation,
        Method::ChunkDictation,
        Method::FinishDictation,
        Method::NextDictation,
        Method::CancelDictation,
    ];
    for (call, method) in calls.into_iter().zip(expected) {
        assert_eq!(call.method(), method);
        assert_eq!(call.required_protocol(), super::super::SINCE);
        assert_eq!(method.since(), super::super::SINCE);
        assert_eq!(method.scope(), Some(Scope::Operate));
        assert_eq!(method.shape(), MethodShape::Unary);
        assert!(!method.forwardable());
    }
}

#[test]
fn minted_stream_ids_are_independent_and_obey_the_same_wire_validator() {
    let first = DictationId::random();
    let second = DictationId::random();
    assert_ne!(first, second);
    assert_eq!(first.as_str().parse::<DictationId>().unwrap(), first);
    assert_eq!(second.as_str().parse::<DictationId>().unwrap(), second);
}
