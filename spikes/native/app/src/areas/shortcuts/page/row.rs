//! One canonical binding row uses the engine's keys and capture projection.
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    div, px, svg,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind};
use vessel_app_view::prefs::ActionId;
use vessel_app_view::shortcuts::definitions::Definition;
use vessel_app_view::shortcuts::engine::EffectiveBinding;
use vessel_app_view::shortcuts::sequence_capture::{CaptureFault, CaptureVerdict, SequenceCapture};
use vessel_app_view::shortcuts::{Action, Chord, Platform, action_label, chord_caps};
const WHY_MOST: f32 = 260.0;
const SLOT: f32 = 32.0;

pub(super) fn row(
    shell: &Shell,
    definition: &Definition<Action>,
    bindings: &[EffectiveBinding<Action>],
    first: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let words = shell.words.shortcuts();
    let capture = shell
        .areas
        .shortcuts
        .page
        .capture
        .as_ref()
        .filter(|capture| capture.binding == definition.id);
    let label = div()
        .flex_1()
        .min_w_0()
        .type_(Type::Body)
        .text_color(neutral.foreground)
        .child(action_label(definition.action, shell.words));
    let right = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .justify_end()
        .max_w(px(WHY_MOST))
        .items_center()
        .gap(px(spacing::S2));
    let right = if let Some(capture) = capture {
        let verdict = super::editing::verdict(shell, capture);
        let can_save = matches!(verdict, CaptureVerdict::Ready { .. });
        right
            .child(capturing_keys(shell, capture, &verdict, tokens))
            .child(
                button_with_kind(
                    "shortcut-done",
                    words.done(),
                    ButtonKind::Ghost,
                    if can_save {
                        ButtonState::Enabled
                    } else {
                        ButtonState::Disabled
                    },
                    tokens,
                )
                .when(can_save, |button| {
                    button.on_click(
                        cx.listener(|shell, _event, window, cx| shell.shortcut_done(window, cx)),
                    )
                }),
            )
            .child(
                button_with_kind(
                    "shortcut-cancel",
                    words.cancel(),
                    ButtonKind::Ghost,
                    ButtonState::Enabled,
                    tokens,
                )
                .on_click(cx.listener(|shell, _event, _window, cx| shell.shortcut_cancel(cx))),
            )
    } else {
        right
            .child(keys(shell, definition, bindings, tokens))
            .child(menu_button(&definition.id, tokens, cx))
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
fn menu_button(id: &ActionId, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let id = id.clone();
    let neutral = tokens.neutral;
    div().size(px(SLOT)).flex_none().child(
        div()
            .id(SharedString::from(format!("shortcut-menu-{id}")))
            .debug_selector({
                let id = id.clone();
                move || format!("shortcut-menu-{id}")
            })
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
                    shell.shortcut_menu(id.clone(), event.position, window, cx);
                }),
            ),
    )
}

fn keys(
    shell: &Shell,
    definition: &Definition<Action>,
    bindings: &[EffectiveBinding<Action>],
    tokens: &Tokens,
) -> Div {
    let bindings: Vec<_> = bindings
        .iter()
        .filter(|binding| binding.id == definition.id)
        .collect();
    if bindings.is_empty() {
        return div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(shell.words.shortcuts().unassigned());
    }
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_end()
        .gap(px(spacing::S2))
        .children(bindings.iter().enumerate().flat_map(|(at, binding)| {
            let or = (at > 0).then(|| {
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .child(shell.words.shortcuts().or())
                    .into_any_element()
            });
            or.into_iter().chain(std::iter::once(
                wrapped_caps(
                    vessel_app_view::shortcuts::help::display_caps(
                        definition.action,
                        &binding.keys,
                        vessel_app_view::shortcuts::modified(
                            definition.action,
                            &shell.prefs.settings.shortcuts,
                        ),
                        Platform::native(),
                    )
                    .into_iter()
                    .map(|caps| {
                        SharedString::from(caps.join(if Platform::native() == Platform::Mac {
                            ""
                        } else {
                            "+"
                        }))
                    })
                    .collect::<Vec<_>>(),
                    tokens,
                )
                .into_any_element(),
            ))
        }))
}
fn caps<'a>(chords: impl Iterator<Item = &'a Chord>, tokens: &Tokens) -> Div {
    let joiner = if Platform::native() == Platform::Mac {
        ""
    } else {
        "+"
    };
    let caps: Vec<SharedString> = chords
        .map(|chord| chord_caps(chord, Platform::native()).join(joiner).into())
        .collect();
    wrapped_caps(caps, tokens)
}

// Each chord stays intact, while a 16-step user sequence can wrap inside the
// Settings row. The shared badge renderer remains the only cap renderer.
fn wrapped_caps(caps: Vec<SharedString>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_wrap()
        .min_w_0()
        .max_w(px(WHY_MOST))
        .justify_end()
        .gap(px(spacing::S1))
        .children(
            caps.into_iter()
                .map(|cap| vessel_app_kit::command::key_badges(&[cap], tokens)),
        )
}
fn capturing_keys(
    shell: &Shell,
    capture: &SequenceCapture,
    verdict: &CaptureVerdict,
    tokens: &Tokens,
) -> Div {
    let words = shell.words.shortcuts();
    let shown = if capture.chords().next().is_some() {
        caps(capture.chords(), tokens)
    } else {
        div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(words.capture())
    };
    let why = match verdict {
        CaptureVerdict::IndexRequired => Some(words.index_required()),
        CaptureVerdict::Refused(CaptureFault::Editing) => Some(words.editing()),
        CaptureVerdict::Refused(CaptureFault::HostReserved) => Some(words.kept()),
        CaptureVerdict::Refused(CaptureFault::Sequence(_)) => Some(words.too_long()),
        CaptureVerdict::Waiting | CaptureVerdict::Ready { .. } => None,
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
                .text_color(tokens.neutral.destructive)
                .child(why)
        }))
}
