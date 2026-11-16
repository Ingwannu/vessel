//! Fictional wire inputs are decoded here; assertions use typed calls.

use super::*;
use crate::rpc::samples;
use crate::{Deliver, Forward, NodeId, RelayCall, Request as _};

#[test]
fn every_metadata_spec_and_envelope_is_unassigned_at_the_current_protocol() {
    assert_eq!(crate::lifecycle_version::SINCE, None);
    let mut calls: Vec<Call> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/labelled-requests.json"
    ))
    .unwrap();
    calls.extend(
        serde_json::from_str::<Vec<Call>>(include_str!(
            "../../../../tests/fixtures/listed-requests.json"
        ))
        .unwrap(),
    );
    for call in calls {
        assert_eq!(call.since(), None);
        let forwarded = Forward {
            node: NodeId::random(),
            call: Box::new(call.clone()),
        }
        .into_call();
        let relayed = RelayCall {
            node: NodeId::random(),
            call: Box::new(call.clone()),
            origin: samples::origin(),
        }
        .into_call();
        let delivered = Deliver {
            call: Box::new(call.clone()),
            origin: samples::origin(),
        }
        .into_call();
        let nested = Forward {
            node: NodeId::random(),
            call: Box::new(
                Deliver {
                    call: Box::new(relayed.clone()),
                    origin: samples::origin(),
                }
                .into_call(),
            ),
        }
        .into_call();
        for request in [call, forwarded, relayed, delivered, nested] {
            assert_eq!(request.since(), None, "{request:?}");
            assert_eq!(request.required_protocol(), None, "{request:?}");
        }
    }
}

#[test]
fn assignment_combines_with_existing_requirements_and_omitted_metadata_needs_none() {
    let labels: crate::SessionLabels = serde_json::from_str(r#"{"env":"prod"}"#).unwrap();
    let old = crate::MIN_PROTOCOL_VERSION;
    let new = crate::PROTOCOL_VERSION;
    assert!(old < new);
    let plain = SessionSpec::new(
        crate::HarnessKind::Codex,
        crate::SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    for spec in [
        plain.clone().with_labels(labels),
        plain.clone().with_listing(crate::SessionListing::Shown),
        plain
            .clone()
            .with_listing(crate::SessionListing::Background),
    ] {
        assert_eq!(required(old, &spec, None), None);
        assert_eq!(required(old, &spec, Some(new)), Some(new));
        assert_eq!(required(new, &spec, Some(old)), Some(new));
    }
    assert_eq!(required(new, &plain, None), Some(new));
}
