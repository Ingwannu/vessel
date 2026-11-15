//! Synthetic assignment reaches future peer cases without assigning a wire number.
//! Fictional metadata is decoded only in these test fixtures.

use super::*;
use vessel_proto::{
    HarnessKind, HarnessSessionId, SessionId, SessionListing, SessionMode, SessionSpec, TransferId,
    TransferManifest,
};

/// Build fictional metadata without creating a real provider session.
pub(crate) fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}

/// Model an assignment threshold without publishing a feature number.
pub(super) fn versions() -> (ProtocolVersion, ProtocolVersion) {
    let new = vessel_proto::PROTOCOL_VERSION;
    let old = ProtocolVersion::new(new.get().saturating_sub(1));
    assert!(old < new);
    (old, new)
}

fn unsupported(result: Result<(), ServiceError>, expected: CallError) {
    let error = result.unwrap_err();
    assert_eq!(error.to_wire().code, vessel_proto::ErrorCode::Unsupported);
    let ServiceError::MetadataProtocol(actual) = error else {
        panic!("expected the shared typed protocol refusal");
    };
    assert_eq!(actual, expected);
}

#[test]
fn default_metadata_needs_no_export_assignment_or_peer_version() {
    let (old, new) = versions();
    for assigned in [None, Some(new)] {
        let mut plain = info();
        for listing in [None, Some(SessionListing::Shown)] {
            plain.spec.listing = listing;
            for peer in [old, new] {
                assert!(check_at(&plain, assigned, peer).is_ok());
            }
        }
    }
}

#[test]
fn current_labels_and_background_share_the_projection_visibility_rule() {
    let (old, new) = versions();
    let mut labelled = info();
    labelled.labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
    let mut background = info();
    background.listing = SessionListing::Background;
    for info in [labelled, background] {
        let before = info.clone();
        for peer in [old, new] {
            unsupported(
                check_at(&info, None, peer),
                CallError::UnassignedProtocol {
                    method: ExportSession::METHOD,
                },
            );
        }
        unsupported(
            check_at(&info, Some(new), old),
            CallError::NotInProtocol {
                method: ExportSession::METHOD,
                needs: new,
                negotiated: old,
            },
        );
        assert!(check_at(&info, Some(new), new).is_ok());
        assert!(
            check(&info, None).is_ok(),
            "a local pull has no wire projection"
        );
        assert_eq!(info, before);
    }
}

/// Supply a synthetic returned manifest for visibility and compensation tests.
pub(crate) fn exported(info: SessionInfo) -> Forwarded {
    Forwarded {
        reply: Box::new(Reply::ExportSession(TransferManifest {
            transfer: TransferId::random(),
            files: std::collections::BTreeMap::new(),
            detail: TransferDetail::Session {
                info: Box::new(info),
                harness_session: HarnessSessionId::new("fictional-export"),
                cwd: "/fixture".into(),
                options: Vec::new(),
            },
        })),
    }
}

#[test]
fn a_synthetic_three_hop_export_refuses_at_the_old_importer_before_projection() {
    // SINCE remains None in the product. The injected assignment models a
    // newer source and home, then an older importing node, without a fake number.
    let (old, new) = versions();
    let mut original = info();
    original.labels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
    original.listing = SessionListing::Background;
    assert!(check_at(&original, Some(new), new).is_ok());
    let source = exported(original.clone());
    let source_wire = serde_json::to_string(&source).unwrap();
    let home: Forwarded = serde_json::from_str(&source_wire).unwrap();
    assert!(forwarded_at(&home, Some(new), new).is_ok());
    let home = Forwarded {
        reply: Box::new(Reply::Relay(home)),
    };
    let home_wire = serde_json::to_string(&home).unwrap();
    let importing: Forwarded = serde_json::from_str(&home_wire).unwrap();
    unsupported(
        forwarded_at(&importing, Some(new), old),
        CallError::NotInProtocol {
            method: ExportSession::METHOD,
            needs: new,
            negotiated: old,
        },
    );
    let Reply::Relay(nested) = importing.reply.as_ref() else {
        panic!("expected relay");
    };
    let Reply::ExportSession(manifest) = nested.reply.as_ref() else {
        panic!("expected export");
    };
    let TransferDetail::Session { info, .. } = &manifest.detail else {
        panic!("expected session");
    };
    assert_eq!(
        info.as_ref(),
        &original,
        "refusal must precede destructive projection"
    );
}

#[test]
fn ordinary_metadata_replies_keep_the_existing_projection_instead_of_export_refusal() {
    let (old, new) = versions();
    let mut current = info();
    current.listing = SessionListing::Background;
    let value = Forwarded {
        reply: Box::new(Reply::GetSession(current)),
    };
    let before = value.clone();
    assert!(forwarded_at(&value, None, old).is_ok());
    assert!(forwarded_at(&value, Some(new), old).is_ok());
    assert_eq!(value, before);
}
