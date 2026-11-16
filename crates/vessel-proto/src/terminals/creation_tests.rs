//! New placement/launch metadata cannot inherit legacy transport support.
use super::*;
use crate::terminal_profiles::{Id, ProfileRevision};
use crate::{Call, Method, PROTOCOL_VERSION, ProtocolVersion};

const LEGACY_TERMINALS: ProtocolVersion = ProtocolVersion::new(11);

fn info(place: TerminalPlace, kind: TerminalKind) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        place,
        "/fixture".into(),
        kind,
        Utc::now(),
    )
}
fn profile() -> TerminalKind {
    TerminalKind::Profile {
        profile: Id::parse("configured").unwrap(),
        revision: ProfileRevision::INITIAL,
    }
}

#[test]
fn workspace_placement_preserves_only_its_explicit_identity() {
    let workspace = WorkspaceId::random();
    let place = TerminalPlace::Workspace { workspace };
    assert_eq!(place.workspace(), Some(workspace));
    assert_eq!(place.project(), None);
    assert_eq!(place.session(), None);
    let encoded = serde_json::to_value(&place).unwrap();
    assert_eq!(encoded["kind"], "workspace");
    assert_eq!(encoded["workspace"], workspace.to_string());
    assert_eq!(
        serde_json::from_value::<TerminalPlace>(encoded).unwrap(),
        place
    );
}

#[test]
fn workspace_call_fields_require_the_unassigned_creation_feature() {
    let place = TerminalPlace::Workspace {
        workspace: WorkspaceId::random(),
    };
    for call in [
        Call::CreateTerminal(CreateTerminal {
            place: place.clone(),
            title: None,
            size: None,
        }),
        Call::ListTerminals(ListTerminals { place: Some(place) }),
    ] {
        assert_eq!(call.required_protocol(), crate::creation::CREATION_PROTOCOL);
    }
    let legacy = Call::CreateTerminal(CreateTerminal {
        place: TerminalPlace::Session {
            session: SessionId::random(),
        },
        title: None,
        size: None,
    });
    assert_eq!(legacy.required_protocol(), Method::CreateTerminal.since());
    assert_eq!(legacy.required_protocol(), Some(LEGACY_TERMINALS));
}

#[test]
fn profile_metadata_carries_no_expanded_execution_fields() {
    let kind = profile();
    assert!(kind.takes_input());
    assert_eq!(kind.script(), None);
    let encoded = serde_json::to_value(&kind).unwrap();
    assert_eq!(encoded.as_object().unwrap().len(), 3);
    assert_eq!(encoded["kind"], "profile");
    assert_eq!(encoded["profile"], "configured");
    for absent in ["command", "program", "args", "prompt", "environment"] {
        assert!(encoded.get(absent).is_none());
    }
    assert_eq!(
        serde_json::from_value::<TerminalKind>(encoded).unwrap(),
        kind
    );
}

#[test]
fn both_new_metadata_shapes_are_capability_gated_independently() {
    let old_place = TerminalPlace::Session {
        session: SessionId::random(),
    };
    let shell = TerminalKind::Shell {
        program: "fixture-shell".into(),
    };
    let legacy = info(old_place.clone(), shell.clone());
    let workspace = info(
        TerminalPlace::Workspace {
            workspace: WorkspaceId::random(),
        },
        shell,
    );
    let profiled = info(old_place, profile());
    for protocol in [LEGACY_TERMINALS, PROTOCOL_VERSION] {
        assert!(legacy.readable_by(protocol));
        let expected =
            crate::creation::CREATION_PROTOCOL.is_some_and(|required| protocol >= required);
        assert_eq!(workspace.readable_by(protocol), expected);
        assert_eq!(profiled.readable_by(protocol), expected);
    }
}
