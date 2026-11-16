//! Real actor preflight refuses pending Skills without a guard or a wire call.
//! Synthetic payload tests below prove weak-envelope validity only: the None
//! feature cannot provide a successful Skills RPC, and no test overrides it.
use super::*;
use crate::{Command, Event, testing::actor::Actor};
use std::future::Future as _;
use vessel_app_view::devices::Tickets;
use vessel_app_view::orchestration::{Action, Consumer, Output, Scope, Visit};
use vessel_proto::skills::{SaveSkillsSelection, SkillNames, SkillSelection};

const QUEUE_CAPACITY: usize = 1;

fn task(context: vessel_app_view::connection::ConnectionContext, action: Action) -> Task {
    let mut tickets = Tickets::default();
    Task {
        scope: Scope {
            consumer: Consumer::Settings,
            target: context,
            visit: Visit::new(tickets.next_ticket().unwrap()),
        },
        ticket: tickets.next_ticket().unwrap(),
        action,
    }
}
fn save() -> Action {
    Action::Save(SaveSkillsSelection {
        selection: SkillSelection::All,
        confirmed_removals: SkillNames::default(),
    })
}
fn payload() -> Output {
    Output::Skills(vessel_proto::skills::SkillsSnapshot {
        state: vessel_proto::skills::SkillsState::NotInstalled,
        ops: Vec::new(),
        available: SkillNames::default(),
        installed: SkillNames::default(),
        selection: SkillSelection::All,
    })
}

#[tokio::test]
async fn actual_actor_refuses_every_pending_skill_before_a_later_real_frame() {
    crate::testing::within(async {
        assert_eq!(vessel_proto::mcp::MCP_PARITY, None);
        let mut actor = Actor::new("host").await;
        for action in [
            Action::ReadSkills,
            Action::Reconcile,
            Action::Uninstall,
            save(),
        ] {
            let original = task(actor.context(), action);
            actor
                .dispatch(Command::Orchestration(original.clone()))
                .await;
            let Event::Orchestration(answer) = actor.next_event().await else {
                panic!("orchestration refusal");
            };
            assert!(
                answer.guard.is_none(),
                "preflight cannot manufacture a guard"
            );
            assert_eq!(answer.into_reply(), original.refused(Problem::Unsupported));
        }
        // A later actual request is a writer barrier, not an early empty-queue
        // guess. The shared Actor remains byte-identical to its owning blob.
        let context = actor.context();
        actor
            .dispatch(Command::History(crate::history::Ask::Profiles {
                scope: vessel_app_view::history::Scope {
                    machine: context.machine().clone(),
                    context,
                    harness: vessel_proto::HarnessKind::ClaudeCode.into(),
                    profile: None,
                    cwd: None,
                },
                ticket: Tickets::default().next_ticket().unwrap(),
            }))
            .await;
        let request = actor.request().await;
        assert!(matches!(request.call, vessel_proto::Call::ListProfiles(_)));
        request
            .answer
            .send(vessel_proto::Outcome::Ok(
                vessel_proto::Reply::ListProfiles(vec![]),
            ))
            .unwrap();
        let Event::History(answer) = actor.next_event().await else {
            panic!("history barrier");
        };
        assert!(matches!(
            answer.into_reply(),
            vessel_app_view::history::Reply::Profiles { result: Ok(_), .. }
        ));
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn queued_write_loses_success_after_original_owner_retirement() {
    crate::testing::within(async {
        let home = crate::testing::Home::new();
        let (mut state, _events) = crate::testing::state(&home);
        let (selected, _peer) = super::super::tests::selected(&mut state).await;
        let original = task(selected.context.clone(), save());
        let answer = Answer::observed(
            Reply {
                task: original.clone(),
                result: Ok(payload()),
            },
            selected.connection.delivery_guard(),
        );
        state.clients.get("host").unwrap().retire();
        let reply = answer.into_reply();
        assert_eq!(reply, original.refused(Problem::Uncertain));
    })
    .await;
}

#[tokio::test]
async fn retained_envelope_does_not_keep_the_original_selected_owner_alive() {
    crate::testing::within(async {
        let home = crate::testing::Home::new();
        let (mut state, _events) = crate::testing::state(&home);
        let (selected, _peer) = super::super::tests::selected(&mut state).await;
        let original = task(selected.context.clone(), Action::ReadSkills);
        let answer = Answer::observed(
            Reply {
                task: original.clone(),
                result: Ok(payload()),
            },
            selected.connection.delivery_guard(),
        );
        drop(selected);
        drop(state.clients.remove("host"));
        assert_eq!(answer.into_reply(), original.refused(Problem::Changed));
    })
    .await;
}

#[tokio::test]
async fn full_result_queue_and_retirement_settle_only_the_original_write() {
    crate::testing::within(async {
        let home = crate::testing::Home::new();
        let (mut state, _events) = crate::testing::state(&home);
        let (selected, _peer) = super::super::tests::selected(&mut state).await;
        let original = task(selected.context.clone(), save());
        let (send, mut receive) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        send.send(Event::Orchestration(Answer::refused(
            original.clone(),
            Problem::Busy,
        )))
        .await
        .unwrap();
        let publishing = publish(
            &selected.connection,
            &send,
            original.clone(),
            Ok(payload()),
            selected.connection.delivery_guard(),
        );
        let mut publishing = std::pin::pin!(publishing);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(publishing.as_mut().poll(&mut cx).is_pending());
        state.clients.get("host").unwrap().retire();
        let _occupied = receive.recv().await.unwrap();
        publishing.await;
        let Event::Orchestration(answer) = receive.recv().await.unwrap() else {
            panic!("orchestration refusal");
        };
        assert!(answer.guard.is_none());
        assert_eq!(answer.into_reply(), original.refused(Problem::Uncertain));
    })
    .await;
}
