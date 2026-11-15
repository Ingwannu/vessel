//! The start card's header (Paseo's setup command row: the state's icon in
//! an 18 box, the words in the foreground, the project muted at the right,
//! Stop while it works, the chevron that folds the log).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button, glyph};
use vessel_app_view::workspace::start::Step;
use vessel_proto::workspace::SetupState;

use super::start_card::Starting;
use crate::shell::Shell;

/// The header's words, its tone and whether the start is still working.
fn headline(shell: &Shell, start: &Starting) -> (String, Tone) {
    let words = shell.words();
    let labels = words.labels();
    match &start.step {
        Step::Idle | Step::Making | Step::Output(_) => {
            (labels.making_worktree.to_owned(), Tone::Working)
        }
        Step::Approval { .. } => (labels.waiting_approval.to_owned(), Tone::Asking),
        Step::Refused(refusal) => (words.start_refusal(refusal), Tone::Failed),
        Step::Setup {
            branch,
            setup,
            commands,
        } => {
            let branch = branch.as_deref().unwrap_or(labels.detached);
            match setup {
                SetupState::Running { .. } => (
                    format!("{} \u{b7} {branch}", labels.setting_up),
                    Tone::Working,
                ),
                SetupState::Failed { failure, .. } => {
                    let commands: Vec<&str> = commands.iter().map(String::as_str).collect();
                    (
                        format!(
                            "{} \u{b7} {}",
                            labels.setup_failed,
                            words.setup_failure(failure, &commands)
                        ),
                        Tone::Failed,
                    )
                }
                SetupState::Skipped => (
                    format!(
                        "{} \u{b7} {}",
                        labels.setup_skipped, labels.starting_session
                    ),
                    Tone::Done,
                ),
                SetupState::Unknown => (
                    shell.words.scripts().setup_unknown().to_owned(),
                    Tone::Failed,
                ),
                SetupState::None | SetupState::Succeeded { .. } => (
                    format!(
                        "{} \u{b7} {}",
                        labels.setup_finished, labels.starting_session
                    ),
                    Tone::Done,
                ),
            }
        }
    }
}

/// The state a header shows.
#[derive(Clone, Copy)]
enum Tone {
    Working,
    Asking,
    Done,
    Failed,
}

pub(super) fn header(
    shell: &Shell,
    start: &Starting,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let (text, tone) = headline(shell, start);
    let icon: AnyElement = match tone {
        // Paseo's `LoadingSpinner` at 14 in the foreground.
        Tone::Working => vessel_app_kit::slider::spinner("start-spinner", 14.0, neutral.foreground),
        Tone::Asking => glyph(Glyph::Shield, 14.0, neutral.foreground_muted).into_any_element(),
        Tone::Done => glyph(Glyph::Check, 14.0, tokens.st_ready).into_any_element(),
        Tone::Failed => glyph(Glyph::Alert, 14.0, neutral.destructive).into_any_element(),
    };
    let foldable = matches!(&start.step, Step::Setup { setup, .. }
        if matches!(setup, SetupState::Running { .. } | SetupState::Failed { .. }));
    let working = matches!(tone, Tone::Working);
    div()
        .id("start-header")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .bg(neutral.surface1)
        .when(foldable, |row| {
            row.cursor_pointer()
                .on_click(cx.listener(|this, _event, _window, cx| {
                    if let Some(start) = this.areas.workspace.start.as_mut() {
                        start.folded = !start.folded;
                    }
                    cx.notify();
                }))
        })
        .child(
            div()
                .size(px(18.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icon),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(text),
        )
        .child(
            div()
                .flex_none()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(start.project.clone()),
        )
        .when(working, |row| {
            row.child(
                button(
                    "start-stop",
                    shell.words.scripts().cancel_creation(),
                    false,
                    tokens,
                )
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.stop_start(cx);
                })),
            )
        })
        .when(foldable, |row| {
            row.child(
                glyph(
                    if start.folded {
                        Glyph::ChevronRight
                    } else {
                        Glyph::ChevronDown
                    },
                    14.0,
                    neutral.foreground_muted,
                )
                .flex_none(),
            )
        })
}
