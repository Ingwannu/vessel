//! A modal decision over the current screen: the archive's and the
//! kill's confirms, a delete, a move to another profile. Paseo's adaptive
//! modal (`components/adaptive-modal-sheet.tsx`), cloned: a card on a
//! wide window, a sheet from the bottom on a phone.
//!
//! [Decision Log]
//! - Purpose: one dialog for every screen, so the scrim, Escape and a
//!   click outside behave the same everywhere, with Paseo's look (the
//!   owner's rule after build 0.0.0.0.3).
//! - Values (Paseo's): on a wide window a card at most 520 wide and 85%
//!   of the window tall, on `surface0`, edged with `surface2` at radius
//!   12, over black at 55% with 24 around; its header 24 by 16 with a
//!   rule under it, the title 14 medium, the close button at its right;
//!   the body 24 inside; the footer 24 by 12 with a rule over it. On a
//!   phone the same parts on a sheet as wide as the window, its top
//!   corners at 16, a handle on top, over black at 45%.
//! - Differences from the page's DESIGN.md (the references win): the
//!   page's dialog is 512 wide on the popup surface at radius 10 with no
//!   rules between its parts; the page has no sheet.
//! - Alternatives: `gpui-component`'s dialog layer (opened through its
//!   root, styled by its theme, its own buttons): a screen would then
//!   name its types.
//! - Choice: the dialog is drawn by the screen that keeps it open, over
//!   everything, with a focus handle of the screen's, so Escape reaches it
//!   and nothing behind it takes a click. On a narrow window it takes the
//!   full viewport width.
//! - Cost: default fitting sheets close from the scrim or close button;
//!   configured sheets reuse the snap engine. Focus is not trapped inside the
//!   dialog the way the page's is
//!   (GPUI has no inert subtree); what is behind it cannot be clicked,
//!   but a Tab could reach it. Focus goes to the dialog on open, not to
//!   its first field: the screen focuses a field itself when it has one.
//!
//! [Decision Log: captured dismissal]
//! - Purpose: closing an old dialog must not dismiss a newer decision.
//! - Constraint: a function pointer cannot retain the decision's identity.
//! - Alternatives: caller-global mutable identity or a second modal renderer.
//! - Choice: accept a clonable closure, used by Escape, scrim and close glyph.
//! - Why: each path retains the same immutable identity without kit domain types.
//! - Cost: the caller owns stale-decision policy; legacy function items still fit.

use gpui::prelude::*;
use gpui::{
    AnimationExt as _, AnyElement, App, Context, FocusHandle, KeyDownEvent, MouseButton,
    SharedString, Window, anchored, deferred, div, point, px, relative, svg,
};

#[cfg(test)]
mod accessibility_tests;
#[cfg(test)]
mod compact_tests;
#[cfg(test)]
mod configuration_tests;
mod header_back;
#[cfg(test)]
mod header_back_tests;
mod header_content;
mod header_search;
#[cfg(test)]
mod header_search_tests;
mod layout;
mod return_focus;
#[cfg(test)]
mod selection_tests;
mod sheet;
#[cfg(test)]
mod title_wrap_tests;

pub use header_back::HeaderBack;
pub use header_content::DialogInput;
pub use header_search::HeaderSearch;
pub use layout::{BodySpacing, CompactLayout, DesktopHeight, DialogLayout, DialogWidth};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The inset of a dialog's parts (Paseo's `spacing[6]`).
const INSIDE: f32 = 24.0;

/// How wide a dialog is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSize {
    /// Paseo's 520 points at most.
    Normal,
    /// 640 pixels, for a grid of cards.
    Wide,
    /// Explicit validated geometry, using the same renderer and snap engine.
    Configured(DialogLayout),
}

/// A dialog's focus, kept by the screen that shows it.
#[derive(Debug)]
pub struct DialogState {
    focus: FocusHandle,
    opener: crate::focus::ReturnFocus,
    sheet: sheet::SheetState,
}

impl DialogState {
    /// A dialog's state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
            opener: crate::focus::ReturnFocus::default(),
            sheet: sheet::SheetState::default(),
        }
    }
}

/// What a dialog shows.
pub struct DialogContent {
    /// Its title.
    pub title: SharedString,
    /// What it says or asks.
    pub body: AnyElement,
    /// Its actions, right-aligned: a quiet Cancel, then one primary.
    pub footer: Vec<AnyElement>,
    /// How wide.
    pub size: DialogSize,
}

impl std::fmt::Debug for DialogContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogContent")
            .field("title", &self.title)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// The dialog, over everything: Escape, the scrim and the close button
/// call `close`. `narrow`: a phone's width (the lead's breakpoint), where
/// it is a sheet from the bottom.
pub fn dialog<V: 'static>(
    state: &DialogState,
    content: impl Into<DialogInput>,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    close: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + Clone + 'static,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let (content, header_parts) = content.into().parts();
    let surfaces = Surfaces::of(tokens);
    let layout = content.size.layout();
    let sheet = match (narrow, layout.compact()) {
        (true, CompactLayout::Draggable(points)) => Some(state.sheet.frame(points, window, cx)),
        (true, CompactLayout::Fixed(_))
        | (false, CompactLayout::Fixed(_) | CompactLayout::Draggable(_)) => None,
    };
    let release = sheet
        .as_ref()
        .map(|sheet| sheet.on_release(close.clone(), cx));
    let footer = footer(content.footer, &surfaces);
    let header = header(
        content.title,
        header_parts,
        &surfaces,
        narrow,
        close.clone(),
        cx,
    );
    let escape_close = close.clone();
    let card = div()
        .id("dialog")
        .track_focus(&state.focus)
        .key_context(popup::KEY_CONTEXT)
        .on_key_down(
            cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    escape_close(view, window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
        // A press inside the card must neither close (the scrim's close) nor
        // move focus (the card's own `track_focus` focus-on-press, and any
        // focusable ancestor's), yet the window's text-selection layer
        // (gpui-base's Root, painted under every deferred layer) must still
        // see it, or a selectable value in a dialog could not be selected
        // (Paseo's `DetailRow` values are `selectable`). So the card occludes
        // the scrim's hitbox and marks the press default-prevented, which
        // GPUI's focus transfer honours and the selection layer ignores,
        // instead of stopping the event. GPUI registers an element's focus
        // listener before its own mouse-down listeners, so this listener runs
        // first in the bubble phase, as the old `stop_propagation` did.
        .occlude()
        .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
            window.prevent_default();
        })
        .flex()
        .flex_col()
        .w_full()
        .overflow_hidden()
        .bg(surfaces.surface0)
        .when(!narrow, |card| {
            card.max_w(layout.width().pixels())
                .map(|card| match layout.height() {
                    DesktopHeight::AtMost(height) => card.max_h(relative(height.fraction())),
                    DesktopHeight::Exact(height) => card.h(relative(height.fraction())),
                })
                .border_1()
                .border_color(surfaces.surface2)
                .rounded(px(popup::DIALOG_RADIUS))
        })
        .when(narrow, |card| {
            card.map(|sheet| match layout.compact() {
                CompactLayout::Fixed(height) => sheet.max_h(relative(height.fraction())),
                CompactLayout::Draggable(_) => sheet,
            })
            .map(
                |card| match sheet.as_ref().and_then(|sheet| sheet.geometry) {
                    Some(geometry) => card.h(geometry.height),
                    None => card,
                },
            )
            .rounded_t(px(popup::SHEET_RADIUS))
            .child(sheet_handle(tokens.neutral.foreground_extra_muted).map(|handle| {
                match (&sheet, &release) {
                    (Some(sheet), Some(release)) => handle
                        .relative()
                        .child(sheet.listener(crate::sheet::snap::Origin::Handle, release)),
                    (Some(_), None) | (None, Some(_) | None) => handle,
                }
            }))
        })
        .child(header)
        .child(body(
            content.body,
            &surfaces,
            sheet.as_ref(),
            release.as_ref(),
            layout.body_spacing(),
        ))
        .children(footer);
    overlay(card, sheet, narrow, window, close, cx)
}

fn overlay<V: 'static>(
    card: gpui::Stateful<gpui::Div>,
    sheet: Option<sheet::SheetFrame>,
    narrow: bool,
    window: &Window,
    close: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + 'static,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    // The whole window, wherever the screen that keeps it open is drawn
    // (Paseo's overlay is `absoluteFill` of the window).
    let size = window.viewport_size();
    let scrim = div()
        .id("dialog-scrim")
        .w(size.width)
        .h(size.height)
        .flex()
        .justify_center()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                close(view, window, cx);
                cx.notify();
            }),
        );
    let (scrim, shade) = if narrow {
        (scrim.items_end(), popup::sheet_scrim())
    } else {
        (scrim.items_center().p(px(24.0)), popup::scrim())
    };
    // Word 39: the scrim darkens and the card arrives over the kit's
    // surface length; Paseo's dialog leaves at once (its Modal unmounts).
    let overlay = if let Some(sheet) = sheet {
        let coverage = sheet.geometry.map_or(0.0, |geometry| geometry.backdrop);
        scrim
            .bg(shade.opacity(coverage))
            .child(card)
            .into_any_element()
    } else {
        let card = popup::appear("dialog-appear", card, crate::motion::STANDARD);
        scrim
            .child(card)
            .with_animation(
                "dialog-scrim-appear",
                crate::motion::transition(crate::motion::STANDARD),
                move |scrim, done| scrim.bg(shade.opacity(done)),
            )
            .into_any_element()
    };
    deferred(anchored().position(point(px(0.0), px(0.0))).child(overlay))
        .with_priority(2)
        .into_any_element()
}

fn footer(children: Vec<AnyElement>, surfaces: &Surfaces) -> Option<gpui::Div> {
    (!children.is_empty()).then(|| {
        div()
            .flex()
            .flex_shrink_0()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .px(px(INSIDE))
            .py(px(12.0))
            .border_t_1()
            .border_color(surfaces.surface2)
            .children(children)
    })
}

fn body(
    content: AnyElement,
    surfaces: &Surfaces,
    sheet: Option<&sheet::SheetFrame>,
    release: Option<&crate::sheet::snap::listener::OnRelease>,
    spacing: BodySpacing,
) -> gpui::Stateful<gpui::Div> {
    div().id("dialog-body").flex().flex_col()
        // The body yields to the card's cap so long text cannot hide actions.
        .flex_auto().min_h(px(0.0)).overflow_y_scroll()
        .map(|body| match sheet { Some(sheet) => body.track_scroll(&sheet.scroll), None => body })
        .when(sheet.is_some(), Styled::relative)
        .gap(px(16.0)).when(spacing == BodySpacing::Inset, |body| body.p(px(INSIDE))).type_(Type::Body)
        .text_color(surfaces.foreground_muted).child(content)
        .map(|body| match (sheet, release) {
            (Some(sheet), Some(release)) => body.child(sheet.listener(crate::sheet::snap::Origin::Content, release)),
            (Some(_), None) | (None, Some(_) | None) => body,
        })
}

/// A sheet's drag handle (Gorhom's indicator; Paseo themes it zinc 600,
/// the kit's nearest neutral step is `foregroundExtraMuted`). Every bottom
/// sheet draws it (Paseo's `SheetHandle`): the dialog's, the menu's and
/// the list's.
pub(crate) fn sheet_handle(color: gpui::Hsla) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .pt(px(8.0))
        .child(div().w(px(32.0)).h(px(4.0)).rounded(px(2.0)).bg(color))
}

/// The title and the close button, over a rule; with a way back, Paseo's back
/// arrow leads the title; with a search, Paseo's search row sits under the
/// title inside the same rule (`headerContainer`).
fn header<V: 'static>(
    title: SharedString,
    parts: header_content::HeaderParts,
    surfaces: &Surfaces,
    narrow: bool,
    close: impl Fn(&mut V, &mut Window, &mut Context<'_, V>) + 'static,
    cx: &mut Context<'_, V>,
) -> gpui::Div {
    let header_content::HeaderParts {
        back,
        actions,
        search,
    } = parts;
    let has_actions = actions.is_some();
    let close_size = if narrow { 44.0 } else { 28.0 };
    // Paseo names both header controls for assistive technology
    // (adaptive-modal-sheet.tsx 411-434); the kit takes the words from the app.
    let names = crate::control_names(cx);
    let hover = surfaces.surface2;
    let back = back.map(|back| {
        header_back::button(
            back,
            &names.back,
            surfaces.foreground_muted,
            surfaces.foreground,
        )
    });
    let row = div()
        .debug_selector(|| "dialog-header".to_owned())
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(8.0))
        .px(px(INSIDE))
        .py(px(16.0))
        .children(back)
        .child(
            div()
                .debug_selector(|| "dialog-title".to_owned())
                .flex_1()
                .min_w(px(0.0))
                .when(has_actions, Styled::truncate)
                .type_(Type::Label)
                .text_color(surfaces.foreground)
                .child(title),
        )
        .children(actions.map(|actions| {
            div()
                .debug_selector(|| "dialog-header-actions".to_owned())
                .flex()
                .flex_none()
                .items_center()
                .gap(px(crate::theme::spacing::S2))
                .child(actions)
        }))
        .child(
            div()
                .id("dialog-close")
                .debug_selector(|| "dialog-close".to_owned())
                .role(gpui::Role::Button)
                .aria_label(names.close)
                .size(px(close_size))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(popup::MENU_RADIUS))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                    close(view, window, cx);
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(Glyph::Close.path())
                        .size(px(10.0))
                        .text_color(surfaces.foreground_muted),
                ),
        );
    match search {
        None => row.border_b_1().border_color(surfaces.surface2),
        Some(search) => div()
            .debug_selector(|| "dialog-header-group".to_owned())
            .flex()
            .flex_col()
            .flex_shrink_0()
            .border_b_1()
            .border_color(surfaces.surface2)
            .child(row)
            .child(header_search::search_row(
                search,
                INSIDE,
                surfaces.foreground_muted,
            )),
    }
}

#[cfg(test)]
mod dismissal_tests;

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{Bounds, Pixels, Render, TestAppContext, size};

    use super::*;
    use crate::theme::Scheme;

    struct Probe {
        state: DialogState,
        narrow: bool,
        lines: u16,
        actions: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    }

    impl Render for Probe {
        fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let actions = Rc::clone(&self.actions);
            let footer = div()
                .flex_shrink_0()
                .on_children_prepainted(move |bounds, _, _| *actions.borrow_mut() = bounds)
                .child(div().w(px(80.0)).h(px(32.0)));
            dialog(
                &self.state,
                DialogContent {
                    title: "A decision".into(),
                    body: div()
                        .h(px(f32::from(self.lines.saturating_mul(21))))
                        .flex_shrink_0()
                        .into_any_element(),
                    footer: vec![footer.into_any_element()],
                    size: DialogSize::Normal,
                },
                &Tokens::of(Scheme::Dark),
                window,
                self.narrow,
                |_, _, _| {},
                cx,
            )
        }
    }

    #[gpui::test]
    fn a_long_body_cannot_shrink_or_hide_the_actions(cx: &mut TestAppContext) {
        for (width, narrow) in [(1280.0, false), (390.0, true)] {
            for lines in [1, 120] {
                let actions = Rc::new(RefCell::new(Vec::new()));
                let (_, window) = cx.add_window_view({
                    let actions = Rc::clone(&actions);
                    move |_, cx| Probe {
                        state: DialogState::new(cx),
                        narrow,
                        lines,
                        actions,
                    }
                });
                window.simulate_resize(size(px(width), px(800.0)));
                window.run_until_parked();
                let measured = actions.borrow();
                let action = measured.first().expect("the action was laid out");
                assert_eq!(
                    action.size.height,
                    px(32.0),
                    "the action must keep its height"
                );
                assert!(action.top() >= px(0.0), "the action is above the window");
                assert!(
                    action.bottom() <= px(800.0),
                    "the action is clipped below the card"
                );
            }
        }
    }
}
