//! Inbox behaviour through real nodes and fake agents, with mixed-scope devices.
use crate::agents::code;
use crate::secure::{device_via, dialer, direct_route};
use crate::support::{
    Fixture, agent_of, ask, client, prompt, read_until, step, structured_spec, within,
};
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_client::{Connector as _, NodeClient};
use vessel_proto::inbox::{InboxItem, InboxRead, ListInbox};
use vessel_proto::{
    AgentEvent, ErrorCode, EventSeq, InviteDevice, PermissionOptionId, PermissionOutcome,
    QuestionAnswer, Scope, SessionInput,
};

async fn paired(
    owner: &NodeClient,
    node: &vessel_node::Node,
    scopes: Vec<Scope>,
) -> (vessel_proto::DeviceId, NodeClient) {
    let invite = owner
        .call(InviteDevice {
            label: None,
            ttl_secs: 60,
            scopes: Some(scopes),
        })
        .await
        .unwrap();
    let mut offer = invite.link.decode().unwrap();
    offer.direct = vec![direct_route(node)];
    let (paired, _) = pair_over(&dialer(), &offer, "inbox reader", RouteTimeouts::default())
        .await
        .unwrap();
    let id = paired.credential.device;
    let connector = device_via(dialer(), &paired, &[&direct_route(node)], &[]);
    (id, connector.connect(client()).await.unwrap())
}

async fn durable(client: &NodeClient, session: vessel_proto::SessionId) {
    // Stream publication precedes SQLite commit. session.read already waits
    // for produced records, so use that boundary rather than a timing sleep.
    let _page = step(
        "inbox source committed",
        client.read(session, EventSeq::ZERO, 1),
    )
    .await
    .unwrap();
}
async fn permission_round_trip(
    owner: &NodeClient,
    reader: &NodeClient,
    writer: &NodeClient,
    session: vessel_proto::SessionId,
    stream: &mut vessel_client::SessionStream,
    seen: &mut Vec<vessel_proto::EventRecord>,
) {
    let page = reader.inbox(ListInbox::default()).await.unwrap();
    let [entry] = page.entries.as_slice() else {
        panic!("one approval should wait");
    };
    assert_eq!(entry.key.session, session);
    assert_eq!(entry.read, InboxRead::Unread);
    let InboxItem::Permission {
        request,
        tool: Some(tool),
        ..
    } = &entry.item
    else {
        panic!("approval must retain the tool");
    };
    assert_eq!(tool.title, "Run fake tool");
    assert_eq!(request.tool_call.as_ref(), Some(&tool.id));
    let input = SessionInput::AnswerPermission {
        id: request.id,
        outcome: PermissionOutcome::Selected {
            option: PermissionOptionId::new("allow-once"),
        },
    };
    assert_eq!(
        code(reader.send(session, input.clone()).await),
        ErrorCode::Forbidden
    );
    reader.mark_inbox(entry.key, InboxRead::Read).await.unwrap();
    assert_eq!(
        writer.inbox(ListInbox::default()).await.unwrap().entries[0].read,
        InboxRead::Read
    );
    assert_eq!(
        owner.conversation(session).await.unwrap().permissions.len(),
        1,
        "reading is not an answer"
    );
    let (first, second) = tokio::join!(
        writer.send(session, input.clone()),
        owner.send(session, input)
    );
    assert_ne!(first.is_ok(), second.is_ok(), "exactly one answer wins");
    let refused = first.err().or_else(|| second.err()).unwrap();
    assert_eq!(refused.code(), Some(ErrorCode::Conflict));
    read_until(stream, seen, |record| {
        matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
    })
    .await;
    durable(owner, session).await;
    assert_eq!(seen.iter().filter(|record| matches!(
        agent_of(record), Some(AgentEvent::PermissionResolved { id, .. }) if *id == request.id
    )).count(), 1);
    for device in [reader, writer] {
        assert!(
            device
                .inbox(ListInbox::default())
                .await
                .unwrap()
                .entries
                .iter()
                .all(|entry| !entry.item.waiting())
        );
    }
    assert_eq!(
        code(reader.mark_inbox(entry.key, InboxRead::Unread).await),
        ErrorCode::NotFound
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_nodes_two_devices_share_read_state_and_only_one_answer_wins() {
    within(async {
        let first = Fixture::new();
        let second = Fixture::new();
        let node = first
            .start_with(crate::secure::advertise_placeholder_route)
            .await;
        let other_node = second.start().await;
        let owner = first.connect(&node).await;
        let other = second.connect(&other_node).await;
        let (_reader_id, reader) = paired(&owner, &node, vec![Scope::Read]).await;
        let (_writer_id, writer) = paired(&owner, &node, vec![Scope::Read, Scope::Operate]).await;
        let session = owner
            .create_session(structured_spec(&first, None))
            .await
            .unwrap()
            .id;
        let other_session = other
            .create_session(structured_spec(&second, None))
            .await
            .unwrap()
            .id;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        let mut other_stream = other.follow(other_session, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        let mut other_seen = Vec::new();
        owner.send(session, prompt("tool")).await.unwrap();
        other.send(other_session, prompt("ask one")).await.unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::PermissionRequested(_)))
        })
        .await;
        read_until(&mut other_stream, &mut other_seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::QuestionAsked(_)))
        })
        .await;
        durable(&owner, session).await;
        permission_round_trip(&owner, &reader, &writer, session, &mut stream, &mut seen).await;
        durable(&other, other_session).await;
        let questions = other.inbox(ListInbox::default()).await.unwrap();
        let InboxItem::Question { question } = &questions.entries[0].item else {
            panic!("a question should wait on the other machine");
        };
        other
            .answer_question(other_session, question.id, QuestionAnswer::Decline)
            .await
            .unwrap();
        read_until(&mut other_stream, &mut other_seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        durable(&other, other_session).await;
        assert!(
            other
                .inbox(ListInbox::default())
                .await
                .unwrap()
                .entries
                .iter()
                .all(|entry| !entry.item.waiting())
        );
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, session, "plain").await,
            "echo: plain"
        );
        durable(&owner, session).await;
        let finished = owner
            .inbox(ListInbox::default())
            .await
            .unwrap()
            .entries
            .remove(0);
        assert_eq!(finished.item, InboxItem::Finished);
        owner
            .mark_inbox(finished.key, InboxRead::Read)
            .await
            .unwrap();
        drop((reader, writer));
        node.stop().await.unwrap();
        other_node.stop().await.unwrap();
        let node = first.start().await;
        let owner = first.connect(&node).await;
        let kept = owner.inbox(ListInbox::default()).await.unwrap();
        assert_eq!(kept.entries.len(), 1);
        assert_eq!(kept.entries[0].key, finished.key);
        assert_eq!(kept.entries[0].read, InboxRead::Read);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_turns_are_durable_and_agents_cannot_mark_personal_attention() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(crate::agents::vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = crate::agents::structured(&owner, &fixture).await;
        let (_, socket, auth) = crate::agents::credential(&owner, &mut parent).await;
        let agent = crate::agents::as_agent(&socket, &auth).await;
        let mut stream = owner.follow(parent.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        owner.send(parent.id, prompt("fail")).await.unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnFailed { .. }))
        })
        .await;
        durable(&owner, parent.id).await;
        let entry = owner
            .inbox(ListInbox::default())
            .await
            .unwrap()
            .entries
            .remove(0);
        assert_eq!(entry.item, InboxItem::Failed);
        assert_eq!(
            code(agent.mark_inbox(entry.key, InboxRead::Read).await),
            ErrorCode::Forbidden
        );
        owner.mark_inbox(entry.key, InboxRead::Read).await.unwrap();
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let page = owner.inbox(ListInbox::default()).await.unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].key, entry.key);
        assert_eq!(page.entries[0].read, InboxRead::Read);
        node.stop().await.unwrap();
    })
    .await;
}

mod pruning;
