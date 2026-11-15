//! The pending feature must remain unsendable through every envelope.
use crate::{Call, Method, MethodShape, ProtocolVersion, Scope, rpc::samples};

#[test]
fn checkpoint_rows_are_unassigned_unary_and_never_rewind_fallbacks() {
    for (method, scope) in [
        (Method::ListCheckpoints, Scope::Read),
        (Method::PreviewCheckpoint, Scope::Operate),
        (Method::ReadCheckpointChanges, Scope::Operate),
        (Method::RestoreCheckpoint, Scope::Operate),
        (Method::CheckpointRestoreStatus, Scope::Operate),
    ] {
        assert_eq!(method.shape(), MethodShape::Unary);
        assert_eq!(method.scope(), Some(scope));
        assert_eq!(method.since(), None);
        let call = samples::call(method).unwrap();
        assert_eq!(call.since(), None);
        assert_eq!(call.required_protocol(), None);
        let encoded = serde_json::to_string(&call).unwrap();
        assert!(!encoded.contains("session.rewind"));
        assert_eq!(serde_json::from_str::<Call>(&encoded).unwrap(), call);
    }
    assert_eq!(
        Method::RewindSession.since(),
        Some(ProtocolVersion::new(14))
    );
}

#[test]
fn generation_and_pagination_offsets_validate_on_decode() {
    assert!(serde_json::from_str::<super::CaptureGeneration>("0").is_err());
    assert!(serde_json::from_str::<super::CaptureGeneration>("1").is_ok());
    let invalid = super::CHECKPOINT_PATHS.saturating_add(1).to_string();
    assert!(serde_json::from_str::<super::ChangeCursor>(&invalid).is_err());
    let detail = r#"{"kind":"checkpoint","reason":"future_checkpoint_reason"}"#;
    assert_eq!(
        serde_json::from_str::<crate::ErrorDetail>(detail).unwrap(),
        crate::ErrorDetail::Checkpoint {
            reason: super::CheckpointError::Unknown,
        }
    );
}
