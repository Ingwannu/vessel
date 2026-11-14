//! One renderer for the static frame and its opt-in motion/gesture policy.
use super::{
    ACROSS, CommandFrame, CommandKey, CommandMotion, CommandState, HEIGHT, TOP, WIDTH, shadow_lg,
};
use crate::fonts::Type;
use crate::popup;
use crate::sheet::snap::listener::{OnRelease, listener};
use crate::sheet::snap::{Origin, Release, Stop};
use crate::theme::COMMAND_SCRIM;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Div, MouseButton, Pixels, ScrollHandle, Window, anchored, canvas,
    deferred, div, point, px, relative, rgba,
};
use gpui_base::input::{Enter, Escape, MoveDown, MoveUp};

/// The owner's key dispatch and focus-aware dismissal.
pub struct CommandHandlers<V: 'static> {
    /// Called before the search's own arrow/Enter/Escape actions.
    pub on_key: fn(&mut V, CommandKey, &mut Window, &mut Context<'_, V>),
    /// Called for a backdrop press or a pan to closed.
    pub close: fn(&mut V, &mut Window, &mut Context<'_, V>),
}
impl<V: 'static> Copy for CommandHandlers<V> {}
impl<V: 'static> Clone for CommandHandlers<V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: 'static> std::fmt::Debug for CommandHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandHandlers").finish_non_exhaustive()
    }
}
/// Persistent state plus the motion sampled before building the rows.
#[derive(Debug, Clone, Copy)]
pub struct CommandAnimation<'a> {
    /// The same state the owner presented and sampled.
    pub state: &'a CommandState,
    /// The list's actual scroll handle, for content-pan handoff.
    pub scroll: &'a ScrollHandle,
    /// The current geometry/opacity and lifecycle.
    pub motion: CommandMotion,
}
/// Compatibility entry point. New hosts use `command_frame_animated`.
pub fn command_frame<V: 'static>(
    frame: CommandFrame,
    on_key: fn(&mut V, CommandKey, &mut Window, &mut Context<'_, V>),
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    draw(
        frame,
        CommandHandlers { on_key, close },
        None,
        tokens,
        window,
        cx,
    )
}
/// Draw a sampled frame and schedule its next frame only while moving.
/// The host stops drawing after `motion.hidden` and restores focus at close.
pub fn command_frame_animated<V: 'static>(
    frame: CommandFrame,
    animation: CommandAnimation<'_>,
    handlers: CommandHandlers<V>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    if animation.motion.animating && !cx.reduce_motion() {
        window.request_animation_frame();
    }
    draw(frame, handlers, Some(&animation), tokens, window, cx)
}
fn draw<V: 'static>(
    frame: CommandFrame,
    handlers: CommandHandlers<V>,
    animation: Option<&CommandAnimation<'_>>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let closing = animation.is_some_and(|a| a.motion.closing);
    let release: OnRelease = std::rc::Rc::new(cx.listener(
        move |view: &mut V, release: &Release, window, cx| {
            if *release == Release::Pan(Stop::Closed) {
                (handlers.close)(view, window, cx);
            }
        },
    ));
    let mut panel = panel(handlers, closing, tokens, cx);
    panel = if frame.compact {
        panel
            .w_full()
            .map(|panel| match animation.and_then(|a| a.motion.sheet) {
                Some(geometry) => panel.h(geometry.height),
                None => panel.h(relative(0.6)),
            })
            .rounded_t(px(popup::SHEET_RADIUS))
            .child(handle(
                neutral.sheet_handle,
                window.viewport_size().width,
                animation,
                &release,
            ))
    } else {
        panel
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .max_w(relative(0.92))
            .max_h(relative(0.8))
            .border_1()
            .border_color(neutral.border)
            .rounded(px(popup::MENU_RADIUS))
            .shadow(shadow_lg(tokens))
    };
    let header = div()
        .flex_none()
        .px(px(ACROSS))
        .py(px(spacing::S3))
        .border_b_1()
        .border_color(neutral.border)
        .child(div().py(px(spacing::S1)).child(frame.search));
    let mut body = div()
        .relative()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(frame.list);
    if let Some(animation) = animation.filter(|_| frame.compact && !closing) {
        body = body.child(listener(
            &animation.state.sheet,
            Origin::Content,
            animation.scroll,
            &release,
        ));
    }
    let panel = panel.child(header).child(body);
    let size = window.viewport_size();
    let scrim = div()
        .id("command-scrim")
        .w(size.width)
        .h(size.height)
        .flex()
        .flex_col()
        .items_center()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                if !closing {
                    (handlers.close)(view, window, cx);
                }
                cx.notify();
            }),
        );
    let scrim = if frame.compact {
        scrim.justify_end().bg(popup::sheet_scrim().opacity(
            animation
                .as_ref()
                .and_then(|a| a.motion.sheet)
                .map_or(1.0, |g| g.backdrop),
        ))
    } else {
        scrim.justify_start().pt(px(TOP)).bg(rgba(COMMAND_SCRIM))
    };
    let scrim = scrim.when(!frame.compact, |scrim| {
        scrim.opacity(animation.map_or(1.0, |a| a.motion.opacity))
    });
    let scrim = scrim.when(closing && frame.compact, |scrim| {
        scrim.child(block_exit_input())
    });
    // Over a dialog (at 2), under a tooltip (at 4).
    deferred(
        anchored()
            .position(point(px(0.0), px(0.0)))
            .child(scrim.child(panel)),
    )
    .with_priority(3)
    .into_any_element()
}

/// Gorhom handle: padding10 around a 7.5%-window-wide, 4-high indicator.
fn handle(
    color: gpui::Hsla,
    width: Pixels,
    animation: Option<&CommandAnimation<'_>>,
    release: &OnRelease,
) -> Div {
    div()
        .relative()
        .w_full()
        .flex_none()
        .flex()
        .justify_center()
        .py(px(10.0))
        .child(
            div()
                .w(px(f32::from(width).mul_add(0.075, 0.0)))
                .h(px(4.0))
                .rounded(px(4.0))
                .bg(color),
        )
        .when_some(
            animation.filter(|a| !a.motion.closing),
            |handle, animation| {
                handle.child(listener(
                    &animation.state.sheet,
                    Origin::Handle,
                    animation.scroll,
                    release,
                ))
            },
        )
}
/// The compact backdrop keeps its target until portal dismissal, as
/// sheet-backdrop.web does. Consume outgoing row input during that spring;
/// desktop close unmounts immediately. Focus is already returned by the host.
fn block_exit_input() -> AnyElement {
    canvas(
        |_, _, _| (),
        |_, (), window, _| {
            window.on_mouse_event(|_: &gpui::MouseDownEvent, phase, _, cx| {
                if phase.capture() {
                    cx.stop_propagation();
                }
            });
            window.on_mouse_event(|_: &gpui::MouseUpEvent, phase, _, cx| {
                if phase.capture() {
                    cx.stop_propagation();
                }
            });
            window.on_mouse_event(|_: &gpui::ScrollEvent, phase, _, cx| {
                if phase.capture() {
                    cx.stop_propagation();
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

/// The key-capture boundary is the same for static and animated frames.
fn panel<V: 'static>(
    handlers: CommandHandlers<V>,
    closing: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<Div> {
    div()
        .id("command")
        .capture_action(cx.listener(move |view: &mut V, _: &MoveUp, window, cx| {
            cx.stop_propagation();
            if !closing {
                (handlers.on_key)(view, CommandKey::Up, window, cx);
            }
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &MoveDown, window, cx| {
            cx.stop_propagation();
            if !closing {
                (handlers.on_key)(view, CommandKey::Down, window, cx);
            }
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Enter, window, cx| {
            cx.stop_propagation();
            if !closing {
                (handlers.on_key)(view, CommandKey::Enter, window, cx);
            }
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Escape, window, cx| {
            cx.stop_propagation();
            if !closing {
                (handlers.on_key)(view, CommandKey::Escape, window, cx);
            }
        }))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(tokens.neutral.surface0)
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
}
