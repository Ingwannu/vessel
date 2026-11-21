//! Settings › Terminals over a detached link: the page reads its machine's
//! list once, every change sends the whole list, the edit dialog says what
//! is missing and closes on the node's answer, a refusal is kept beside the
//! list, and New tab's rows come from the same list (ADR 0117).

use gpui::{
    AppContext as _, Entity, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext,
};
use tokio::sync::mpsc::Receiver;
use vessel_app_kit::ui::{ParentElement as _, Styled as _, div};
use vessel_app_link::Command;
use vessel_app_link::terminal_profiles::Answer;
use vessel_app_view::{Machine, Reach};
use vessel_proto::ErrorCode;

use super::*;
use crate::shell::language_tests;

struct Probe(Entity<Shell>, Receiver<Command>);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            shell.settle_terminal_profiles(window);
            let tokens = vessel_app_kit::tokens(cx);
            div()
                .size_full()
                .child(page::render(shell, &tokens, false, window, cx))
        })
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

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, sent) = language_tests::with_commands(window, cx);
            shell.route = Route::Settings;
            shell.areas.settings.section = Section::Terminals;
            shell.fleet.machines = vec![laptop()];
            commands = Some(sent);
            shell
        });
        Probe(shell, commands.unwrap())
    });
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(800.0)));
    cx.run_until_parked();
    (probe, cx)
}

/// Every profiles ask sent since the last look.
fn asks(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> Vec<Ask> {
    probe.update(cx, |probe, _cx| {
        let mut asked = Vec::new();
        while let Ok(command) = probe.1.try_recv() {
            if let Command::TerminalProfiles(ask) = command {
                assert_eq!(ask.machine.as_str(), "laptop");
                asked.push(ask);
            }
        }
        asked
    })
}

fn answer(
    probe: &Entity<Probe>,
    ask: &Ask,
    result: Result<Profiles, Problem>,
    cx: &mut VisualTestContext,
) {
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.terminal_profiles_answer(
                Answer {
                    machine: ask.machine.clone(),
                    ticket: ask.ticket,
                    result,
                },
                cx,
            );
            cx.notify();
        });
    });
    cx.run_until_parked();
}

fn shipped() -> Profiles {
    Profiles::shipped().unwrap()
}

fn ids(list: &Profiles) -> Vec<&str> {
    list.entries().iter().map(|p| p.id().as_str()).collect()
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx.debug_bounds(selector).unwrap();
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
}

/// The page, read and answered with the shipped four.
fn listed(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    let (probe, cx) = open(cx);
    let asked = asks(&probe, cx);
    assert_eq!(asked.len(), 1);
    assert!(asked[0].save.is_none());
    answer(&probe, &asked[0], Ok(shipped()), cx);
    (probe, cx)
}

#[gpui::test]
fn the_page_reads_its_machine_once_and_lists_the_rows(cx: &mut TestAppContext) {
    let (probe, cx) = listed(cx);
    for id in ["claude", "codex", "opencode", "pi"] {
        assert!(
            cx.debug_bounds(format!("terminal-profile-row-{id}").leak())
                .is_some(),
            "{id}"
        );
    }
    probe.update(cx, |probe, cx| probe.0.update(cx, |_, cx| cx.notify()));
    cx.run_until_parked();
    assert!(asks(&probe, cx).is_empty(), "read once while shown");
}

#[gpui::test]
fn a_move_sends_the_whole_list_and_the_buttons_wait_for_it(cx: &mut TestAppContext) {
    let (probe, cx) = listed(cx);
    click(cx, "terminal-profile-move-down-claude");
    let asked = asks(&probe, cx);
    assert_eq!(asked.len(), 1);
    let saved = asked[0].save.clone().unwrap();
    assert_eq!(ids(&saved), ["codex", "claude", "opencode", "pi"]);
    // While it waits, a second press sends nothing.
    click(cx, "terminal-profile-move-down-codex");
    assert!(asks(&probe, cx).is_empty());
    answer(&probe, &asked[0], Ok(saved.clone()), cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(shell.terminal_profiles_page_list(), Some(&saved));
    });
}

#[gpui::test]
fn add_says_what_is_missing_then_saves_a_new_row_last_and_closes(cx: &mut TestAppContext) {
    let (probe, cx) = listed(cx);
    click(cx, "terminal-profiles-add");
    click(cx, "terminal-profile-save");
    probe.read_with(cx, |probe, cx| {
        let edit = probe
            .0
            .read(cx)
            .areas
            .terminal_profiles
            .edit
            .as_ref()
            .unwrap();
        assert!(edit.errors.name && edit.errors.command);
    });
    assert!(asks(&probe, cx).is_empty());
    // Enter moves from Name to Command to Arguments, and saves from there.
    cx.simulate_input("Login shell");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("/bin/zsh");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("--login   -i");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let asked = asks(&probe, cx);
    assert_eq!(asked.len(), 1);
    let saved = asked[0].save.clone().unwrap();
    let added = saved.entries().last().unwrap();
    assert_eq!(added.name().as_str(), "Login shell");
    assert_eq!(
        vessel_app_view::terminal_profiles::command_line(added),
        "/bin/zsh --login -i"
    );
    assert!(added.id().as_str().starts_with("profile_"));
    probe.read_with(cx, |probe, cx| {
        assert!(
            probe
                .0
                .read(cx)
                .areas
                .terminal_profiles
                .edit
                .as_ref()
                .unwrap()
                .saving()
        );
    });
    answer(&probe, &asked[0], Ok(saved), cx);
    probe.read_with(cx, |probe, cx| {
        assert!(probe.0.read(cx).areas.terminal_profiles.edit.is_none());
    });
}

#[gpui::test]
fn a_refused_save_stays_in_the_dialog_with_the_draft(cx: &mut TestAppContext) {
    let (probe, cx) = listed(cx);
    click(cx, "terminal-profile-edit-pi");
    click(cx, "terminal-profile-save");
    let asked = asks(&probe, cx);
    assert_eq!(asked.len(), 1);
    let refused = Problem::Refused(ErrorCode::Forbidden);
    answer(&probe, &asked[0], Err(refused), cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let edit = shell.areas.terminal_profiles.edit.as_ref().unwrap();
        assert!(!edit.saving());
        assert_eq!(edit.refused, Some(refused));
        assert_eq!(edit.name.text(cx), "Pi");
        assert!(shell.areas.terminal_profiles.refused.is_none());
    });
}

#[gpui::test]
fn a_refused_removal_is_said_over_the_list_which_stays(cx: &mut TestAppContext) {
    let (probe, cx) = listed(cx);
    click(cx, "terminal-profile-remove-pi");
    click(cx, "terminal-profile-remove-confirm");
    let asked = asks(&probe, cx);
    assert_eq!(asked.len(), 1);
    assert_eq!(
        ids(asked[0].save.as_ref().unwrap()),
        ["claude", "codex", "opencode"]
    );
    let refused = Problem::Refused(ErrorCode::Unavailable);
    answer(&probe, &asked[0], Err(refused), cx);
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(shell.areas.terminal_profiles.refused, Some(refused));
        assert_eq!(shell.terminal_profiles_page_list(), Some(&shipped()));
        assert!(shell.areas.terminal_profiles.removing.is_none());
    });
}

#[gpui::test]
fn a_machine_not_connected_is_not_asked(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let mut commands = None;
        let shell = cx.new(|cx| {
            let (mut shell, sent) = language_tests::with_commands(window, cx);
            shell.route = Route::Settings;
            shell.areas.settings.section = Section::Terminals;
            let mut machine = laptop();
            machine.reach = Reach::Unreachable("down".to_owned());
            shell.fleet.machines = vec![machine];
            commands = Some(sent);
            shell
        });
        Probe(shell, commands.unwrap())
    });
    cx.run_until_parked();
    assert!(asks(&probe, cx).is_empty());
    assert!(cx.debug_bounds("terminal-profiles-add").is_none());
}
