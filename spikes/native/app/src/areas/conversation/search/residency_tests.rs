//! Replacing session resources preserves the single global search lifetime.

use gpui::{AppContext as _, Entity, Render, TestAppContext, VisualTestContext};
use tokio::sync::mpsc;
use vessel_app_kit::ui::{Context, Window, div};
use vessel_app_link::{Command, search::Ask};
use vessel_app_view::{Machine, Reach};
use vessel_proto::search::{MessageRole, SearchHit, SearchPage, Snippet, SnippetPart};
use vessel_proto::{EventSeq, SessionId};

use super::*;
use crate::areas::conversation::ConversationArea;
use crate::shell::language_tests;

struct Probe {
    shell: Entity<Shell>,
    commands: mpsc::Receiver<Command>,
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

fn answer(ask: &Ask) -> Searched {
    Searched {
        machine: ask.machine.clone(),
        ticket: ask.ticket,
        result: Ok(SearchPage {
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
        }),
    }
}

#[gpui::test]
fn replacing_conversation_residents_preserves_global_search_debounce_and_replies(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, inbox) = language_tests::with_commands(window, cx);
            commands = Some(inbox);
            shell.fleet.machines = vec![machine("one"), machine("two")];
            shell.message_search_query("needle", cx);
            shell.areas.conversation = ConversationArea::new(cx);
            shell
        });
        Probe {
            shell,
            commands: commands.unwrap(),
        }
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    let asks = probe.update(cx, |probe, _| {
        let mut asks = Vec::new();
        while let Ok(command) = probe.commands.try_recv() {
            if let Command::Search(ask) = command {
                asks.push(ask);
            }
        }
        asks
    });
    assert_eq!(asks.len(), 2, "the pre-replacement debounce remained live");
    update(&probe, cx, |shell, cx| {
        shell.message_searched(answer(&asks[0]), cx);
        assert_eq!(shell.message_search_rows().len(), 1);
        let ticket = shell.areas.palette.messages_search.view.ticket();
        shell.areas.conversation = ConversationArea::new(cx);
        assert_eq!(shell.areas.palette.messages_search.view.ticket(), ticket);
        assert_eq!(shell.message_search_rows().len(), 1);
        shell.message_searched(answer(&asks[1]), cx);
        assert_eq!(shell.message_search_rows().len(), 2);
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
            commands: commands.unwrap(),
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
