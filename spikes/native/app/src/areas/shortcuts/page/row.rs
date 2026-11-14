//! One row of a section (Paseo's `ShortcutRow`).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    div, px, svg,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::shortcuts::{
    Action, Capture, Chord, Platform, Verdict, binding_words, bindings_of, chord_caps, jump_shown,
};

use crate::shell::Shell;

/// The widest a refused key's reason is drawn beside the keys, points.
const WHY_MOST: f32 = 260.0;

/// Paseo's `menuSlot` and `menuButton`: 32 by 32.
const SLOT: f32 = 32.0;

pub(super) fn row(
    shell: &Shell,
    action: Action,
    first: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let words = shell.t.lang.shortcuts();
    let capture = shell
        .areas
        .shortcuts
        .page
        .capture
        .as_ref()
        .filter(|capture| capture.action == action);
    let label = div()
        .flex_1()
        .min_w_0()
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .child(words.label(action));
    let right = div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(spacing::S2));
    let right = match capture {
        Some(capture) => {
            let platform = Platform::native();
            let verdict = capture.verdict(&shell.prefs.settings.shortcuts, platform);
            let can_save = matches!(verdict, Verdict::Ready | Verdict::Reassign(_));
            right
                .child(capturing_keys(shell, capture, verdict, tokens))
                .when(capture.chord.is_some(), |right| {
                    right.child(
                        button_with_kind(
                            "shortcut-done",
                            words.done,
                            ButtonKind::Ghost,
                            if can_save {
                                ButtonState::Enabled
                            } else {
                                ButtonState::Disabled
                            },
                            tokens,
                        )
                        .when(can_save, |button| {
                            button.on_click(cx.listener(|shell, _event, window, cx| {
                                shell.shortcut_done(window, cx);
                            }))
                        }),
                    )
                })
                .child(
                    button_with_kind(
                        "shortcut-cancel",
                        words.cancel,
                        ButtonKind::Ghost,
                        ButtonState::Enabled,
                        tokens,
                    )
                    .on_click(cx.listener(|shell, _event, _window, cx| {
                        shell.shortcut_cancel(cx);
                    })),
                )
        }
        None => right
            .child(keys(shell, action, tokens))
            .child(menu_slot(action, tokens, cx)),
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .p(px(spacing::S3))
        .when(!first, |row| row.border_t_1().border_color(neutral.border))
        .when(capture.is_some(), |row| row.bg(neutral.surface2))
        .child(label)
        .child(right)
}

/// The action's keys: one group of badges per binding (Paseo's
/// `Shortcut`), or "Not set".
fn keys(shell: &Shell, action: Action, tokens: &Tokens) -> Div {
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    let caps = |chord: &Chord| -> SharedString { chord_caps(chord, platform).join(joiner).into() };
    let bindings: Vec<Vec<SharedString>> = if action.fixed() {
        vec![vec![caps(&jump_shown())]]
    } else {
        bindings_of(action, &shell.prefs.settings.shortcuts)
            .iter()
            .map(|binding| binding.chords().map(caps).collect())
            .collect()
    };
    if bindings.is_empty() {
        return div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(shell.t.lang.shortcuts().unassigned);
    }
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .children(bindings.iter().enumerate().flat_map(|(at, chords)| {
            // Two keys read as two: G M, or G F (not G M G F).
            let or = (at > 0).then(|| {
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(shell.t.lang.shortcuts().or)
                    .into_any_element()
            });
            or.into_iter().chain(std::iter::once(
                vessel_app_kit::command::key_badges(chords, tokens).into_any_element(),
            ))
        }))
}

/// While recording: the key pressed, or the prompt; under it why the
/// key cannot be used, when it cannot.
fn capturing_keys(shell: &Shell, capture: &Capture, verdict: Verdict, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let words = shell.t.lang.shortcuts();
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    let shown = capture.chord.as_ref().map_or_else(
        || {
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.capture_prompt)
        },
        |chord| {
            let caps: SharedString = chord_caps(chord, platform).join(joiner).into();
            vessel_app_kit::command::key_badges(&[caps], tokens)
        },
    );
    let why = match verdict {
        Verdict::Refused(refused) => Some(words.refusal(refused).to_owned()),
        Verdict::Clashes(clash) => Some(
            words.clash(
                clash,
                &capture
                    .chord
                    .as_ref()
                    .map(|chord| binding_words(&super::chord_binding(chord), platform, words.then))
                    .unwrap_or_default(),
                capture.action,
            ),
        ),
        Verdict::Waiting | Verdict::Ready | Verdict::Reassign(_) => None,
    };
    div()
        .flex()
        .flex_col()
        .items_end()
        .gap(px(spacing::S1))
        .child(shown)
        .children(why.map(|why| {
            div()
                .max_w(px(WHY_MOST))
                .type_(Type::Meta)
                .text_color(neutral.destructive)
                .child(why)
        }))
}

/// Paseo's fixed menu slot: the "…" button, or empty for a fixed row
/// so every row's keys keep one rail.
fn menu_slot(action: Action, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let neutral = tokens.neutral;
    let slot = div().size(px(SLOT)).flex_none();
    if action.fixed() {
        return slot;
    }
    slot.child(
        div()
            .id(SharedString::from(format!("shortcut-menu-{}", action.id())))
            .size(px(SLOT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius::R2))
            .cursor_pointer()
            .hover(|button| button.bg(neutral.surface2))
            .active(|button| button.bg(neutral.surface3))
            .child(
                svg()
                    .path(Glyph::More.path())
                    .size(px(14.0))
                    .text_color(neutral.foreground_muted),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                    shell.shortcut_menu(action, event.position, window, cx);
                }),
            ),
    )
}
