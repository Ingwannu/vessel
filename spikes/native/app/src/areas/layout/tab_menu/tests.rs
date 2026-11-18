//! The tab menu (TRACKER 797): one builder for the strip and the switcher,
//! Paseo's rows per tab kind, and what its copies put on the clipboard;
//! Reload agent and its toasts (TRACKER 803).
use super::*;
use crate::areas::layout::menu::chose;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{Entity, IntoElement, Window, div};
use vessel_app_view::layout::{AgentRole, LayoutMode, OpenIntent, Placement};
use vessel_app_view::{Card, Machine, Reach};
use vessel_app_link::{Act, ActAnswer, ActFailed, ActKind, Acted, Command};
use vessel_proto::{
    AgentAbilities, ErrorCode, HarnessKind, HarnessSessionId, ProtocolVersion, SessionInfo,
    SessionMode, SessionSpec, TerminalId,
};

/// The shell, its tabs (the open agent's, another agent's, a terminal's),
/// the other agent's session, the terminal, and what the shell sent.
struct Probe(
    Entity<Shell>,
    [TabId; 3],
    SessionId,
    TerminalId,
    tokio::sync::mpsc::Receiver<Command>,
);

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let other = SessionId::random();
    let terminal = TerminalId::random();
    let mut tabs = None;
    let mut sent = None;
    let shell = cx.new(|cx| {
        let (mut shell, commands) = language_tests::with_commands(window, cx);
        sent = Some(commands);
        shell.route = Route::Session;
        let session = SessionId::random();
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
        tabs = with_layout(&shell, |layout| {
            let mut open = |target| {
                layout.open(
                    target,
                    OpenIntent::Reveal,
                    Placement::Focused,
                    AgentRole::Unknown,
                    LayoutMode::SplitPanes,
                )
            };
            let agent = open(Target::Agent(session))?;
            let second = open(Target::Agent(other))?;
            let shell_tab = open(Target::Terminal(terminal))?;
            layout.select(agent)?;
            Ok([agent, second, shell_tab])
        })
        .ok();
        shell
    });
    Probe(shell, tabs.unwrap(), other, terminal, sent.unwrap())
}

/// A structured session's description, with its agent's own id.
fn info(session: SessionId, kind: HarnessKind, native: Option<&str>) -> SessionInfo {
    let mut spec = SessionSpec::new(
        kind,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    spec.cwd = Some("/work/shop".into());
    let mut info = SessionInfo::new(session, spec, chrono::DateTime::UNIX_EPOCH);
    info.harness_session = native.map(HarnessSessionId::new);
    info
}

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.add_window_view(fixture)
}

fn labels(items: &[(Action, MenuItem)]) -> Vec<String> {
    items
        .iter()
        .map(|(_, item)| item.label.to_string())
        .collect()
}

/// Choose `action` on `tab`; the toast up after it, its words and tone.
fn choose(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    tab: TabId,
    action: Action,
) -> Option<(String, ToastTone)> {
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            chose(shell, tab, action, window, cx);
            shell
                .areas
                .toasts
                .shown()
                .map(|toast| (toast.text.clone(), toast.tone))
        })
    })
}

/// The toast a copy says: its words, Paseo's `toast.copied`.
fn copy<E>(words: Result<String, E>) -> Option<(String, ToastTone)> {
    words.ok().map(|text| (text, ToastTone::Copied))
}

/// The toast a copy that could not be made says: Paseo's `toast.error`.
fn error(words: &str) -> (String, ToastTone) {
    (words.to_owned(), ToastTone::Error)
}

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

#[gpui::test]
fn one_builder_names_only_the_directional_closes_by_surface(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let words = shell.words.layout();
        for tab in probe.1 {
            let strip = entries(shell, tab, Surface::Strip);
            let switcher = entries(shell, tab, Surface::Switcher);
            let actions = |items: &[(Action, MenuItem)]| -> Vec<Action> {
                items.iter().map(|(action, _)| *action).collect()
            };
            assert_eq!(actions(&strip), actions(&switcher));
            let renamed: Vec<String> = labels(&strip)
                .into_iter()
                .map(|label| {
                    if label == words.close_left() {
                        words.close_above().to_owned()
                    } else if label == words.close_right() {
                        words.close_below().to_owned()
                    } else {
                        label
                    }
                })
                .collect();
            assert_eq!(renamed, labels(&switcher));
        }
        // Paseo's desktop words: "Close to the left / right".
        assert_eq!(words.close_left(), "Close to the left");
        assert_eq!(words.close_right(), "Close to the right");
    });
}

#[gpui::test]
fn an_agent_leads_with_paseos_two_copies_and_a_terminal_with_its_hinted_id(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let words = shell.words.layout();
        let [_, other, terminal] = probe.1;
        // Another agent's tab: Paseo's rows, no session extras.
        let items = entries(shell, other, Surface::Strip);
        assert_eq!(
            labels(&items),
            [
                words.copy_resume_command(),
                words.copy_agent_id(),
                words.close_left(),
                words.close_right(),
                words.close_others(),
                words.close(),
            ]
        );
        assert_eq!(items[0].0, Action::CopyResume);
        assert_eq!(items[1].0, Action::Copy(Copied::AgentId));
        assert_eq!(items[0].1.value, None);
        let hint: String = probe.2.to_string().chars().take(7).collect();
        assert_eq!(items[1].1.value.as_deref(), Some(hint.as_str()));
        let groups: Vec<bool> = items.iter().map(|(_, item)| item.starts_group).collect();
        assert_eq!(groups, [false, false, true, false, false, false]);
        assert!(items.iter().all(|(_, item)| item.leading.is_some()));
        // A terminal: Copy terminal id with its hint, then the rule.
        let items = entries(shell, terminal, Surface::Strip);
        assert_eq!(labels(&items)[0], words.copy_terminal_id());
        assert_eq!(items[0].0, Action::Copy(Copied::TerminalId));
        let hint: String = probe.3.to_string().chars().take(7).collect();
        assert_eq!(items[0].1.value.as_deref(), Some(hint.as_str()));
        assert!(items[1].1.starts_group);
    });
}

#[gpui::test]
fn copy_resume_command_copies_the_clis_command_or_says_why_not(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let ([mine, other, _], other_session) = probe.read_with(cx, |probe, _| (probe.1, probe.2));
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    // The open session before its agent named its own session.
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            let session = shell.areas.session.open.as_ref().unwrap().session;
            shell.areas.conversation.info =
                Some(Box::new(info(session, HarnessKind::ClaudeCode, None)));
        });
    });
    let said = choose(&probe, cx, mine, Action::CopyResume);
    assert_eq!(said, Some(error(words.resume_id_unavailable())));
    assert_eq!(clipboard(cx), None);
    // Once it did: Paseo's template for Claude Code, and its "Copied …"
    // (built by the same catalog call, Fluent's isolation marks included).
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            if let Some(info) = shell.areas.conversation.info.as_mut() {
                info.harness_session = Some(HarnessSessionId::new("4f1c2a"));
            }
        });
    });
    let said = choose(&probe, cx, mine, Action::CopyResume);
    assert_eq!(said, copy(words.copied(words.copied_resume_command())));
    assert_eq!(clipboard(cx).as_deref(), Some("claude --resume 4f1c2a"));
    // Another agent's tab reads its card: Codex's command.
    let card = |kind| {
        let mut machine = Machine {
            name: "laptop".to_owned(),
            reach: Reach::Online,
            sessions: Vec::new(),
            agents: Vec::new(),
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
            node: None,
            connection_epoch: None,
        };
        machine.sessions.push(Card::from_session(
            "laptop",
            &info(other_session, kind, Some("9e8d")),
        ));
        machine
    };
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            shell.fleet.machines = vec![card(HarnessKind::Codex)];
        });
    });
    let said = choose(&probe, cx, other, Action::CopyResume);
    assert_eq!(said, copy(words.copied(words.copied_resume_command())));
    assert_eq!(clipboard(cx).as_deref(), Some("codex resume 9e8d"));
    // Gemini has no template in Paseo's table.
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            shell.fleet.machines = vec![card(HarnessKind::Gemini)];
        });
    });
    let said = choose(&probe, cx, other, Action::CopyResume);
    assert_eq!(said, Some(error(words.resume_command_unavailable())));
    assert_eq!(clipboard(cx).as_deref(), Some("codex resume 9e8d"));
}

#[gpui::test]
fn the_id_copies_put_the_tabs_id_on_the_clipboard_and_say_so(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let ([_, other, terminal], session, shell_id) =
        probe.read_with(cx, |probe, _| (probe.1, probe.2, probe.3));
    // Paseo's "Copied …", built by the same catalog call (so Fluent's
    // isolation marks around the label match).
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    let said = choose(&probe, cx, other, Action::Copy(Copied::AgentId));
    assert_eq!(said, copy(words.copied(words.copied_agent_id())));
    assert_eq!(clipboard(cx), Some(session.to_string()));
    let said = choose(&probe, cx, terminal, Action::Copy(Copied::TerminalId));
    assert_eq!(said, copy(words.copied(words.copied_terminal_id())));
    assert_eq!(clipboard(cx), Some(shell_id.to_string()));
}

/// The laptop speaking `protocol` (`None`: not connected), listing the
/// other agent's session, whose agent can load a conversation when `loads`.
fn laptop(other: SessionId, protocol: Option<ProtocolVersion>, loads: bool) -> Machine {
    let mut session = info(other, HarnessKind::Codex, Some("9e8d"));
    session.abilities = Some(AgentAbilities {
        resume: loads,
        ..AgentAbilities::default()
    });
    Machine {
        name: "laptop".to_owned(),
        reach: Reach::Online,
        sessions: vec![Card::from_session("laptop", &session)],
        agents: Vec::new(),
        protocol,
        version: None,
        node: None,
        connection_epoch: None,
    }
}

fn put_laptop(probe: &Entity<Probe>, cx: &mut VisualTestContext, machine: Machine) {
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| shell.fleet.machines = vec![machine]);
    });
}

fn reloading() -> Option<ProtocolVersion> {
    vessel_proto::reload::RELOAD_PROTOCOL
}

#[gpui::test]
fn reload_agent_follows_close_other_tabs_with_paseos_tooltip_where_its_node_can(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(cx);
    let ([_, other, terminal], session) = probe.read_with(cx, |probe, _| (probe.1, probe.2));
    put_laptop(&probe, cx, laptop(session, reloading(), true));
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let words = shell.words.layout();
        // Paseo's order: Close other tabs, Reload agent, Close.
        let items = entries(shell, other, Surface::Strip);
        assert_eq!(
            labels(&items),
            [
                words.copy_resume_command(),
                words.copy_agent_id(),
                words.close_left(),
                words.close_right(),
                words.close_others(),
                words.reload_agent(),
                words.close(),
            ]
        );
        let (action, reload) = &items[5];
        assert_eq!(*action, Action::Reload);
        assert_eq!(reload.leading, Some(Glyph::RotateCw));
        assert_eq!(
            reload.tooltip.as_deref(),
            Some("Reload agent to update skills, MCPs or login status.")
        );
        assert!(!reload.disabled && !reload.starts_group);
        let switcher = entries(shell, other, Surface::Switcher);
        assert_eq!(switcher[5].0, Action::Reload);
        // A terminal has none.
        let items = entries(shell, terminal, Surface::Strip);
        assert!(!items.iter().any(|(action, _)| *action == Action::Reload));
    });
    let offered = |probe: &Entity<Probe>, cx: &mut VisualTestContext| {
        probe.read_with(cx, |probe, cx| {
            entries(probe.0.read(cx), probe.1[1], Surface::Strip)
                .iter()
                .any(|(action, _)| *action == Action::Reload)
        })
    };
    // A connected node older than the reload protocol: none.
    put_laptop(&probe, cx, laptop(session, Some(ProtocolVersion::new(30)), true));
    assert!(!offered(&probe, cx));
    // A disconnected one: offered, as Paseo's (a press says so).
    put_laptop(&probe, cx, laptop(session, None, true));
    assert!(offered(&probe, cx));
    // An agent that cannot load its conversation: none, as the page's.
    put_laptop(&probe, cx, laptop(session, reloading(), false));
    assert!(!offered(&probe, cx));
}

/// The reload's answer for `session`, as the link gives it.
fn answered(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    session: SessionId,
    result: Result<Acted, ActFailed>,
) -> Option<(String, ToastTone)> {
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            let passed = shell.tab_menu_reload_acted(
                ActAnswer {
                    terminal: None,
                    machine: "laptop".to_owned(),
                    session,
                    kind: ActKind::Reload,
                    result,
                },
                cx,
            );
            // A success goes on to the conversation; a failure is the
            // toast's alone.
            assert_eq!(passed.is_some(), shell.areas.toasts.shown().is_some_and(
                |toast| toast.tone == ToastTone::Ok
            ));
            shell
                .areas
                .toasts
                .shown()
                .map(|toast| (toast.text.clone(), toast.tone))
        })
    })
}

#[gpui::test]
fn reload_holds_its_toast_until_the_answer_says_how_it_went(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let ([_, other, _], session) = probe.read_with(cx, |probe, _| (probe.1, probe.2));
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    put_laptop(&probe, cx, laptop(session, reloading(), true));
    let said = choose(&probe, cx, other, Action::Reload);
    let held = Some((words.reloading_agent().to_owned(), ToastTone::Plain));
    assert_eq!(said, held);
    let sent = probe.update(cx, |probe, _| probe.4.try_recv().ok());
    assert!(matches!(
        sent,
        Some(Command::Act { machine, session: acted, act: Act::Reload })
            if machine == "laptop" && acted == session
    ));
    // Paseo's `durationMs: null`: still up long after any toast's life.
    cx.executor().advance_clock(std::time::Duration::from_secs(60));
    cx.run_until_parked();
    let up = probe.read_with(cx, |probe, cx| {
        probe.0.read(cx).areas.toasts.shown().map(|toast| (toast.text.clone(), toast.tone))
    });
    assert_eq!(up, held);
    let reloaded = info(session, HarnessKind::Codex, Some("9e8d"));
    let said = answered(&probe, cx, session, Ok(Acted::Session(Box::new(reloaded))));
    assert_eq!(said, Some((words.reloaded_agent().to_owned(), ToastTone::Ok)));
    // A refusal says the node's words; none, Paseo's line.
    let refused = ActFailed {
        code: Some(ErrorCode::Conflict),
        message: "The running turn did not end.".to_owned(),
    };
    let said = answered(&probe, cx, session, Err(refused));
    assert_eq!(said, Some(error("The running turn did not end.")));
    let lost = ActFailed {
        code: None,
        message: " ".to_owned(),
    };
    let said = answered(&probe, cx, session, Err(lost));
    assert_eq!(said, Some(error(words.failed_to_reload_agent())));
}

#[gpui::test]
fn reload_on_a_disconnected_host_says_so_and_sends_nothing(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let ([_, other, _], session) = probe.read_with(cx, |probe, _| (probe.1, probe.2));
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    put_laptop(&probe, cx, laptop(session, None, true));
    let said = choose(&probe, cx, other, Action::Reload);
    assert_eq!(said, Some(error(words.host_disconnected())));
    let sent = probe.update(cx, |probe, _| probe.4.try_recv().ok());
    assert!(sent.is_none());
}
