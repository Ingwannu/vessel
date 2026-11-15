//! External argument shapes, pending admission and bounded schema share validators.

use super::*;
use vessel_client::CallError;
use vessel_proto::{HarnessKind, Method, SessionListing, SessionMode, WireError};

#[test]
fn implicit_discovery_can_use_legacy_but_explicit_selection_never_disappears() {
    let peer = vessel_proto::PROTOCOL_VERSION;
    let implicit: Selection = decode(&JsonObject::new()).unwrap();
    let (request, fallback) = implicit.request(None, peer);
    assert!(request.selection().is_none());
    assert_eq!(fallback, LegacyFallback::Forbidden);
    let implicit: Selection = decode(&JsonObject::new()).unwrap();
    let (request, fallback) = implicit.request(Some(peer), peer);
    assert_eq!(request.selection(), Some(&SessionSelection::shown()));
    assert_eq!(fallback, LegacyFallback::Allowed);
    let implicit: Selection = decode(&JsonObject::new()).unwrap();
    let (request, fallback) = implicit.request(Some(peer), vessel_proto::MIN_PROTOCOL_VERSION);
    assert!(request.selection().is_none());
    assert_eq!(fallback, LegacyFallback::Forbidden);
    for args in [
        json!({"scope":"all"}),
        json!({"labels":{}}),
        json!({"labels":{"env":"prod"}}),
    ] {
        let selected: Selection = decode(args.as_object().unwrap()).unwrap();
        let (request, fallback) = selected.request(None, peer);
        assert!(request.selection().is_some());
        assert_eq!(fallback, LegacyFallback::Forbidden);
    }
    let unsupported = ToolError::Node(CallError::Remote(WireError::new(
        ErrorCode::Unsupported,
        "old hop",
    )));
    assert!(LegacyFallback::Allowed.after(&unsupported));
    assert!(!LegacyFallback::Forbidden.after(&unsupported));
    assert!(
        LegacyFallback::Allowed.after(&ToolError::Node(CallError::UnassignedProtocol {
            method: Method::ListSessions
        }))
    );
    for error in [
        ToolError::Arguments("invalid".to_owned()),
        ToolError::Node(CallError::Disconnected),
        ToolError::Node(CallError::Remote(WireError::new(
            ErrorCode::Forbidden,
            "scope",
        ))),
    ] {
        assert!(!LegacyFallback::Allowed.after(&error));
    }
}

#[test]
fn creation_keeps_omission_distinct_and_invalid_metadata_is_refused() {
    let mut spec = SessionSpec::new(
        HarnessKind::Codex,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    creation(&JsonObject::new(), &mut spec).unwrap();
    assert_eq!(spec.listing, None);
    let args = json!({"listing":"shown","labels":{"env":" prod=a,b "}});
    creation(args.as_object().unwrap(), &mut spec).unwrap();
    assert_eq!(spec.listing, Some(SessionListing::Shown));
    assert_eq!(spec.labels.summary(), "env= prod=a,b ");
    for invalid in [
        json!({"scope":"unknown"}),
        json!({"scope":null}),
        json!({"labels":null}),
        json!({"labels":{"":"x"}}),
    ] {
        assert!(decode::<Selection>(invalid.as_object().unwrap()).is_err());
    }
    for invalid in [
        json!({"listing":null}),
        json!({"listing":true}),
        json!({"labels":null}),
        json!({"labels":{"":"x"}}),
    ] {
        assert!(creation(invalid.as_object().unwrap(), &mut spec).is_err());
    }
    assert!(existing_target(&JsonObject::new()).is_ok());
    assert!(existing_target(args.as_object().unwrap()).is_err());
    let entries: JsonObject = (0..SessionLabels::MAX)
        .map(|index| (format!("key{index}"), Value::String("value".to_owned())))
        .collect();
    let mut args = json!({"labels":entries});
    assert!(creation(args.as_object().unwrap(), &mut spec).is_ok());
    args["labels"]["one_more"] = json!("value");
    assert!(creation(args.as_object().unwrap(), &mut spec).is_err());
}
