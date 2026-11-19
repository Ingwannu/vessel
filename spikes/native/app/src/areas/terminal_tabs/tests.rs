//! Terminal tabs: a create opens its tab in place of the New tab tab and
//! follows it; keys and pastes queue in order through the pane's sender
//! until it is live; Close asks first and closes on the node; a list adds
//! and removes tabs as Paseo's reconcile does (TRACKER 481, 482, 342, 338).

use super::*;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use chrono::Utc;
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext};
use tokio::sync::mpsc::Receiver;
use vessel_app_kit::ui::{IntoElement, div};
use vessel_app_link::Command;
use vessel_app_link::terminals::{Answer, Reply, Request};
use vessel_app_view::layout::{
    AgentRole, LayoutAction, LayoutMode, OpenIntent, Placement, TabId, Target,
};
use vessel_app_view::terminal::Outgoing;
use vessel_app_view::{Machine, Reach};
use vessel_proto::SessionId;
use vessel_proto::terminals::{TerminalKind, TerminalList};

struct Probe(Entity<Shell>, Receiver<Command>, SessionId, TabId);

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

fn laptop() -> Machine {
    Machine {
        name: "laptop".to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
        node: None,
        connection_epoch: None,
    }
}

fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let session = SessionId::random();
    let mut parts = None;
    let shell = cx.new(|cx| {
        let (mut shell, commands) = language_tests::with_commands(window, cx);
        shell.route = Route::Session;
        shell.fleet.machines = vec![laptop()];
        shell.areas.session.open = Some(Open {
            machine: "laptop".to_owned(),
            session,
            title: "Fix the cart".to_owned(),
            agent: "Claude".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        let new_tab = crate::areas::layout::current_key(&shell)
            .and_then(|key| {
                shell.update_layout(
                    &key,
                    |layout| {
                        let _agent = layout.open(
                            Target::Agent(session),
                            OpenIntent::Reveal,
                            Placement::Focused,
                            AgentRole::Root,
                            LayoutMode::SplitPanes,
                        )?;
                        layout.open(
                            Target::NewTab,
                            OpenIntent::New,
                            Placement::Focused,
                            AgentRole::Unknown,
                            LayoutMode::SplitPanes,
                        )
                    },
                    cx,
                )
            })
            .unwrap();
        parts = Some((commands, new_tab));
        shell
    });
    let (commands, new_tab) = parts.unwrap();
    Probe(shell, commands, session, new_tab)
}

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.add_window_view(fixture)
}

/// Every terminal command sent since the last look.
fn sent(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> Vec<Request> {
    probe.update(cx, |probe, _cx| {
        let mut asked = Vec::new();
        while let Ok(command) = probe.1.try_recv() {
            if let Command::Terminals(ask) = command {
                assert_eq!(ask.machine, "laptop");
                asked.push(ask.request);
            }
        }
        asked
    })
}

fn shell_info(session: SessionId) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Session { session },
        "/work/shop".into(),
        TerminalKind::Shell {
            program: "/bin/sh".into(),
        },
        Utc::now(),
    )
}

fn answer(shell: &mut Shell, reply: Reply, cx: &mut Context<'_, Shell>) {
    shell.terminal_tabs_answer(
        Answer {
            machine: "laptop".to_owned(),
            reply,
        },
        cx,
    );
}

fn terminal_tabs(shell: &Shell) -> Vec<TerminalId> {
    let key = crate::areas::layout::current_key(shell).unwrap();
    crate::areas::layout::read_layout(shell, &key, |layout| {
        layout
            .tabs()
            .filter_map(|tab| {
                if let Target::Terminal(id) = tab.target() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    })
}

#[gpui::test]
fn terminal_creates_a_shell_whose_tab_replaces_the_new_tab(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let (session, new_tab) = probe.read_with(cx, |probe, _| (probe.2, probe.3));
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_new(Some(new_tab), window, cx);
    });
    let asked = sent(&probe, cx);
    let place = TerminalPlace::Session { session };
    assert!(matches!(asked.as_slice(), [Request::Create { place: at }] if *at == place));
    let info = shell_info(session);
    let terminal = info.id;
    shell.update_in(cx, |shell, _window, cx| {
        answer(
            shell,
            Reply::Created {
                place: place.clone(),
                result: Ok(Box::new(info)),
            },
            cx,
        );
        assert_eq!(terminal_tabs(shell), vec![terminal]);
        let key = crate::areas::layout::current_key(shell).unwrap();
        let new_tab_left = crate::areas::layout::read_layout(shell, &key, |layout| {
            layout.tab(new_tab).map(|tab| tab.target().clone())
        });
        // The New tab tab became the terminal's (Paseo's `replace`).
        assert_eq!(new_tab_left, Some(Target::Terminal(terminal)));
        assert_eq!(label(shell, terminal), "Terminal");
    });
    let asked = sent(&probe, cx);
    assert!(matches!(
        asked.as_slice(),
        [Request::Follow { terminal: followed, .. }] if *followed == terminal
    ));
}

#[gpui::test]
fn a_failed_create_says_so_and_opens_nothing(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_new(None, window, cx);
        answer(
            shell,
            Reply::Created {
                place: TerminalPlace::Session { session },
                result: Err(vessel_app_view::terminal_tabs::Failure::Refused(
                    vessel_proto::ErrorCode::Unavailable,
                )),
            },
            cx,
        );
        assert!(terminal_tabs(shell).is_empty());
        let said = shell.areas.toasts.shown().map(|toast| toast.text.clone());
        assert_eq!(said.as_deref(), Some("Failed to create terminal"));
    });
}

#[gpui::test]
fn keys_and_a_paste_wait_in_order_until_the_shell_is_live(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let info = shell_info(session);
    let key = PaneKey {
        machine: "laptop".to_owned(),
        terminal: info.id,
    };
    shell.update_in(cx, |shell, _window, cx| {
        let pane = Pane::new(Origin::Opened, cx, &key);
        let _none = shell.areas.terminal_tabs.panes.insert(key.clone(), pane);
        shell.terminal_tab_type(&key, "ls", cx);
        shell.terminal_tab_type(&key, "\r", cx);
    });
    // Attaching: nothing is sent yet.
    assert!(sent(&probe, cx).is_empty());
    shell.update_in(cx, |shell, _window, cx| {
        answer(shell, Reply::Following(Box::new(info.clone())), cx);
    });
    let asked = sent(&probe, cx);
    assert!(matches!(
        asked.as_slice(),
        [Request::Send { outgoing: Outgoing::Keys(keys), .. }] if keys == "ls\r"
    ));
    // One send in flight: the next keys wait for its answer.
    shell.update_in(cx, |shell, _window, cx| {
        shell.terminal_tab_type(&key, "pwd\r", cx);
    });
    assert!(sent(&probe, cx).is_empty());
    shell.update_in(cx, |shell, _window, cx| {
        answer(
            shell,
            Reply::Sent {
                terminal: key.terminal,
                result: Ok(()),
            },
            cx,
        );
    });
    let asked = sent(&probe, cx);
    assert!(matches!(
        asked.as_slice(),
        [Request::Send { outgoing: Outgoing::Keys(keys), .. }] if keys == "pwd\r"
    ));
}

#[gpui::test]
fn close_asks_first_then_closes_the_terminal_on_its_machine(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let info = shell_info(session);
    let terminal = info.id;
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_new(None, window, cx);
        answer(
            shell,
            Reply::Created {
                place: TerminalPlace::Session { session },
                result: Ok(Box::new(info)),
            },
            cx,
        );
    });
    let _created = sent(&probe, cx);
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.run_layout_action(
            LayoutAction::Close(vessel_app_view::layout::CloseScope::Current),
            window,
            cx
        ));
        assert!(shell.areas.terminal_tabs.confirm.is_some());
        // Cancel keeps it.
        shell.terminal_tab_keep(window, cx);
        assert!(shell.areas.terminal_tabs.confirm.is_none());
        assert_eq!(terminal_tabs(shell), vec![terminal]);
        assert!(shell.run_layout_action(
            LayoutAction::Close(vessel_app_view::layout::CloseScope::Current),
            window,
            cx
        ));
        shell.terminal_tab_close(window, cx);
        assert!(terminal_tabs(shell).is_empty());
    });
    let asked = sent(&probe, cx);
    assert!(matches!(
        asked.as_slice(),
        [Request::Close { terminal: closed }] if *closed == terminal
    ));
}

#[gpui::test]
fn a_list_adds_the_places_shells_and_drops_ended_ones(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let place = TerminalPlace::Session { session };
    let first = shell_info(session);
    let second = shell_info(session);
    let (one, two) = (first.id, second.id);
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tabs_poll();
        answer(
            shell,
            Reply::Listed {
                place: place.clone(),
                result: Ok(TerminalList {
                    terminals: vec![first.clone(), second.clone()],
                }),
            },
            cx,
        );
        shell.terminal_tabs_reconcile(crate::areas::layout::mode(window), cx);
        let mut both = vec![one, two];
        both.sort_unstable();
        let mut shown = terminal_tabs(shell);
        shown.sort_unstable();
        assert_eq!(shown, both);
        // The second ended: its tab goes, as an exited terminal leaves
        // Paseo's list.
        let mut ended = second.clone();
        ended.status = vessel_proto::SessionStatus::Exited { code: Some(0) };
        shell.terminal_tabs_poll();
        answer(
            shell,
            Reply::Listed {
                place: place.clone(),
                result: Ok(TerminalList {
                    terminals: vec![first.clone(), ended],
                }),
            },
            cx,
        );
        shell.terminal_tabs_reconcile(crate::areas::layout::mode(window), cx);
        assert_eq!(terminal_tabs(shell), vec![one]);
    });
    let asked = sent(&probe, cx);
    assert_eq!(
        asked
            .iter()
            .filter(|request| matches!(request, Request::List { .. }))
            .count(),
        2
    );
}

#[gpui::test]
fn a_focused_tab_is_the_shortcuts_terminal_scope(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let key = PaneKey {
        machine: "laptop".to_owned(),
        terminal: shell_info(session).id,
    };
    shell.update_in(cx, |shell, window, cx| {
        let pane = Pane::new(Origin::Opened, cx, &key);
        let _none = shell.areas.terminal_tabs.panes.insert(key.clone(), pane);
        assert!(!shell.terminal_focused(window));
        shell.terminal_tab_focus(&key, window, cx);
        assert!(shell.terminal_focused(window));
        shell.areas.session.composer.focus(window, cx);
        assert!(!shell.terminal_focused(window));
    });
}
