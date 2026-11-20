//! The terminal find bar as Paseo draws it (`pane-find/index.tsx` and its
//! styles, placed by `terminal/find/index.tsx`): 340 wide over the pane's
//! top right (or bottom right when it would cover the current match), a
//! field with its counter inside, previous, next and close. Its state and
//! what its keys and buttons do are `find`'s.

use gpui::{AnyElement, Context, KeyDownEvent, Window};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk};
use vessel_app_kit::ui::{
    ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{ButtonState, ControlSizing, Tokens, Type, TypeExt as _, popup};

use super::PaneKey;
use super::find::Count;
use crate::shell::Shell;

/// Paseo's `FIND_WIDGET_WIDTH`.
const WIDTH: f32 = 340.0;
/// Paseo's `COLLAPSED_FIELD_HEIGHT`.
const FIELD_HEIGHT: f32 = 28.0;
/// The overlay's inset (`frame: { padding: spacing[2] }`).
const INSET: f32 = spacing::S2;
/// The widget's padding (`spacing[1.5]`) and gap (`spacing[1]`).
const PADDING: f32 = 6.0;
const GAP: f32 = spacing::S1;

/// The rows and columns the bar covers from the pane's top right, for
/// `cell` sized cells: the frame's inset, the widget's padding and border
/// and the field.
#[expect(
    clippy::float_arithmetic,
    reason = "the bar's size in pixels, from its constants"
)]
pub(super) fn covered(cell: vessel_app_kit::terminal_grid::CellSize) -> (u16, u16) {
    let height = INSET + PADDING * 2.0 + 2.0 + FIELD_HEIGHT;
    let width = INSET + WIDTH;
    (
        cells(height, f32::from(cell.height)),
        cells(width, f32::from(cell.width)),
    )
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    reason = "a length in cells, rounded up and clamped to 0 ..= u16::MAX before the cast"
)]
fn cells(length: f32, cell: f32) -> u16 {
    if cell <= 0.0 {
        return 0;
    }
    (length / cell).ceil().clamp(0.0, f32::from(u16::MAX)) as u16
}

/// `key`'s bar, when it is open.
pub(super) fn bar(
    shell: &Shell,
    key: &PaneKey,
    tokens: &Tokens,
    _window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let pane = shell.areas.terminal_tabs.panes.get(key)?;
    let find = pane.find.as_ref()?;
    let words = shell.words.terminal_tabs();
    let surfaces = popup::Surfaces::of(tokens);
    let field = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(spacing::S2))
        .h(px(FIELD_HEIGHT))
        .bg(surfaces.surface2)
        .rounded(px(radius::MD))
        .child(div().flex_1().min_w_0().child(find.field.input(tokens, cx)))
        .child(
            div()
                .id(element("terminal-find-count", key))
                .role(gpui::Role::Status)
                .aria_label(words.find_matches().to_owned())
                .flex_none()
                .type_(Type::Meta)
                .text_color(surfaces.foreground_muted)
                .child(counter(shell, find)),
        );
    let keys = key.clone();
    let widget = div()
        .w(px(WIDTH))
        .max_w_full()
        .flex()
        .flex_row()
        .items_center()
        .p(px(PADDING))
        .gap(px(GAP))
        .bg(surfaces.surface1)
        .border_1()
        .border_color(tokens.neutral.border)
        .rounded(px(radius::R2))
        .shadow(popup::shadow_md(tokens))
        // A press on the bar is the bar's: it must not reach the surface
        // below, which would take the keyboard and start a selection.
        .occlude()
        .on_key_down(cx.listener(move |shell, event: &KeyDownEvent, window, cx| {
            bar_key(shell, &keys, event, window, cx);
        }))
        .child(field)
        .children(buttons(shell, key, find.can_move(), tokens, cx));
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .when(find.bottom, gpui::Styled::bottom_0)
            .when(!find.bottom, gpui::Styled::top_0)
            .flex()
            .flex_row()
            .justify_end()
            .p(px(INSET))
            .child(widget)
            .into_any_element(),
    )
}

/// What the counter says (Paseo's `terminal/find/index.tsx` 100-106).
fn counter(shell: &Shell, find: &super::find::PaneFind) -> String {
    let words = shell.words.terminal_tabs();
    match find.count() {
        Count::Nothing => String::new(),
        Count::None => words.find_none().to_owned(),
        Count::Total(total) => words.find_total(&total).unwrap_or(total),
        Count::At(current, total) => words
            .find_position(&current, &total)
            .unwrap_or_else(|_| format!("{current} / {total}")),
    }
}

/// The bar's keys: Escape closes, Shift+Enter goes back (Enter is the
/// field's own submit), Mod+F selects the field again.
fn bar_key(
    shell: &mut Shell,
    key: &PaneKey,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let stroke = &event.keystroke;
    match stroke.key.as_str() {
        "escape" => {
            shell.terminal_find_close(key, window, cx);
            cx.stop_propagation();
        }
        "enter" if stroke.modifiers.shift => {
            shell.terminal_find_move(key, false, cx);
            cx.stop_propagation();
        }
        "f" if stroke.modifiers.secondary(cx)
            && !stroke.modifiers.shift
            && !stroke.modifiers.alt =>
        {
            shell.terminal_find_open(key, window, cx);
            cx.stop_propagation();
        }
        _ => {}
    }
}

/// Previous and next (only while there are matches), and close.
fn buttons(
    shell: &Shell,
    key: &PaneKey,
    can_move: bool,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> [ToolbarButton; 3] {
    let words = shell.words.terminal_tabs();
    let state = if can_move {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let config = ToolbarConfig::new(ControlSizing::Pointer, state);
    let close_config = ToolbarConfig::new(ControlSizing::Pointer, ButtonState::Enabled);
    let (previous, next, close) = (key.clone(), key.clone(), key.clone());
    [
        ToolbarButton::new(
            element("terminal-find-previous", key),
            words.find_previous().to_owned(),
            ToolbarIcon::new(Glyph::ArrowUp, ToolbarInk::Muted),
            config,
            tokens,
        )
        .on_click(listen(cx, move |shell, _window, cx| {
            shell.terminal_find_move(&previous, false, cx);
        })),
        ToolbarButton::new(
            element("terminal-find-next", key),
            words.find_next().to_owned(),
            ToolbarIcon::new(Glyph::ArrowDown, ToolbarInk::Muted),
            config,
            tokens,
        )
        .on_click(listen(cx, move |shell, _window, cx| {
            shell.terminal_find_move(&next, true, cx);
        })),
        ToolbarButton::new(
            element("terminal-find-close", key),
            words.find_close().to_owned(),
            ToolbarIcon::new(Glyph::Close, ToolbarInk::Muted),
            close_config,
            tokens,
        )
        .on_click(listen(cx, move |shell, window, cx| {
            shell.terminal_find_close(&close, window, cx);
        })),
    ]
}

/// A button's click as a shell call.
fn listen(
    cx: &Context<'_, Shell>,
    act: impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) + 'static,
) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static {
    let shell = cx.weak_entity();
    move |_event, window, cx| {
        drop(shell.update(cx, |shell, cx| act(shell, window, cx)));
    }
}

/// An element id naming `part` of `key`'s bar.
fn element(part: &str, key: &PaneKey) -> ElementId {
    ElementId::Name(format!("{part}-{}-{}", key.machine, key.terminal).into())
}
