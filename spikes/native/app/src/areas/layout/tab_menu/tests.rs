//! The tab menu (TRACKER 797): one builder for the strip and the switcher,
//! Paseo's rows per tab kind, and what its copies put on the clipboard.
use super::*;
use crate::areas::layout::menu::chose;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{Entity, IntoElement, Window, div};
use vessel_app_view::layout::{AgentRole, LayoutMode, OpenIntent, Placement};
use vessel_app_view::{Card, Machine, Reach};
use vessel_proto::{
    HarnessKind, HarnessSessionId, SessionInfo, SessionMode, SessionSpec, TerminalId,
};

/// The shell, its tabs (the open agent's, another agent's, a terminal's),
/// the other agent's session and the terminal.
struct Probe(Entity<Shell>, [TabId; 3], SessionId, TerminalId);

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let other = SessionId::random();
    let terminal = TerminalId::random();
    let mut tabs = None;
    let shell = cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
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
    Probe(shell, tabs.unwrap(), other, terminal)
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

/// Choose `action` on `tab`; the newest toast's words, if any.
fn choose(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    tab: TabId,
    action: Action,
) -> Option<String> {
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            chose(shell, tab, action, window, cx);
            shell
                .areas
                .toasts
                .list()
                .last()
                .map(|toast| toast.text.clone())
        })
    })
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
    assert_eq!(said.as_deref(), Some(words.resume_id_unavailable()));
    assert_eq!(clipboard(cx), None);
    // Once it did: Paseo's template for Claude Code.
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            if let Some(info) = shell.areas.conversation.info.as_mut() {
                info.harness_session = Some(HarnessSessionId::new("4f1c2a"));
            }
        });
    });
    let said = choose(&probe, cx, mine, Action::CopyResume);
    assert_eq!(said.as_deref(), Some("Copied resume command"));
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
    assert_eq!(said.as_deref(), Some("Copied resume command"));
    assert_eq!(clipboard(cx).as_deref(), Some("codex resume 9e8d"));
    // Gemini has no template in Paseo's table.
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, _| {
            shell.fleet.machines = vec![card(HarnessKind::Gemini)];
        });
    });
    let said = choose(&probe, cx, other, Action::CopyResume);
    assert_eq!(said.as_deref(), Some(words.resume_command_unavailable()));
    assert_eq!(clipboard(cx).as_deref(), Some("codex resume 9e8d"));
}

#[gpui::test]
fn the_id_copies_put_the_tabs_id_on_the_clipboard_and_say_so(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx);
    let ([_, other, terminal], session, shell_id) =
        probe.read_with(cx, |probe, _| (probe.1, probe.2, probe.3));
    let said = choose(&probe, cx, other, Action::Copy(Copied::AgentId));
    assert_eq!(said.as_deref(), Some("Copied Agent ID"));
    assert_eq!(clipboard(cx), Some(session.to_string()));
    let said = choose(&probe, cx, terminal, Action::Copy(Copied::TerminalId));
    assert_eq!(said.as_deref(), Some("Copied Terminal ID"));
    assert_eq!(clipboard(cx), Some(shell_id.to_string()));
}
