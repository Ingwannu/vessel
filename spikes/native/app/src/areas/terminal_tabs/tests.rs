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
    AgentRole, CloseScope, LayoutAction, LayoutMode, OpenIntent, Placement, TabId, Target,
};
use vessel_app_view::terminal::Outgoing;
use vessel_app_view::{Machine, Reach};
use vessel_proto::terminals::{TerminalKind, TerminalList};
use vessel_proto::{ResizeIntent, SessionId};

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
fn a_second_new_terminal_waits_for_the_first_create(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_new(None, window, cx);
        assert!(shell.terminal_tab_creating());
        // Paseo's `createTerminalDisabled`: a press while the first create
        // waits sends nothing.
        shell.terminal_tab_new(None, window, cx);
    });
    assert_eq!(sent(&probe, cx).len(), 1);
    shell.update_in(cx, |shell, window, cx| {
        answer(
            shell,
            Reply::Created {
                place: TerminalPlace::Session { session },
                result: Err(vessel_app_view::terminal_tabs::Failure::Offline),
            },
            cx,
        );
        assert!(!shell.terminal_tab_creating());
        shell.terminal_tab_new(None, window, cx);
    });
    assert!(matches!(
        sent(&probe, cx).as_slice(),
        [Request::Create { .. }]
    ));
}

#[gpui::test]
fn a_tab_that_lost_its_machine_follows_again_once_it_is_back(cx: &mut TestAppContext) {
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
    let _created_and_followed = sent(&probe, cx);
    shell.update_in(cx, |shell, _window, cx| {
        // The connection dropped while the fleet still reads it online.
        answer(
            shell,
            Reply::Lost {
                terminal,
                failure: vessel_app_view::terminal_tabs::Failure::Offline,
            },
            cx,
        );
        let key = PaneKey {
            machine: "laptop".to_owned(),
            terminal,
        };
        let stage = shell
            .areas
            .terminal_tabs
            .panes
            .get(&key)
            .map(|pane| pane.stage);
        assert!(matches!(stage, Some(Stage::Offline)));
        // The next fleet read finds the machine online.
        shell.terminal_tabs_rejoin();
    });
    assert!(matches!(
        sent(&probe, cx).as_slice(),
        [Request::Follow { terminal: followed, .. }] if *followed == terminal
    ));
}

#[gpui::test]
fn only_a_terminal_tabs_own_close_asks_to_close_it(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let (session, new_tab) = probe.read_with(cx, |probe, _| (probe.2, probe.3));
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_new(Some(new_tab), window, cx);
        answer(
            shell,
            Reply::Created {
                place: TerminalPlace::Session { session },
                result: Ok(Box::new(shell_info(session))),
            },
            cx,
        );
        let key = crate::areas::layout::current_key(shell).unwrap();
        // The agent's tab, then the terminal's: "close to the right" of the
        // agent would close only the terminal.
        let (own, right) = crate::areas::layout::read_layout(shell, &key, |layout| {
            let agent = layout
                .tabs()
                .find(|tab| matches!(tab.target(), Target::Agent(_)))
                .map(vessel_app_view::layout::Tab::id);
            (
                layout.close_plans(new_tab, CloseScope::Current).ok(),
                agent.and_then(|agent| layout.close_plans(agent, CloseScope::After).ok()),
            )
        });
        let (own, right) = (own.unwrap(), right.unwrap());
        assert_eq!(right.len(), 1);
        assert!(closes_terminal(CloseScope::Current, &own));
        // Paseo asks a bulk close with its own confirm, not drawn yet.
        assert!(!closes_terminal(CloseScope::After, &right));
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
        let pane = Pane::new(Origin::Opened, ScrollbackLines::default(), cx, &key);
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
        assert!(shell.run_layout_action(LayoutAction::Close(CloseScope::Current), window, cx));
        assert!(shell.areas.terminal_tabs.confirm.is_some());
        // Cancel keeps it.
        shell.terminal_tab_keep(window, cx);
        assert!(shell.areas.terminal_tabs.confirm.is_none());
        assert_eq!(terminal_tabs(shell), vec![terminal]);
        assert!(shell.run_layout_action(LayoutAction::Close(CloseScope::Current), window, cx));
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
                    revision: 1,
                }),
                waits: false,
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
                    revision: 2,
                }),
                waits: false,
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
        let pane = Pane::new(Origin::Opened, ScrollbackLines::default(), cx, &key);
        let _none = shell.areas.terminal_tabs.panes.insert(key.clone(), pane);
        assert!(!shell.terminal_focused(window));
        shell.terminal_tab_focus(&key, window, cx);
        assert!(shell.terminal_focused(window));
        shell.areas.session.composer.focus(window, cx);
        assert!(!shell.terminal_focused(window));
    });
}

fn listed(place: &TerminalPlace, revision: u64, waits: bool) -> Reply {
    Reply::Listed {
        place: place.clone(),
        result: Ok(TerminalList {
            terminals: Vec::new(),
            revision,
        }),
        waits,
    }
}

/// ADR 0116's long poll: asked again at once from the answered revision
/// while the list moves; an answer that did not move (the node's bound, or
/// a wait it had no room for) waits for the next fleet read; one ask in
/// flight, so fleet reads meanwhile ask nothing.
#[gpui::test]
fn a_node_that_waits_is_asked_again_only_when_the_list_moved(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let place = TerminalPlace::Session { session };
    let from = |requests: &[Request]| -> Vec<Option<u64>> {
        requests
            .iter()
            .filter_map(|request| match request {
                Request::List { changed_from, .. } => Some(*changed_from),
                Request::Create { .. }
                | Request::ListAll
                | Request::Follow { .. }
                | Request::Unfollow { .. }
                | Request::Send { .. }
                | Request::Close { .. } => None,
            })
            .collect()
    };
    shell.update_in(cx, |shell, _window, _cx| shell.terminal_tabs_poll());
    assert_eq!(
        from(&sent(&probe, cx)),
        [None],
        "the first ask answers at once"
    );
    shell.update_in(cx, |shell, _window, cx| {
        answer(shell, listed(&place, 4, true), cx);
    });
    assert_eq!(
        from(&sent(&probe, cx)),
        [Some(4)],
        "then waits from what it saw"
    );
    shell.update_in(cx, |shell, _window, _cx| shell.terminal_tabs_poll());
    assert!(from(&sent(&probe, cx)).is_empty(), "one ask in flight");
    shell.update_in(cx, |shell, _window, cx| {
        answer(shell, listed(&place, 4, true), cx);
    });
    assert!(
        from(&sent(&probe, cx)).is_empty(),
        "an unmoved answer is not re-asked"
    );
    shell.update_in(cx, |shell, _window, cx| {
        shell.terminal_tabs_poll();
        answer(shell, listed(&place, 5, true), cx);
    });
    assert_eq!(from(&sent(&probe, cx)), [Some(4), Some(5)]);
    // A node below the protocol answers at once: the fleet read asks.
    shell.update_in(cx, |shell, _window, cx| {
        answer(shell, listed(&place, 0, false), cx);
    });
    assert!(from(&sent(&probe, cx)).is_empty());
}

/// ADR 0116's size claim: the focused, live pane claims its size, later
/// fits are updates, and taking the keyboard again claims again, even at
/// the size already sent.
#[gpui::test]
fn a_pane_claims_its_size_on_focus_and_updates_it_after(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let info = shell_info(session);
    let terminal = info.id;
    let key = PaneKey {
        machine: "laptop".to_owned(),
        terminal,
    };
    let fit = |cols, rows| GridSize::new(cols, rows).ok();
    let sizes = |requests: Vec<Request>| -> Vec<(u16, u16, ResizeIntent)> {
        requests
            .into_iter()
            .filter_map(|request| match request {
                Request::Send {
                    outgoing: Outgoing::Resize { cols, rows, intent },
                    ..
                } => Some((cols, rows, intent)),
                Request::Send { .. }
                | Request::Create { .. }
                | Request::List { .. }
                | Request::ListAll
                | Request::Follow { .. }
                | Request::Unfollow { .. }
                | Request::Close { .. } => None,
            })
            .collect()
    };
    let sent_ok = |shell: &mut Shell, cx: &mut Context<'_, Shell>| {
        answer(
            shell,
            Reply::Sent {
                terminal,
                result: Ok(()),
            },
            cx,
        );
    };
    shell.update_in(cx, |shell, window, cx| {
        let mut pane = Pane::new(Origin::Opened, ScrollbackLines::default(), cx, &key);
        pane.stage = Stage::Live;
        pane.fit = fit(80, 24);
        let _none = shell.areas.terminal_tabs.panes.insert(key.clone(), pane);
        shell.terminal_tab_focused_in(&key, window, cx);
    });
    assert_eq!(sizes(sent(&probe, cx)), [(80, 24, ResizeIntent::Claim)]);
    shell.update_in(cx, |shell, _window, cx| {
        sent_ok(shell, cx);
        if let Some(pane) = shell.areas.terminal_tabs.panes.get_mut(&key) {
            pane.fit = fit(100, 30);
        }
        shell.terminal_tab_claim(&key);
        // The same fit again sends nothing.
        sent_ok(shell, cx);
        shell.terminal_tab_claim(&key);
    });
    assert_eq!(sizes(sent(&probe, cx)), [(100, 30, ResizeIntent::Update)]);
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tab_focused_out(&key, cx);
        shell.terminal_tab_claim(&key);
        shell.terminal_tab_focused_in(&key, window, cx);
    });
    assert_eq!(
        sizes(sent(&probe, cx)),
        [(100, 30, ResizeIntent::Claim)],
        "nothing while unfocused; the keyboard back claims again"
    );
}

/// P1 review note 4: after a confirmed Close the keyboard goes to the shell
/// the focused pane now shows, not to the closed pane.
#[gpui::test]
fn a_confirmed_close_hands_the_keyboard_to_the_shell_now_shown(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let first = shell_info(session);
    let second = shell_info(session);
    let kept = first.id;
    for info in [first, second] {
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
    }
    let _created = sent(&probe, cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.terminal_tabs.focus_next = None;
        assert!(shell.run_layout_action(LayoutAction::Close(CloseScope::Current), window, cx));
        shell.terminal_tab_close(window, cx);
        assert_eq!(terminal_tabs(shell), vec![kept]);
        assert_eq!(
            shell.areas.terminal_tabs.focus_next,
            Some(PaneKey {
                machine: "laptop".to_owned(),
                terminal: kept,
            })
        );
    });
}

/// A tab the list added and nobody opened yet shows the title another
/// client gave its shell (Paseo labels every listed tab), not the fallback.
#[gpui::test]
fn a_listed_tab_not_yet_opened_shows_its_title(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let session = probe.read_with(cx, |probe, _| probe.2);
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let place = TerminalPlace::Session { session };
    let mut named = shell_info(session);
    named.title = "Opened by the CLI".parse().ok();
    let id = named.id;
    shell.update_in(cx, |shell, window, cx| {
        shell.terminal_tabs_poll();
        answer(
            shell,
            Reply::Listed {
                place: place.clone(),
                result: Ok(TerminalList {
                    terminals: vec![named.clone()],
                    revision: 1,
                }),
                waits: false,
            },
            cx,
        );
        shell.terminal_tabs_reconcile(crate::areas::layout::mode(window), cx);
        assert_eq!(terminal_tabs(shell), vec![id]);
        assert_eq!(label(shell, id), "Opened by the CLI");
    });
}

mod p3;
mod places;
