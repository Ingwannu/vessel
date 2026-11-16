//! Replacing session resources preserves the single global search lifetime.

use gpui::{AppContext as _, Entity, Render, TestAppContext, VisualTestContext};
use tokio::sync::mpsc;
use vessel_app_kit::ui::{Context, Window, div};
use vessel_app_link::{
    Command, Event, Link,
    search::Ask,
    testing::{PeerNodes, PeerSlot, Scenario},
};
use vessel_app_view::machine_book::Name;
use vessel_app_view::{Machine, Reach};
use vessel_proto::search::{MessageRole, SearchHit, SearchPage, Snippet, SnippetPart};
use vessel_proto::{Call, EventSeq, Outcome, Reply, SessionId};

use super::*;
use crate::areas::conversation::ConversationArea;
use crate::shell::language_tests;

struct Probe {
    shell: Entity<Shell>,
    commands: Option<mpsc::Receiver<Command>>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl gpui::IntoElement {
        div()
    }
}

fn machine(name: &str) -> Machine {
    Machine {
        name: name.to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
        node: None,
        connection_epoch: None,
    }
}

fn update<R>(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Shell, &mut Context<'_, Shell>) -> R,
) -> R {
    probe.update(cx, |probe, cx| probe.shell.update(cx, f))
}

fn page() -> SearchPage {
    SearchPage {
        hits: vec![SearchHit {
            session: SessionId::random(),
            seq: EventSeq::ZERO,
            at: chrono::DateTime::UNIX_EPOCH,
            role: MessageRole::Reply,
            snippet: Snippet {
                parts: vec![SnippetPart {
                    text: "needle".to_owned(),
                    matched: true,
                }],
                cut_before: false,
                cut_after: false,
            },
        }],
        next: None,
        reached: None,
    }
}
fn answer(ask: &Ask) -> Searched {
    Searched {
        machine: ask.machine.clone(),
        connection: ask.connection.clone(),
        delivery: None,
        ticket: ask.ticket,
        result: Ok(page()),
    }
}
fn actual_answer(scene: &mut Scenario, link: &Link, peer: PeerSlot) -> Searched {
    scene
        .reply_to(link, peer, Outcome::Ok(Reply::SearchMessages(page())))
        .unwrap();
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::Searched(answer) = scene.event(link).unwrap() {
            assert_eq!(answer.connection, scene.context_of(peer));
            assert!(
                answer.delivery.is_some(),
                "only the real actor issues validity"
            );
            assert!(answer.result.is_ok());
            return answer;
        }
    }
    panic!("expected the bounded actual search answer");
}
fn admitted_machine(context: &ConnectionContext) -> Machine {
    Machine {
        node: Some(context.node()),
        connection_epoch: Some(context.epoch()),
        ..machine(context.machine().as_str())
    }
}

#[gpui::test]
fn replacing_conversation_residents_preserves_global_search_debounce_and_replies(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut scene) = Scenario::start_pair(
        &Name::parse("one").unwrap(),
        &Name::parse("two").unwrap(),
        PeerNodes::SameNode,
    )
    .unwrap();
    let sources = [
        scene.context(),
        scene.context_of(PeerSlot::Secondary).unwrap(),
    ];
    let (probe, cx) = cx.add_window_view(move |window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::with_test_link(window, cx, link);
            shell.fleet.machines = sources.iter().map(admitted_machine).collect();
            shell.message_search_query("needle", cx);
            shell.areas.conversation = ConversationArea::new(cx);
            shell
        });
        Probe {
            shell,
            commands: None,
        }
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    update(&probe, cx, |shell, cx| {
        for peer in [PeerSlot::Primary, PeerSlot::Secondary] {
            let Call::SearchMessages(request) =
                scene.request_from(shell.fixture_link(), peer).unwrap()
            else {
                panic!("expected the ordinary search command to reach its original peer");
            };
            assert_eq!(request.query.as_str(), "needle");
            assert!(request.session.is_none());
        }
        let frames = scene
            .transcript_of(shell.fixture_link(), PeerSlot::Primary)
            .unwrap()
            .len()
            .saturating_add(
                scene
                    .transcript_of(shell.fixture_link(), PeerSlot::Secondary)
                    .unwrap()
                    .len(),
            );
        assert_eq!(frames, 2, "the pre-replacement debounce remained live");
        let first = actual_answer(&mut scene, shell.fixture_link(), PeerSlot::Primary);
        shell.message_searched(first, cx);
        assert_eq!(shell.message_search_rows().len(), 1);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        shell.areas.conversation = ConversationArea::new(cx);
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(shell.message_search_rows().len(), 1);
        let second = actual_answer(&mut scene, shell.fixture_link(), PeerSlot::Secondary);
        shell.message_searched(second, cx);
        assert_eq!(shell.message_search_rows().len(), 2);
        assert_eq!(
            scene
                .transcript_of(shell.fixture_link(), PeerSlot::Primary)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            scene
                .transcript_of(shell.fixture_link(), PeerSlot::Secondary)
                .unwrap()
                .len(),
            1
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn message_search_failure_facts_reword_without_restarting_the_query(cx: &mut TestAppContext) {
    use vessel_app_view::localization::{Locale, Translator};
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, inbox) = language_tests::with_commands(window, cx);
            commands = Some(inbox);
            shell.fleet.machines = vec![machine("one"), machine("two")];
            shell.message_search_query("needle", cx);
            shell
        });
        Probe {
            shell,
            commands: Some(commands.unwrap()),
        }
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let ticket = shell.areas.palette.messages_search.view.ticket();
            for (machine, why) in [
                ("one", Refusal::TimedOut),
                (
                    "two",
                    Refusal::Failed {
                        code: Some(vessel_proto::ErrorCode::Forbidden),
                    },
                ),
            ] {
                shell.message_searched(
                    Searched {
                        machine: machine.to_owned(),
                        connection: None,
                        delivery: None,
                        ticket,
                        result: Err(why),
                    },
                    cx,
                );
            }
            let notes = shell.areas.palette.messages_search.view.notes();
            assert_eq!(
                failure_text(shell, SearchFailure::TimedOut),
                "Search timed out. Try again."
            );
            assert_eq!(
                failure_text(shell, SearchFailure::Failed),
                "This machine could not search its messages. Try again."
            );
            shell.apply_language(
                Translator::get(Locale::from_tag("ko").unwrap()).unwrap(),
                window,
                cx,
            );
            assert_eq!(
                failure_text(shell, SearchFailure::TimedOut),
                "검색 제한 시간을 초과했습니다. 다시 시도해 주세요."
            );
            assert_eq!(
                failure_text(shell, SearchFailure::Failed),
                "이 머신의 메시지를 검색하지 못했습니다. 다시 시도해 주세요."
            );
            let view = &shell.areas.palette.messages_search.view;
            assert_eq!(view.ticket(), ticket);
            assert_eq!(view.query(), Some("needle"));
            assert_eq!(
                view.notes(),
                notes,
                "translate facts without resetting results"
            );
        });
    });
}

#[gpui::test]
fn a_success_without_an_actor_delivery_guard_cannot_install_search_rows(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, inbox) = language_tests::with_commands(window, cx);
            commands = Some(inbox);
            shell.fleet.machines = vec![machine("one")];
            shell.message_search_query("needle", cx);
            shell
        });
        Probe {
            shell,
            commands: Some(commands.unwrap()),
        }
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    let ask = probe.update(cx, |probe, _| {
        std::iter::from_fn(|| probe.commands.as_mut().unwrap().try_recv().ok())
            .find_map(|command| match command {
                Command::Search(ask) => Some(ask),
                _ => None,
            })
            .unwrap()
    });
    update(&probe, cx, |shell, cx| {
        // Detached fixtures cannot issue real validity. This tests rejection,
        // not successful wire delivery or an authorized connected machine.
        let forged = answer(&ask);
        assert!(forged.delivery.is_none());
        shell.message_searched(forged, cx);
        assert!(shell.message_search_rows().is_empty());
    });
}
