//! Ordinary commands and actual framed responses reach the real guarded event.
use crate::{Command, Event, testing::actor::Actor};
use vessel_app_view::{
    devices::Tickets,
    history::{Ask, Batch, Entry, Problem, Reply, Scope},
};
use vessel_proto::{Call, HarnessKind, HarnessSessionId, Outcome, SessionId, SessionInfo};
fn scope(actor: &Actor) -> Scope {
    let context = actor.context();
    Scope {
        machine: context.machine().clone(),
        context,
        harness: HarnessKind::Codex.into(),
        profile: None,
        cwd: None,
    }
}
#[tokio::test]
async fn active_actor_command_and_typed_response_install_without_fake_guard() {
    let mut actor = Actor::new("fixture").await;
    let scope = scope(&actor);
    let ticket = Tickets::default().next_ticket().unwrap();
    actor
        .dispatch(Command::History(Ask::Profiles { scope, ticket }))
        .await;
    let request = actor.request().await;
    assert!(matches!(request.call, Call::ListProfiles(_)));
    request
        .answer
        .send(Outcome::Ok(vessel_proto::Reply::ListProfiles(vec![])))
        .unwrap();
    let Event::History(answer) = actor.next_event().await else {
        panic!("history event")
    };
    assert!(matches!(
        answer.into_reply(),
        Reply::Profiles { result: Ok(_), .. }
    ));
}
#[tokio::test]
async fn queued_actual_history_page_cannot_install_after_connected_retirement() {
    let mut actor = Actor::new("fixture").await;
    let scope = scope(&actor);
    let ticket = Tickets::default().next_ticket().unwrap();
    actor
        .dispatch(Command::History(Ask::List {
            scope: scope.clone(),
            ticket,
            cursor: None,
        }))
        .await;
    let request = actor.request().await;
    assert!(matches!(request.call, Call::GetAgentHistory(_)));
    request
        .answer
        .send(Outcome::Ok(vessel_proto::Reply::GetAgentHistory(
            vessel_proto::history::AgentHistoryPage {
                conversations: vec![],
                next: None,
            },
        )))
        .unwrap();
    let Event::History(answer) = actor.next_event().await else {
        panic!("history event")
    };
    actor.retire().await;
    let Reply::Listed {
        scope: original,
        ticket: original_ticket,
        result,
    } = answer.into_reply()
    else {
        panic!("list")
    };
    assert_eq!(original, scope);
    assert_eq!(original_ticket, ticket);
    assert_eq!(result, Err(Problem::IdentityChanged));
}
#[tokio::test]
async fn queued_actual_create_receipt_becomes_uncertain_after_physical_peer_close() {
    let mut actor = Actor::new("fixture").await;
    let scope = scope(&actor);
    let ticket = Tickets::default().next_ticket().unwrap();
    let native = HarnessSessionId::new("native-history");
    let entry = Entry {
        id: native.clone(),
        cwd: "/work/original".into(),
        title: None,
        updated_at: None,
        in_vessel: None,
    };
    let batch = Batch {
        node: scope.context.node(),
        scope,
        ticket,
        entries: vec![entry],
    };
    actor.dispatch(Command::History(Ask::Import(batch))).await;
    let request = actor.request().await;
    let Call::CreateSession(create) = request.call else {
        panic!("existing create method")
    };
    let info = SessionInfo::new(
        SessionId::random(),
        create.spec,
        chrono::DateTime::UNIX_EPOCH,
    );
    request
        .answer
        .send(Outcome::Ok(vessel_proto::Reply::CreateSession(info)))
        .unwrap();
    let Event::History(answer) = actor.next_event().await else {
        panic!("history event")
    };
    actor.close_peer().await;
    let Reply::Imported { outcomes, .. } = answer.into_reply() else {
        panic!("import")
    };
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes.first().unwrap().id, native);
    assert_eq!(outcomes.first().unwrap().result, Err(Problem::Uncertain));
}
