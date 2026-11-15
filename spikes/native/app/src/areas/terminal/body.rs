//! The terminal screen in the session area's body: a line while its log
//! is read, else the grid on the terminal's own background, edge to edge
//! (Paseo's host has no padding: `components/terminal-emulator.tsx`
//! 52–64), with a line over it when the earlier output could not be read.

use vessel_app_kit::picker::anchor_probe;
use vessel_app_kit::terminal_grid::cell_size;
use vessel_app_kit::theme::spacing::S6;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

use super::{Phase, convert, font_size};
use crate::shell::Shell;

/// The body of a terminal session's screen.
pub(crate) fn body(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.terminal;
    let t = shell.t;
    let background = convert::hsla(convert::palette(&tokens.terminal).background);
    // The cells that fit are taken from this frame's box once it is drawn
    // (the probe paints it), so on the next frame: a deferred call would
    // still see the last frame's box, a window resized one step behind.
    let cell = cell_size(window, font_size());
    cx.on_next_frame(window, move |shell, _window, cx| {
        shell.terminal_measured(cell, cx);
    });
    let unread = (area.phase == Phase::Unread).then(|| {
        vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(t.terminal_unread.into()),
                description: None,
                action: None,
            },
            vessel_app_kit::alert::Variant::Error,
            vessel_app_kit::alert::Size::Small,
            tokens,
        )
        .flex_none()
    });
    // While the log is read the box is measured too, so the history is
    // replayed at the width it will be drawn at.
    let screen = if area.phase == Phase::Loading {
        div()
            .p(px(S6))
            .type_(Type::Body)
            .text_color(tokens.text_secondary)
            .child(t.terminal_loading)
            .into_any_element()
    } else {
        area.grid.clone().into_any_element()
    };
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(background)
        .children(unread)
        .child(
            div()
                .id("terminal-viewport")
                .track_focus(&area.focus)
                .flex_1()
                .min_h_0()
                .relative()
                .overflow_hidden()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|shell, _, window, cx| {
                        shell.focus_terminal(window, cx);
                    }),
                )
                .on_scroll(
                    cx.listener(move |shell, event, _, cx| shell.terminal_scroll(event, cell, cx)),
                )
                .child(screen)
                .child(anchor_probe(&area.probe)),
        )
        .into_any_element()
}
