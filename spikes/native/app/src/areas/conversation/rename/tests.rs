//! The one rename dialog (TRACKER 800, Paseo's `AdaptiveRenameModal`): the
//! title prefilled, "Name is required" for none, the rename held while it
//! waits for the node, a refusal kept under the field with the typed title,
//! and success closing it with the session (and its tab) renamed.
use super::*;
use crate::areas::session::Open;
use crate::routes::Route as Page;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{Entity, IntoElement};
use vessel_app_link::ActFailed;
use vessel_proto::{ErrorCode, HarnessKind, SessionInfo, SessionMode, SessionSpec};

struct Probe(Entity<Shell>, SessionId);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        self.0.update(cx, |shell, cx| {
            div().children(rename_dialog(shell, &tokens, window, cx))
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
        Probe(
            cx.new(|cx| {
                let mut shell = Shell::without_io(window, cx);
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

#[gpui::test]
fn the_field_stops_at_the_nodes_longest_title(cx: &mut TestAppContext) {
    let (probe, cx) = shown(cx);
    update(&probe, cx, Shell::open_rename);
    cx.run_until_parked();
    // The prefilled title is selected: typing replaces it, up to the limit.
    let longest = "x".repeat(SessionTitle::MAX_CHARS);
    cx.simulate_input(&longest);
    cx.simulate_input("y");
    assert_eq!(
        state(&probe, cx).map(|(text, ..)| text.chars().count()),
        Some(SessionTitle::MAX_CHARS)
    );
}
