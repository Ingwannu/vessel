//! Only desktop placement is new; compact content delegates to the one dialog.

use super::{Alignment, Opening, PopoverHandlers, PopoverState, Width, dismiss};
use crate::dialog::{CompactLayout, DesktopHeight, DialogContent, DialogLayout, DialogSize};
use crate::popup::{self, Surfaces};
use crate::sheet::snap::Percentage;
use crate::{Tokens, dialog};
use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Context, ElementId, KeyDownEvent, MouseButton, SharedString, Window,
};
use gpui::{anchored, deferred, div, point, px};
use std::rc::Rc;

const OFFSET: f32 = 4.0;
const MARGIN: f32 = 8.0;
const HORIZONTAL_PADDING: f32 = 16.0;
const MIN_VISIBLE: f32 = 1.0;
const COMPACT_CAP: u8 = 90;

/// Caller-owned fields, authored desktop width and existing sheet configuration.
pub struct PopoverContent {
    /// Stable caller identity for layout and animation.
    pub id: ElementId,
    /// Only the compact sheet has a header; desktop keeps content height natural.
    pub title: SharedString,
    /// Any existing kit fields or other caller body, never a menu-row encoding.
    pub body: AnyElement,
    /// Positive maximum desktop width in logical pixels.
    pub width: Width,
    /// Fitting or draggable compact layout through the existing snap engine.
    pub compact: CompactLayout,
}

impl PopoverContent {
    /// Natural-height compact content, capped at the existing dialog's 90% limit.
    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        body: AnyElement,
        width: Width,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body,
            width,
            compact: CompactLayout::Fixed(Percentage::new(COMPACT_CAP).unwrap_or(Percentage::FULL)),
        }
    }
}

impl std::fmt::Debug for PopoverContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopoverContent")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("compact", &self.compact)
            .finish_non_exhaustive()
    }
}

/// One adaptive content surface, or None while closed. `narrow` uses the app's
/// existing phone breakpoint. The same lease guards every dismissal path.
pub fn popover<V: 'static>(
    state: &PopoverState,
    content: PopoverContent,
    handlers: &PopoverHandlers<V>,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let opening = state.opening.as_ref()?;
    let lease = opening.lease.clone();
    let close_handlers = PopoverHandlers {
        state: handlers.state,
        admit: Rc::clone(&handlers.admit),
    };
    let close = move |view: &mut V, window: &mut Window, cx: &mut Context<'_, V>| {
        dismiss(view, &lease, &close_handlers, window, cx);
    };
    if narrow {
        let layout = DialogLayout::new(
            content.width.get(),
            DesktopHeight::AtMost(Percentage::FULL),
            content.compact,
        )?;
        return Some(dialog(
            &state.dialog,
            DialogContent {
                title: content.title,
                body: content.body,
                footer: Vec::new(),
                size: DialogSize::Configured(layout),
            },
            tokens,
            window,
            true,
            close,
            cx,
        ));
    }
    Some(desktop(state, opening, content, tokens, window, close, cx))
}

fn desktop<V: 'static>(
    state: &PopoverState,
    opening: &Opening,
    content: PopoverContent,
    tokens: &Tokens,
    window: &Window,
    close: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + Clone + 'static,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let (corner, at) = match opening.alignment {
        Alignment::Start => (Anchor::TopLeft, opening.anchor.bottom_left()),
        Alignment::End => (Anchor::TopRight, opening.anchor.bottom_right()),
    };
    let id = content.id;
    let surfaces = Surfaces::of(tokens);
    let viewport = window.viewport_size();
    let width = content
        .width
        .pixels()
        .min((viewport.width - px(MARGIN) - px(MARGIN)).max(px(MIN_VISIBLE)));
    let height = (viewport.height - px(MARGIN) - px(MARGIN)).max(px(MIN_VISIBLE));
    let escape = close.clone();
    let card = popup::layer(
        div()
            .id(id.clone())
            .track_focus(state.dialog.focus_handle())
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    escape(view, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .flex()
            .flex_col()
            .w(width)
            .max_h(height)
            .overflow_y_scroll()
            .px(px(HORIZONTAL_PADDING))
            .bg(surfaces.surface1)
            .border_1()
            .border_color(surfaces.border_accent)
            .rounded(px(popup::MENU_RADIUS))
            .shadow(popup::shadow_md(tokens))
            .child(content.body),
    );
    let surface = anchored()
        .anchor(corner)
        .position(at)
        .offset(point(px(0.0), px(OFFSET)))
        .snap_to_window_with_margin(px(MARGIN))
        .child(popup::appear(
            ElementId::NamedChild(std::sync::Arc::new(id), "appear".into()),
            card,
            crate::motion::QUICK,
        ));
    let overlay = popup::layer(
        div()
            .w(viewport.width)
            .h(viewport.height)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _, window, cx| {
                    close(view, window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(surface),
    );
    deferred(anchored().position(point(px(0.0), px(0.0))).child(overlay))
        .with_priority(2)
        .into_any_element()
}
