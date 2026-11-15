//! Real serialisation at each forwarding boundary, including hostile native ids.
use super::reference::ReferenceContext;
use super::{TimelineReferenceOrigin, ToolCallScope};
use crate::{
    AgentEvent, EventPage, EventRecord, EventSeq, Forwarded, Outcome, ProtocolVersion, Reply,
    RequestId, ServerFrame, SessionEvent, ToolCall, ToolCallId,
};
use serde_json::json;
use std::collections::HashMap;

const MODERN: ProtocolVersion = ProtocolVersion::new(30);
const OLD: ProtocolVersion = ProtocolVersion::new(29);

fn context(assigned: Option<ProtocolVersion>) -> ReferenceContext {
    ReferenceContext {
        origin: None,
        assigned,
    }
}

fn fixture() -> EventPage {
    let call: ToolCall = serde_json::from_value(json!({"id":"x","title":"A","kind":"execute","status":"in_progress","scope":{"kind":"session_turn","started":1}})).unwrap();
    let mut b = call.clone();
    b.title = "B".into();
    b.scope = Some(Box::new(ToolCallScope::SessionTurn {
        started: EventSeq::new(2),
    }));
    let mut native_collision = call.clone();
    native_collision.title = "native reserved spelling".into();
    native_collision.scope = None;
    native_collision.id = ToolCallId::new("vessel-timeline:v1:s:1:eA");
    let permission = serde_json::from_value(json!({"id":crate::PermissionId::random(),"tool_call":"x","tool_scope":call.scope,"options":[]})).unwrap();
    let update =
        serde_json::from_value(json!({"id":"x","title":"late A","scope":call.scope})).unwrap();
    let events = [
        AgentEvent::ToolCall(call),
        AgentEvent::ToolCall(b),
        AgentEvent::ToolCall(native_collision),
        AgentEvent::PermissionRequested(permission),
        AgentEvent::ToolCallUpdate(update),
    ];
    EventPage {
        records: events
            .into_iter()
            .enumerate()
            .map(|(index, event)| EventRecord {
                seq: EventSeq::new(index.try_into().unwrap()),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent { event },
            })
            .collect(),
        next: EventSeq::new(5),
        at_end: true,
    }
}

fn hop(
    forwarded: Forwarded,
    wrap: fn(Forwarded) -> Reply,
    peer: ProtocolVersion,
    gate: Option<ProtocolVersion>,
) -> Forwarded {
    let original = wrap(forwarded);
    let masked = original
        .for_protocol_with_reference(peer, context(gate))
        .unwrap_or(original);
    // Use the actual response envelope codec, as on the target/home/caller links.
    let frame = ServerFrame::Response {
        id: RequestId::new(7),
        outcome: Outcome::Ok(masked),
    };
    let bytes = serde_json::to_vec(&frame).unwrap();
    let ServerFrame::Response {
        outcome: Outcome::Ok(reply),
        ..
    } = serde_json::from_slice(&bytes).unwrap()
    else {
        panic!("response")
    };
    let (Reply::Forward(mut forwarded)
    | Reply::Relay(mut forwarded)
    | Reply::Deliver(mut forwarded)) = reply
    else {
        panic!("forwarded")
    };
    forwarded.received_with(peer, gate);
    forwarded
}

fn page(value: &Forwarded) -> &EventPage {
    let Reply::ReadEvents(page) = value.reply.as_ref() else {
        panic!("page")
    };
    page
}

fn assert_isolation(value: &EventPage) {
    let mut tools = HashMap::new();
    let mut permission_ref = None;
    for record in &value.records {
        if let SessionEvent::Agent {
            event: AgentEvent::ToolCall(call),
        } = &record.event
        {
            assert!(tools.insert(call.id.clone(), call.title.clone()).is_none());
        } else if let SessionEvent::Agent {
            event: AgentEvent::ToolCallUpdate(update),
        } = &record.event
        {
            tools
                .get_mut(&update.id)
                .unwrap()
                .clone_from(update.title.as_ref().unwrap());
        } else if let SessionEvent::Agent {
            event: AgentEvent::PermissionRequested(request),
        } = &record.event
        {
            permission_ref = Some(request.tool_call.clone());
        } else {
            panic!("fixture kind");
        }
    }
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[permission_ref.as_ref().unwrap()], "late A");
    assert!(tools.values().any(|value| value == "B"));
    assert!(
        tools
            .values()
            .any(|value| value == "native reserved spelling")
    );
}

#[test]
fn local_self_forward_first_projects_and_repeated_classified_hops_are_stable() {
    for gate in [None, Some(MODERN)] {
        let original = fixture();
        let direct = original
            .for_protocol_with_reference(OLD, context(gate))
            .unwrap();
        let local = Forwarded {
            reference_origin: None,
            reply: Box::new(Reply::ReadEvents(original.clone())),
        };
        let target = hop(local, Reply::Deliver, OLD, gate);
        assert_eq!(page(&target), &direct);
        assert_isolation(page(&target));
        let home = hop(target.clone(), Reply::Relay, OLD, gate);
        let caller = hop(home.clone(), Reply::Forward, OLD, gate);
        assert_eq!(target, home);
        assert_eq!(home, caller);
        assert_eq!(page(&caller).next, original.next);
        for (source, projected) in original.records.iter().zip(&page(&caller).records) {
            assert_eq!((source.seq, source.at), (projected.seq, projected.at));
            assert_eq!(
                source
                    .for_protocol_with_reference(OLD, context(gate))
                    .unwrap(),
                *projected
            );
        }
    }
}

#[test]
fn modern_connections_keep_originals_and_old_ingress_cannot_upgrade() {
    let local = Forwarded {
        reference_origin: None,
        reply: Box::new(Reply::ReadEvents(fixture())),
    };
    let modern = hop(local.clone(), Reply::Deliver, MODERN, Some(MODERN));
    assert_eq!(modern, local);
    let modern_again = hop(modern, Reply::Relay, MODERN, Some(MODERN));
    assert_eq!(modern_again, local);
    let legacy = hop(modern_again, Reply::Deliver, OLD, Some(MODERN));
    let restored = hop(legacy.clone(), Reply::Forward, MODERN, Some(MODERN));
    assert_eq!(restored, legacy);
    assert_eq!(
        restored.reference_origin,
        Some(TimelineReferenceOrigin::LegacyProjected)
    );
    assert_isolation(page(&restored));
}

#[test]
fn client_unknown_origin_is_inert_and_never_creates_scoped_authority() {
    let request = super::RespondPermission {
        session: crate::SessionId::random(),
        permission: crate::PermissionId::random(),
        response: super::PermissionResponse {
            outcome: crate::PermissionOutcome::Cancelled,
            modified_input: None,
            message: None,
            interrupt: super::PermissionInterrupt::Continue,
        },
    };
    let mut injected = serde_json::to_value(&request).unwrap();
    injected["reference_origin"] = json!("canonical");
    assert_eq!(
        serde_json::from_value::<super::RespondPermission>(injected).unwrap(),
        request
    );
    let legacy = Forwarded {
        reference_origin: Some(TimelineReferenceOrigin::LegacyProjected),
        reply: Box::new(Reply::ReadEvents(fixture())),
    };
    assert_eq!(
        legacy.check_scoped_references(),
        Err(crate::ErrorCode::Unsupported)
    );
    let local = Forwarded {
        reference_origin: None,
        ..legacy
    };
    // Unassigned activation cannot resolve a fleet tool reference as canonical.
    assert_eq!(
        local.check_scoped_references(),
        Err(crate::ErrorCode::Unsupported)
    );
}

#[test]
fn unknown_origin_survives_real_decode_as_display_only() {
    let legacy = hop(
        Forwarded {
            reference_origin: None,
            reply: Box::new(Reply::ReadEvents(fixture())),
        },
        Reply::Deliver,
        OLD,
        Some(MODERN),
    );
    let mut encoded = serde_json::to_value(&legacy).unwrap();
    encoded["reference_origin"] = json!("future_origin");
    let decoded: Forwarded = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, legacy);
    assert_eq!(hop(decoded, Reply::Relay, MODERN, Some(MODERN)), legacy);
}
