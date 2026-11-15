//! Actual authenticated fleet hops must not re-project scoped display identities.
use crate::across::{home, member_granting, member_joined};
use crate::fleet::fleet_config;
use crate::secure::{Relay, relayed_with};
use crate::support::{Fixture, structured_spec, within};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use vessel_proto::{
    AgentEvent, EventPage, EventRecord, EventSeq, HarnessKind, PermissionId, ReadEvents, Scope,
    SessionEvent, SessionId, SessionInfo, SessionStatus, StopReason,
};
use vessel_store::{Store, StoreConfig};

fn tool(title: &str, started: Option<u64>, id: &str) -> AgentEvent {
    serde_json::from_value(json!({"event":"tool_call","id":id,"title":title,"kind":"execute","status":"pending","scope":started.map(|started|json!({"kind":"session_turn","started":started}))})).unwrap()
}
async fn seed(fixture: &Fixture) -> (SessionId, Vec<Arc<EventRecord>>) {
    let store = Store::open(&fixture.data_dir(), StoreConfig::default())
        .await
        .unwrap()
        .store;
    let id = SessionId::random();
    store
        .create_session(SessionInfo::new(
            id,
            structured_spec(fixture, None),
            chrono::DateTime::UNIX_EPOCH,
        ))
        .await
        .unwrap();
    let late = serde_json::from_value(json!({"event":"tool_call_update","id":"x","title":"late A","scope":{"kind":"session_turn","started":0}})).unwrap();
    let permission = serde_json::from_value(json!({"event":"permission_requested","id":PermissionId::random(),"tool_call":"x","tool_scope":{"kind":"session_turn","started":0},"options":[]})).unwrap();
    let agent_events = [
        AgentEvent::TurnStarted {
            provenance: None,
            content: vec![],
            client_message_id: None,
        },
        tool("A", Some(0), "x"),
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
        AgentEvent::TurnStarted {
            provenance: None,
            content: vec![],
            client_message_id: None,
        },
        tool("B", Some(3), "x"),
        tool(
            "native reserved spelling",
            None,
            "vessel-timeline:v1:s:0:eA",
        ),
        late,
        permission,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    ];
    let mut records: Vec<_> = agent_events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            Arc::new(EventRecord {
                seq: EventSeq::new(index.try_into().unwrap()),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent { event },
            })
        })
        .collect();
    records.push(Arc::new(EventRecord {
        seq: EventSeq::new(9),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(0) },
        },
    }));
    store.append(id, records.clone()).await.unwrap();
    store.close().await;
    (id, records)
}
fn isolated(page: &EventPage) {
    let mut tools = HashMap::new();
    let mut referenced = None;
    for record in &page.records {
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
            referenced = request.tool_call.clone();
        }
    }
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[referenced.as_ref().unwrap()], "late A");
    assert!(tools.values().any(|title| title == "B"));
    assert!(
        tools
            .values()
            .any(|title| title == "native reserved spelling")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn direct_self_and_two_fleet_hops_share_one_stable_legacy_projection() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = home(&relay, sources.path()).await;
        let caller = member_granting(
            &relay,
            &home,
            "timeline-caller",
            vec![Scope::Sync, Scope::Relay],
            vec![Scope::Read, Scope::Operate],
        )
        .await;
        let fixture = Fixture::new();
        let (session, original) = seed(&fixture).await;
        let applied = fixture.cwd().join("timeline-applied");
        let node = relayed_with(&fixture, &[&relay.url], |config| {
            config.name = Some("timeline-target".into());
            config.fleet = fleet_config(HarnessKind::Codex, applied);
        })
        .await;
        let target = member_joined(
            &home,
            fixture,
            node,
            "timeline-target",
            vec![Scope::Sync],
            vec![Scope::Read, Scope::Operate],
        )
        .await;
        let request = ReadEvents {
            session,
            from: EventSeq::ZERO,
            limit: 100,
            wait: None,
        };
        let direct = target.owner.call(request).await.unwrap();
        let self_forward = target.owner.call_on(target.id(), request).await.unwrap();
        let via_home = home.owner.call_on(target.id(), request).await.unwrap();
        let via_member = caller.owner.call_on(target.id(), request).await.unwrap();
        assert_eq!(direct, self_forward);
        assert_eq!(direct, via_home);
        assert_eq!(direct, via_member);
        assert_eq!(direct.next, EventSeq::new(10));
        for (original, projected) in original.iter().zip(&direct.records) {
            assert_eq!((original.seq, original.at), (projected.seq, projected.at));
        }
        isolated(&direct);
        caller.node.stop().await.unwrap();
        target.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
