//! Pending field admission and projection cover direct and every nested reply.
use super::*;
use crate::dictation::{DictationChange, DictationSettings, DictationSwitch, SINCE};
use crate::forward::Forwarded;
use crate::{Call, ProtocolVersion, Reply, Request as _};

#[test]
fn a_present_block_needs_assignment_even_when_empty_and_null_cannot_hide_it() {
    assert!(SINCE.is_none());
    let absent: SetNodeSettings = serde_json::from_str("{}").unwrap();
    assert!(absent.into_call().required_protocol().is_some());
    for json in [r#"{"dictation":{}}"#, r#"{"dictation":{"key":null}}"#] {
        let patch: SetNodeSettings = serde_json::from_str(json).unwrap();
        assert!(patch.dictation.is_some());
        assert_eq!(patch.into_call().required_protocol(), None);
    }
    assert!(serde_json::from_str::<SetNodeSettings>(r#"{"dictation":null}"#).is_err());
    assert!(serde_json::from_str::<SetNodeSettings>(r#"{"dictation":{"later":true}}"#).is_err());
    let reset: SetNodeSettings = serde_json::from_str(r#"{"dictation":{"key":null}}"#).unwrap();
    assert_eq!(reset.dictation.unwrap().key, Some(DictationChange::Reset));
}
fn settings() -> NodeSettings {
    NodeSettings {
        park_after: ParkAfter::DEFAULT,
        drivers: AgentDrivers::default(),
        restart_recovery: RestartRecovery::default(),
        dictation: Some(DictationSettings::default()),
    }
}
fn envelopes(reply: Reply) -> [Reply; 3] {
    [
        Reply::Forward(Forwarded {
            reply: Box::new(reply.clone()),
        }),
        Reply::Relay(Forwarded {
            reply: Box::new(reply.clone()),
        }),
        Reply::Deliver(Forwarded {
            reply: Box::new(reply),
        }),
    ]
}

#[test]
fn default_dictation_keeps_legacy_bytes_and_nondefault_values_remain_visible() {
    let original = settings();
    let mut legacy = original.clone();
    legacy.dictation = None;
    for (reply, expected) in [
        (
            Reply::GetNodeSettings(original.clone()),
            Reply::GetNodeSettings(legacy.clone()),
        ),
        (
            Reply::SetNodeSettings(original.clone()),
            Reply::SetNodeSettings(legacy.clone()),
        ),
    ] {
        assert_eq!(
            serde_json::to_vec(&reply).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        for (nested, expected) in envelopes(reply).into_iter().zip(envelopes(expected)) {
            assert_eq!(
                serde_json::to_vec(&nested).unwrap(),
                serde_json::to_vec(&expected).unwrap()
            );
        }
    }
    let mut changed = original;
    changed.dictation.as_mut().unwrap().switch = DictationSwitch::Off;
    assert!(
        serde_json::to_value(changed)
            .unwrap()
            .get("dictation")
            .is_some()
    );
}

#[test]
fn pending_dictation_is_hidden_on_get_legacy_set_and_every_nested_envelope() {
    let original = settings();
    let mut expected = original.clone();
    expected.dictation = None;
    for number in crate::MIN_PROTOCOL_VERSION.get()..=crate::PROTOCOL_VERSION.get() {
        let protocol = ProtocolVersion::new(number);
        for (reply, projected) in [
            (
                Reply::GetNodeSettings(original.clone()),
                Reply::GetNodeSettings(expected.clone()),
            ),
            (
                Reply::SetNodeSettings(original.clone()),
                Reply::SetNodeSettings(expected.clone()),
            ),
        ] {
            assert_eq!(reply.for_protocol(protocol), Some(projected.clone()));
            for (nested, want) in envelopes(reply).into_iter().zip(envelopes(projected)) {
                assert_eq!(nested.for_protocol(protocol), Some(want.clone()));
                for (twice, twice_want) in envelopes(nested).into_iter().zip(envelopes(want)) {
                    assert_eq!(twice.for_protocol(protocol), Some(twice_want));
                }
            }
        }
    }
}

#[test]
fn synthetic_assignment_hides_older_replies_and_keeps_current_values() {
    let original = settings();
    let required = crate::PROTOCOL_VERSION;
    let older = ProtocolVersion::new(required.get().checked_sub(1).unwrap());
    assert!(
        original
            .for_dictation_protocol(older, Some(required))
            .unwrap()
            .dictation
            .is_none()
    );
    assert_eq!(
        original.for_dictation_protocol(required, Some(required)),
        None
    );
    assert!(
        original
            .for_dictation_protocol(required, None)
            .unwrap()
            .dictation
            .is_none()
    );
}

#[test]
fn an_envelope_does_not_turn_a_pending_settings_field_into_an_assigned_call() {
    let node = crate::NodeId::random();
    let origin = crate::Origin {
        node,
        by: crate::OriginBy::Owner,
        scopes: Vec::new(),
    };
    let inner = SetNodeSettings {
        dictation: Some(crate::dictation::SetDictation::default()),
        ..SetNodeSettings::default()
    }
    .into_call();
    let forward = crate::Forward {
        node,
        call: Box::new(inner.clone()),
    }
    .into_call();
    let relay = crate::RelayCall {
        node,
        origin: origin.clone(),
        call: Box::new(inner.clone()),
    }
    .into_call();
    let deliver = crate::Deliver {
        origin,
        call: Box::new(inner),
    }
    .into_call();
    for call in [forward, relay, deliver] {
        assert_eq!(call.required_protocol(), None);
        let nested: Call = crate::Forward {
            node,
            call: Box::new(call),
        }
        .into_call();
        assert_eq!(nested.required_protocol(), None);
    }
}
