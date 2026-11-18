//! Ordinary commands and actual framed responses reach the real guarded event.
use crate::{Command, Event, testing::actor::Actor};
use vessel_app_view::{
    devices::{Ticket, Tickets},
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
    crate::testing::within(async {
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

        actor.close_peer().await;
    })
    .await;
}
#[tokio::test]
async fn queued_actual_history_page_cannot_install_after_connected_retirement() {
    crate::testing::within(async {
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
                    already_imported: None,
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

        actor.close_peer().await;
    })
    .await;
}
#[tokio::test]
async fn queued_actual_create_receipt_becomes_uncertain_after_physical_peer_close() {
    crate::testing::within(async {
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

        actor.close_peer().await;
    })
    .await;
}

/// A sheet open on `scope`'s host with its one agent, and that read's ticket.
fn opened(scope: &Scope) -> (vessel_app_view::history::Sheet, Ticket) {
    let mut model = vessel_app_view::history::Sheet::default();
    let opening = vessel_app_view::history::Opening {
        context: scope.context.clone(),
        machine: scope.machine.clone(),
        cwd: None,
    };
    let asks = model.open(opening, vec![scope.harness.clone()]).unwrap();
    let Ok([Ask::List { ticket, .. }]) = <[Ask; 1]>::try_from(asks) else {
        panic!("list")
    };
    (model, ticket)
}

#[tokio::test(start_paused = true)]
async fn a_full_owner_queue_past_both_old_deadlines_still_settles_the_exact_import() {
    crate::testing::within(async {
        let peer = crate::history::tests::peer::connect(vec![]).await;
        let (_home, _state, selected, context, _events) =
            crate::history::tests::peer::admitted(&peer);
        let scope = Scope {
            machine: context.machine().clone(),
            context,
            harness: HarnessKind::Codex.into(),
            profile: None,
            cwd: None,
        };
        let (mut model, ticket) = opened(&scope);
        let entry = Entry {
            id: HarnessSessionId::new("native"),
            cwd: "/work".into(),
            title: None,
            updated_at: None,
            in_vessel: None,
        };
        assert!(
            model
                .answer(Reply::Listed {
                    scope: scope.clone(),
                    ticket,
                    result: Ok(vessel_app_view::history::Listed {
                        entries: vec![entry.clone()],
                        next: None,
                        node: scope.context.node()
                    })
                })
                .applied
        );
        let request = model.import(&scope.harness, &entry.id).unwrap().unwrap();
        let reply = crate::testing::phase(
            "actual create receipt",
            crate::history::answer(&selected, request.clone()),
        )
        .await;
        assert_eq!(peer.calls.lock().unwrap().len(), 1);
        let (events, mut incoming) = tokio::sync::mpsc::channel(1);
        events
            .try_send(Event::History(super::Answer::refused(
                Ask::Profiles {
                    scope: scope.clone(),
                    ticket,
                },
                Problem::Unsent,
            )))
            .unwrap();
        let guard = selected.delivery_guard();
        let publication = super::publish(&selected, &events, request, reply, guard);
        tokio::pin!(publication);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                publication.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        tokio::time::advance(crate::history::PUBLICATION_DEADLINE).await;
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                publication.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        tokio::time::advance(crate::history::PUBLICATION_DEADLINE).await;
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                publication.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        let _blocker = incoming.try_recv().unwrap();
        crate::testing::phase("terminal settlement after backpressure", publication).await;
        let Event::History(answer) = incoming.try_recv().unwrap() else {
            panic!("terminal history")
        };
        let terminal = answer.into_reply();
        assert!(model.answer(terminal.clone()).applied);
        assert!(model.importing().is_none());
        assert_eq!(model.import_failed(), Some(Problem::Uncertain));
        assert_eq!(model.refresh().unwrap().len(), 1);
        assert!(
            !model.answer(terminal).applied,
            "duplicate terminal cannot settle the fresh read"
        );
        assert!(model.loading());
        assert!(incoming.try_recv().is_err());
        assert_eq!(
            peer.calls.lock().unwrap().len(),
            1,
            "settlement must never replay create"
        );
        peer.finish().await;
    })
    .await;
}
