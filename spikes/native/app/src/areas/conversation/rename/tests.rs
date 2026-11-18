//! The one rename dialog (TRACKER 800, Paseo's `AdaptiveRenameModal`): the
//! title prefilled, "Name is required" for none, the rename held while it
//! waits for the node, a refusal kept under the field with the typed title,
//! and success closing it with the session (and its tab) renamed.
use super::*;
use crate::areas::session::Open;
use crate::routes::Route as Page;
use crate::shell::language_tests;
use gpui::{AppContext as _, FocusHandle, Render, TestAppContext, VisualTestContext};
use tokio::sync::mpsc::Receiver;
use vessel_app_kit::ui::{Entity, InteractiveElement as _, IntoElement};
use vessel_app_link::{ActFailed, Command, Link};
use vessel_proto::{ErrorCode, HarnessKind, SessionInfo, SessionMode, SessionSpec};

/// The shell, its session, two openers that can hold the keys, and what
/// the shell asked the link.
struct Probe(
    Entity<Shell>,
    SessionId,
    [FocusHandle; 2],
    Receiver<Command>,
);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        let openers = self.2.clone();
        self.0.update(cx, |shell, cx| {
            div()
                .children(openers.iter().map(|opener| div().track_focus(opener)))
                .children(rename_dialog(shell, &tokens, window, cx))
        })
    }
}

/// The session as the node describes it, live, under `title`.
fn info(session: SessionId, title: &str) -> SessionInfo {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(session, spec, chrono::DateTime::UNIX_EPOCH);
    info.title = Some(title.parse().unwrap());
    info
}

/// A shell showing a live session "Fix the cart" on the laptop.
fn shown(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let session = SessionId::random();
        let (link, commands) = Link::detached(16).unwrap();
        let openers = [cx.focus_handle(), cx.focus_handle()];
        Probe(
            cx.new(|cx| {
                let mut shell = Shell::with_test_link(window, cx, link);
                shell.route = Page::Session;
                shell.areas.session.open = Some(Open {
                    machine: "laptop".to_owned(),
                    session,
                    title: "Fix the cart".to_owned(),
                    agent: "Claude Code".to_owned(),
                    structured: true,
                    live: true,
                    archived_at: None,
                    binding: None,
                });
                shell.areas.conversation.open("laptop", session);
                shell.areas.conversation.info = Some(Box::new(info(session, "Fix the cart")));
                shell
            }),
            session,
            openers,
            commands,
        )
    });
    cx.update(|window, _| window.activate_window());
    (probe, cx)
}

fn update<R>(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Shell, &mut Window, &mut Context<'_, Shell>) -> R,
) -> R {
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| f(shell, window, cx))
    })
}

fn typed(probe: &Entity<Probe>, cx: &mut VisualTestContext, text: &'static str) {
    update(probe, cx, |shell, window, cx| {
        if let Some(rename) = shell.areas.conversation.rename.as_ref() {
            rename.field.set_text(text, window, cx);
        }
    });
}

/// The text, whether Rename may be pressed, what shows under the field,
/// and whether it waits.
type Seen = (String, bool, Option<String>, bool);

fn state(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> Option<Seen> {
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        shell.areas.conversation.rename.as_ref().map(|rename| {
            let text = rename.field.text(cx);
            let save = savable(rename, &text);
            (text, save, rename.error.clone(), rename.pending)
        })
    })
}

fn field_focused(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> bool {
    update(probe, cx, |shell, window, cx| {
        shell
            .areas
            .conversation
            .rename
            .as_ref()
            .is_some_and(|rename| rename.field.focus_handle(cx).is_focused(window))
    })
}

fn answer(probe: &Entity<Probe>, cx: &mut VisualTestContext, result: Result<Acted, ActFailed>) {
    let session = probe.read_with(cx, |probe, _| probe.1);
    update(probe, cx, |shell, _window, cx| {
        shell.acted(
            ActAnswer {
                terminal: None,
                machine: "laptop".to_owned(),
                session,
                kind: ActKind::Rename,
                result,
            },
            cx,
        );
    });
}

#[gpui::test]
fn the_dialog_opens_prefilled_and_asks_for_a_name(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    update(&probe, cx, Shell::open_rename);
    cx.run_until_parked();
    assert_eq!(
        state(&probe, cx),
        Some(("Fix the cart".to_owned(), false, None, false)),
        "prefilled; Rename waits for another title"
    );
    // None at all: Paseo's "Name is required", and the dialog stays.
    typed(&probe, cx, "  ");
    update(&probe, cx, Shell::rename_pressed);
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    assert_eq!(
        state(&probe, cx).and_then(|(_, _, error, _)| error),
        Some(words.name_required().to_owned())
    );
    // Typing (a real edit: a programmatic text sends no change) clears it.
    cx.simulate_input("Fix the basket");
    assert_eq!(
        state(&probe, cx).map(|(_, save, error, _)| (save, error)),
        Some((true, None)),
        "typing clears the reason"
    );
    update(&probe, cx, Shell::cancel_rename);
    assert_eq!(state(&probe, cx), None);
}

#[gpui::test]
fn a_rename_waits_for_the_node_and_keeps_the_title_on_a_refusal(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    update(&probe, cx, Shell::open_rename);
    typed(&probe, cx, "Fix the basket");
    update(&probe, cx, Shell::rename_pressed);
    cx.run_until_parked();
    // Waiting: Rename held, the field read-only, Cancel and Escape held.
    assert_eq!(
        state(&probe, cx),
        Some(("Fix the basket".to_owned(), false, None, true))
    );
    // Without its caret: the dialog holds the keys, not the field.
    assert!(!field_focused(&probe, cx), "no caret while it waits");
    cx.simulate_input("zz");
    update(&probe, cx, Shell::cancel_rename);
    assert_eq!(
        state(&probe, cx).map(|(text, ..)| text).as_deref(),
        Some("Fix the basket")
    );
    // Refused: the node's words under the field, the typed title kept,
    // not said again under the bar's title.
    answer(
        &probe,
        cx,
        Err(ActFailed {
            code: Some(ErrorCode::InvalidRequest),
            message: "That title is taken".to_owned(),
        }),
    );
    assert_eq!(
        state(&probe, cx),
        Some((
            "Fix the basket".to_owned(),
            true,
            Some("That title is taken".to_owned()),
            false
        ))
    );
    probe.read_with(cx, |probe, cx| {
        assert_eq!(probe.0.read(cx).areas.conversation.refused, None);
    });
    // The field takes the keys back at the next frame.
    probe.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert!(field_focused(&probe, cx), "the field has the keys again");
    // Again, and taken: the dialog closes and the open session (its tab's
    // label, `layout::tab_label`) has the new title.
    update(&probe, cx, Shell::rename_pressed);
    let session = probe.read_with(cx, |probe, _| probe.1);
    answer(
        &probe,
        cx,
        Ok(Acted::Session(Box::new(info(session, "Fix the basket")))),
    );
    assert_eq!(state(&probe, cx), None);
    probe.read_with(cx, |probe, cx| {
        let open = probe.0.read(cx).areas.session.open.as_ref().unwrap();
        assert_eq!(open.title, "Fix the basket");
    });
}

/// The field's length after typing one past `most` over the selection.
fn typed_past(probe: &Entity<Probe>, cx: &mut VisualTestContext, most: usize) -> Option<usize> {
    update(probe, cx, Shell::open_rename);
    cx.run_until_parked();
    // The prefilled title is selected: typing replaces it, up to the limit.
    cx.simulate_input(&"x".repeat(most));
    cx.simulate_input("y");
    state(probe, cx).map(|(text, ..)| text.chars().count())
}

/// `laptop`, connected at `protocol`.
fn connected(probe: &Entity<Probe>, cx: &mut VisualTestContext, protocol: u32) {
    update(probe, cx, |shell, _window, _cx| {
        shell.fleet.machines = vec![vessel_app_view::Machine {
            name: "laptop".to_owned(),
            reach: vessel_app_view::Reach::Online,
            sessions: Vec::new(),
            agents: Vec::new(),
            protocol: Some(vessel_proto::ProtocolVersion::new(protocol)),
            version: None,
            node: None,
            connection_epoch: None,
        }];
    });
}

#[gpui::test]
fn the_field_stops_at_paseos_200_on_a_node_with_long_titles(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    connected(&probe, cx, 30);
    let most = SessionTitle::MAX_CHARS;
    assert_eq!(typed_past(&probe, cx, most), Some(most));
}

#[gpui::test]
fn the_field_stops_at_120_on_an_older_node(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    connected(&probe, cx, 29);
    let most = SessionTitle::LEGACY_MAX_CHARS;
    assert_eq!(typed_past(&probe, cx, most), Some(most));
}

#[gpui::test]
fn the_field_stops_at_120_while_the_node_is_not_connected(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    let most = SessionTitle::LEGACY_MAX_CHARS;
    assert_eq!(typed_past(&probe, cx, most), Some(most));
}

/// The Rename acts the shell sent since the last look.
fn renames_sent(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> usize {
    probe.update(cx, |probe, _| {
        std::iter::from_fn(|| probe.3.try_recv().ok())
            .filter(|command| {
                matches!(
                    command,
                    Command::Act {
                        act: Act::Rename(_),
                        ..
                    }
                )
            })
            .count()
    })
}

fn opener_focused(probe: &Entity<Probe>, cx: &mut VisualTestContext, which: usize) -> bool {
    probe.update_in(cx, |probe, window, _| probe.2[which].is_focused(window))
}

fn focus_opener(probe: &Entity<Probe>, cx: &mut VisualTestContext, which: usize) {
    probe.update_in(cx, |probe, window, cx| probe.2[which].focus(window, cx));
    cx.run_until_parked();
}

/// A frame of the probe (where the dialog acknowledges a close).
fn frame(probe: &Entity<Probe>, cx: &mut VisualTestContext) {
    probe.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
}

#[gpui::test]
fn a_taken_rename_hands_the_keys_back_to_its_opener(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    focus_opener(&probe, cx, 0);
    update(&probe, cx, Shell::open_rename);
    frame(&probe, cx);
    typed(&probe, cx, "Fix the basket");
    update(&probe, cx, Shell::rename_pressed);
    frame(&probe, cx);
    assert_eq!(renames_sent(&probe, cx), 1);
    let session = probe.read_with(cx, |probe, _| probe.1);
    answer(
        &probe,
        cx,
        Ok(Acted::Session(Box::new(info(session, "Fix the basket")))),
    );
    frame(&probe, cx);
    assert_eq!(state(&probe, cx), None);
    assert!(
        opener_focused(&probe, cx, 0),
        "the opener has the keys again"
    );
    // The next opening remembers its own opener: Cancel goes back there.
    focus_opener(&probe, cx, 1);
    update(&probe, cx, Shell::open_rename);
    frame(&probe, cx);
    update(&probe, cx, Shell::cancel_rename);
    cx.run_until_parked();
    assert!(
        opener_focused(&probe, cx, 1),
        "Cancel returns to the new opener"
    );
}

#[gpui::test]
fn a_rename_is_not_sent_to_a_session_that_is_no_longer_shown(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    update(&probe, cx, Shell::open_rename);
    typed(&probe, cx, "Fix the basket");
    // The session went from the page while the dialog was open.
    update(&probe, cx, |shell, _, _| {
        shell.areas.conversation.shown = None;
    });
    update(&probe, cx, Shell::rename_pressed);
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    assert_eq!(
        renames_sent(&probe, cx),
        0,
        "nothing goes to another session"
    );
    assert_eq!(
        state(&probe, cx),
        Some((
            "Fix the basket".to_owned(),
            true,
            Some(words.unable_to_save().to_owned()),
            false
        )),
        "not waiting: Cancel works"
    );
}

#[gpui::test]
fn a_rename_stops_waiting_after_its_deadline(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    update(&probe, cx, Shell::open_rename);
    typed(&probe, cx, "Fix the basket");
    update(&probe, cx, Shell::rename_pressed);
    assert_eq!(state(&probe, cx).map(|(.., pending)| pending), Some(true));
    cx.executor()
        .advance_clock(RENAME_DEADLINE.saturating_add(std::time::Duration::from_secs(1)));
    cx.run_until_parked();
    let words = probe
        .read_with(cx, |probe, cx| probe.0.read(cx).words)
        .layout();
    assert_eq!(
        state(&probe, cx).map(|(_, save, error, pending)| (save, error, pending)),
        Some((true, Some(words.rename_timeout().to_owned()), false))
    );
}
