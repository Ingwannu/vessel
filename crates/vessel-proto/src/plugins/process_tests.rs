use super::*;
use crate::RawJson;

#[test]
fn hook_names_wait_bits_and_alias_refusal_match_reference() {
    let waited = [
        "workspace.create.before",
        "worktree.create.before",
        "worktree.create.after",
        "agent.create.before",
        "agent.open.before",
    ];
    assert_eq!(HookName::ALL.len(), 14);
    for hook in HookName::ALL {
        let encoded = serde_json::to_value(hook).expect("encode");
        let text = encoded.as_str().expect("name");
        assert_eq!(
            hook.wait(),
            if waited.contains(&text) {
                HookWait::Blocks
            } else {
                HookWait::Notifies
            },
            "{text}"
        );
        assert_eq!(
            serde_json::from_value::<HookName>(encoded).expect("decode"),
            hook
        );
    }
    assert!(serde_json::from_str::<HookName>(r#""agent.created""#).is_err());
    assert!(
        serde_json::from_str::<HookSubscriptions>(
            r#"["agent.create.before","agent.create.before"]"#
        )
        .is_err()
    );
}

#[test]
fn handshake_version_correlations_and_unit_objects_are_strict() {
    assert!(serde_json::from_str::<HelperProtocol>("2").is_err());
    assert!(serde_json::from_str::<InvocationId>("0").is_err());
    assert!(
        serde_json::from_str::<NodeToHelper>(r#"{"type":"shutdown","ignored":"bad"}"#).is_err()
    );
    assert!(serde_json::from_str::<NodeToHelper>(r#"{"type":"shutdown"}"#).is_ok());
    assert!(
        serde_json::from_str::<HelperToNode>(
            r#"{"type":"paseo_frame","data":"unrestricted broker"}"#
        )
        .is_err()
    );
}

#[test]
fn helper_payload_is_opaque_and_never_debug_printed() {
    let raw = RawJson::from_text(r#"{"secret":"PRIVATE-HELPER-CANARY"}"#).expect("json");
    let payload = HelperPayload::try_from(raw).expect("bounded");
    assert!(!format!("{payload:?}").contains("PRIVATE-HELPER-CANARY"));
    let encoded = serde_json::to_string(&payload).expect("encode");
    let decoded: HelperPayload = serde_json::from_str(&encoded).expect("decode");
    assert_eq!(decoded, payload);
    let oversized = serde_json::to_string(&"x".repeat(MAX_HELPER_PAYLOAD_BYTES)).expect("encode");
    let raw = RawJson::from_text(oversized.clone()).expect("json");
    assert!(HelperPayload::try_from(raw).is_err());
    assert!(serde_json::from_str::<HelperPayload>(&oversized).is_err());
}

#[test]
fn helper_diagnostics_cannot_escape_the_envelope_debug_path() {
    let message: HelperToNode = serde_json::from_str(
        r#"{"type":"error","request":1,"kind":"callback","diagnostic":"PRIVATE-HELPER-CANARY"}"#,
    )
    .expect("decode");
    assert!(!format!("{message:?}").contains("PRIVATE-HELPER-CANARY"));
    let HelperToNode::Error { diagnostic, .. } = message else {
        panic!("error");
    };
    assert_eq!(diagnostic.as_str(), "PRIVATE-HELPER-CANARY");
}
