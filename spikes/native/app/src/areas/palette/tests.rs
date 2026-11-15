//! Exercise the real palette input/actions through a shell with no external I/O.

use gpui::{AppContext as _, Entity, Render, TestAppContext, VisualTestContext};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_kit::ui::{Context, IntoElement as _, ParentElement as _, Styled as _, Window, div};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_link::{
    Command,
    search::{Answer, Ask},
};
use vessel_app_view::{Card, Machine, Reach};
use vessel_proto::search::{MessageRole, SearchHit, SearchPage, Snippet, SnippetPart};
use vessel_proto::{EventSeq, HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec};

use super::*;
use crate::shell::language_tests;

const QUIET: Duration = Duration::from_secs(1);

struct Probe {
    shell: Entity<Shell>,
    commands: mpsc::Receiver<Command>,
}

impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        self.shell.update(cx, |shell, cx| {
            div()
                .size_full()
                .children(render(shell, &Tokens::of(Scheme::Dark), window, cx))
                .into_any_element()
        })
    }
}

fn harness(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, inbox) = language_tests::with_commands(window, cx);
            commands = Some(inbox);
            shell.fleet.machines = vec![
                machine("one"),
                machine("two"),
                Machine {
                    reach: Reach::Unreachable("offline".to_owned()),
                    ..machine("offline")
                },
            ];
            shell.open_palette(window, cx);
            shell
        });
        cx.observe(&shell, |_probe, _shell, cx| cx.notify())
            .detach();
        Probe {
            shell,
            commands: commands.unwrap(),
        }
    })
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
    view: &Entity<Probe>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Shell, &mut Window, &mut Context<'_, Shell>) -> R,
) -> R {
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| f(shell, window, cx))
    })
}

fn ready(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.executor().advance_clock(QUIET);
    cx.run_until_parked();
}

fn asks(view: &Entity<Probe>, cx: &mut VisualTestContext) -> Vec<Ask> {
    view.update(cx, |probe, _cx| {
        let mut asks = Vec::new();
        while let Ok(command) = probe.commands.try_recv() {
            if let Command::Search(ask) = command {
                asks.push(ask);
            }
        }
        asks
    })
}

fn answer(ask: &Ask, session: SessionId, count: usize) -> Answer {
    Answer {
        machine: ask.machine.clone(),
        ticket: ask.ticket,
        result: Ok(SearchPage {
            hits: (0..count)
                .map(|seq| SearchHit {
                    session,
                    seq: EventSeq::new(u64::try_from(seq).unwrap()),
                    at: chrono::DateTime::from_timestamp(i64::try_from(seq).unwrap(), 0).unwrap(),
                    role: MessageRole::Reply,
                    snippet: Snippet {
                        parts: vec![SnippetPart {
                            text: "needle".to_owned(),
                            matched: true,
                        }],
                        cut_before: false,
                        cut_after: false,
                    },
                })
                .collect(),
            next: None,
            reached: None,
        }),
    }
}

fn enter_messages(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.palette_mode(PaletteMode::Messages, window, cx);
}

#[gpui::test]
fn real_input_queries_only_online_nodes_and_edits_invalidate_before_debounce(
    cx: &mut TestAppContext,
) {
    let (view, cx) = harness(cx);
    cx.simulate_input("needle");
    ready(cx);
    assert!(
        asks(&view, cx).is_empty(),
        "Commands never searches transcripts"
    );
    let segment = cx
        .debug_bounds("palette-mode-Messages")
        .expect("real Messages segment");
    cx.simulate_click(segment.center(), gpui::Modifiers::default());
    ready(cx);
    let original = asks(&view, cx);
    assert_eq!(
        original
            .iter()
            .map(|ask| ask.machine.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert!(
        original
            .iter()
            .all(|ask| ask.request.session.is_none() && ask.request.query.as_str() == "needle")
    );
    update(&view, cx, |shell, _window, cx| {
        shell.message_searched(answer(&original[0], SessionId::random(), 1), cx);
    });
    cx.run_until_parked();
    update(&view, cx, |shell, _window, _cx| {
        assert_eq!(shell.message_search_rows().len(), 1);
    });
    cx.simulate_input(" changed");
    cx.run_until_parked();
    update(&view, cx, |shell, _window, cx| {
        assert!(shell.message_search_rows().is_empty());
        shell.message_searched(answer(&original[1], SessionId::random(), 1), cx);
        assert!(
            shell.message_search_rows().is_empty(),
            "old reply ignored during debounce"
        );
    });
    ready(cx);
    let changed = asks(&view, cx);
    assert_eq!(changed.len(), 2);
    assert!(
        changed
            .iter()
            .all(|ask| ask.request.query.as_str() == "needle changed")
    );
    update(&view, cx, |shell, window, cx| {
        shell.palette_key(CommandKey::Escape, window, cx);
    });
    update(&view, cx, |shell, window, cx| {
        shell.message_searched(answer(&changed[0], SessionId::random(), 1), cx);
        assert!(shell.message_search_rows().is_empty());
        assert!(!shell.areas.palette.is_open());
        shell.open_palette(window, cx);
        assert_eq!(shell.areas.palette.mode, PaletteMode::Commands);
        assert!(shell.areas.palette.query.is_empty());
    });
}

#[gpui::test]
fn keyboard_more_keeps_palette_open_and_hit_opens_the_session_with_find(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    update(&view, cx, enter_messages);
    cx.simulate_input("needle");
    ready(cx);
    let asked = asks(&view, cx);
    let session = SessionId::random();
    update(&view, cx, |shell, _window, cx| {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let info = SessionInfo::new(session, spec, chrono::Utc::now());
        shell.fleet.machines[0]
            .sessions
            .push(Card::from_session("one", &info));
        shell.message_searched(
            answer(&asked[0], session, vessel_app_view::search::PAGE + 1),
            cx,
        );
        shell.message_searched(answer(&asked[1], session, 0), cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("up enter");
    cx.run_until_parked();
    update(&view, cx, |shell, _window, _cx| {
        assert!(shell.areas.palette.is_open());
        assert_eq!(
            shell.message_search_rows().len(),
            vessel_app_view::search::PAGE + 1
        );
        assert!(
            !shell
                .message_search_rows()
                .contains(&vessel_app_view::search::Row::More)
        );
    });
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    update(&view, cx, |shell, window, cx| {
        assert!(!shell.areas.palette.is_open());
        assert_eq!(shell.route, Route::Session);
        let open = shell.areas.session.open.as_ref().unwrap();
        assert_eq!((open.machine.as_str(), open.session), ("one", session));
        assert_eq!(
            shell.find_destination(),
            Some((
                "needle",
                Some(EventSeq::new(
                    u64::try_from(vessel_app_view::search::PAGE).unwrap()
                ))
            ))
        );
        assert!(
            shell.message_search_rows().is_empty(),
            "query captured before clearing"
        );
        shell.open_palette(window, cx);
        shell.close_palette(window, cx);
    });
}

#[gpui::test]
fn a_removed_session_is_reported_and_switching_scope_cancels_pending_work(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    update(&view, cx, enter_messages);
    cx.simulate_input("needle");
    ready(cx);
    let asked = asks(&view, cx);
    update(&view, cx, |shell, window, cx| {
        shell.message_searched(answer(&asked[0], SessionId::random(), 1), cx);
        shell.palette_key(CommandKey::Enter, window, cx);
        assert!(shell.areas.palette.unavailable.is_some());
        assert!(shell.areas.palette.is_open());
        shell.palette_mode(PaletteMode::Commands, window, cx);
        assert!(shell.message_search_rows().is_empty());
        assert_eq!(shell.areas.palette.query, "needle");
        shell.message_searched(answer(&asked[1], SessionId::random(), 1), cx);
        assert!(shell.message_search_rows().is_empty());
        shell.palette_mode(PaletteMode::Messages, window, cx);
        shell.close_palette(window, cx);
    });
    ready(cx);
    assert!(
        asks(&view, cx).is_empty(),
        "closed pending debounce sends no requests"
    );
}

#[gpui::test]
fn messages_language_change_preserves_typed_text_selection_and_focus(cx: &mut TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            shell.open_palette(window, cx);
            enter_messages(shell, window, cx);
        },
        |shell| &shell.areas.palette.search,
        |shell| {
            assert_eq!(shell.areas.palette.query, "typed ABCDE");
            assert_eq!(shell.areas.palette.mode, PaletteMode::Messages);
        },
    );
}

#[gpui::test]
fn a_keyboard_command_enters_messages_with_an_empty_query(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    cx.simulate_input("Search messages");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    update(&view, cx, |shell, window, cx| {
        assert!(shell.areas.palette.is_open());
        assert_eq!(shell.areas.palette.mode, PaletteMode::Messages);
        assert_eq!(shell.areas.palette.search.text(cx), "");
        assert!(
            shell
                .areas
                .palette
                .search
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    ready(cx);
    assert!(asks(&view, cx).is_empty());
}

#[gpui::test]
fn find_does_not_bypass_the_open_palette(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    update(&view, cx, |shell, window, cx| {
        shell.route = Route::Session;
        shell.areas.conversation.open("one", SessionId::random());
        shell.conversation_find(window, cx);
        assert!(shell.areas.conversation.find.is_none());
        shell.close_palette(window, cx);
        shell.conversation_find(window, cx);
        assert!(shell.areas.conversation.find.is_some());
    });
}

#[gpui::test]
fn global_messages_survive_replacing_a_conversation_resident(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    update(&view, cx, |shell, window, cx| {
        enter_messages(shell, window, cx);
        shell.message_search_query("needle", cx);
    });
    ready(cx);
    let pending = asks(&view, cx);
    assert_eq!(pending.len(), 2);
    update(&view, cx, |shell, _window, cx| {
        shell.message_searched(answer(&pending[0], SessionId::random(), 1), cx);
        assert_eq!(shell.message_search_rows().len(), 1);
        let replacement = crate::areas::conversation::ConversationArea::new(cx);
        let previous = std::mem::replace(&mut shell.areas.conversation, replacement);
        assert_eq!(
            shell.message_search_rows().len(),
            1,
            "the window owns results, not the resident"
        );
        shell.message_searched(answer(&pending[1], SessionId::random(), 1), cx);
        assert_eq!(
            shell.message_search_rows().len(),
            2,
            "the outstanding ticket still belongs to the window"
        );
        drop(previous);
    });
}

#[gpui::test]
fn readmitting_a_machine_invalidates_old_pages_before_debounce(cx: &mut TestAppContext) {
    use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
    let (view, cx) = harness(cx);
    let mut admissions = ConnectionEpochs::default();
    let node = vessel_proto::NodeId::random();
    let first = admissions
        .admit(MachineKey::from_book("one"), node)
        .unwrap();
    update(&view, cx, |shell, window, cx| {
        shell.fleet.machines[0].node = Some(node);
        shell.fleet.machines[0].connection_epoch = Some(first.epoch());
        enter_messages(shell, window, cx);
        shell.message_search_query("needle", cx);
    });
    ready(cx);
    let old = asks(&view, cx);
    assert_eq!(old[0].connection, Some(first));
    let next = admissions
        .admit(MachineKey::from_book("one"), node)
        .unwrap();
    update(&view, cx, |shell, _, cx| {
        shell.message_searched(answer(&old[0], SessionId::random(), 1), cx);
        assert_eq!(shell.message_search_rows().len(), 1);
        shell.fleet.machines[0].connection_epoch = Some(next.epoch());
        shell.message_search_fleet_changed(cx);
        assert!(shell.message_search_rows().is_empty());
        shell.message_searched(answer(&old[0], SessionId::random(), 1), cx);
        assert!(
            shell.message_search_rows().is_empty(),
            "the old generation is retired"
        );
    });
    ready(cx);
    let new = asks(&view, cx);
    assert_eq!(new.len(), 2);
    assert_eq!(new[0].connection, Some(next));
    assert_ne!(new[0].ticket, old[0].ticket);
}
