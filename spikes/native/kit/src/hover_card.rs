//! A hover card, as Paseo's (`components/ui/hover-card.tsx`, TRACKER 810):
//! a card that opens while the pointer is on its trigger and stays open
//! while the pointer crosses onto it, so what it holds can be pressed.
//!
//! [Decision Log]
//! - Purpose: a session row's details (its branch, folder, pull request)
//!   a hover away, with buttons in them, as Paseo's sidebar has them.
//! - Before: the kit's [`crate::Tooltips`] shows words at once beside a
//!   control, but they close when the pointer leaves the control, so
//!   nothing in them can be pressed.
//! - Choice: one card per screen, kept by the screen as its tooltips are.
//!   - A trigger records where it is drawn and, on hover, makes its key
//!     the card shown, at once (Paseo's `openNow`).
//!   - Leaving the trigger or the card closes it 100 ms later
//!     (`CLOSE_GRACE_MS`), unless the pointer reaches the other first. One
//!     owned timer does this; dropping it cancels it, as the tooltip's
//!     does.
//!   - A press on the trigger closes it, so the trigger's own click and
//!     menu win. Escape closes it, whatever has the keys (an app-wide
//!     interceptor that does not consume the key).
//!   - The screen draws [`HoverCards::render`] with what the card holds.
//!     It sits right of the trigger, centred on its height, 4 away
//!     (Paseo's `placement="right"`, `alignment="center"`, `offset={4}`),
//!     and is kept 8 from the window's edges.
//!   - The card is Paseo's: `surface1`, a 1 `borderAccent` edge, radius 8,
//!     8 above its first row, its own shadow (0/4/8, black at 20%, in
//!     both themes).
//! - Differences, named: Paseo also keeps the card open while keyboard focus
//!   is inside it. The native rows are not tab stops (pointer only, the
//!   coordinator's ruling). Paseo's safe zone (`useHoverSafeZone`) is the
//!   grace here: the 4 between trigger and card take less than 100 ms to
//!   cross.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, BoxShadow, MouseButton, SharedString, Stateful, Subscription, Task, Window,
    anchored, deferred, div, point, px,
};

use crate::picker::anchor_probe;
use crate::popup::Surfaces;
use crate::theme::Tokens;
use crate::tooltip::Place;

/// How long the card waits after the pointer left it or its trigger
/// (Paseo's `CLOSE_GRACE_MS`).
pub const CLOSE_GRACE: Duration = Duration::from_millis(100);
/// Its distance from the trigger (Paseo's `offset` default).
const OFFSET: f32 = 4.0;
/// Its distance from the window's edges.
const MARGIN: f32 = 8.0;
/// Its corners (Paseo's `borderRadius.lg`) and the room above its first
/// row (`paddingTop: spacing[2]`).
const CORNER: f32 = 8.0;
const TOP: f32 = 8.0;
/// Its shadow's strength (`shadowOpacity: 0.2`).
const SHADOW_ALPHA: f32 = 0.2;

/// The card shown, where the pointer is, and the timer that will close
/// the card.
#[derive(Default)]
struct State {
    shown: Option<Shown>,
    /// The trigger the pointer is on, by key: a leave from another
    /// trigger, arriving after this one's enter, changes nothing.
    on_trigger: Option<SharedString>,
    /// The pointer is on the card.
    on_card: bool,
    // Ownership is cancellation: GPUI's `Task` cancels when dropped.
    closing: Option<Task<()>>,
    /// Escape's interceptor (`listen_for_escape`).
    escape: Option<Subscription>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("shown", &self.shown)
            .field("on_trigger", &self.on_trigger)
            .field("on_card", &self.on_card)
            .field("closing", &self.closing.is_some())
            .field("escape", &self.escape.is_some())
            .finish()
    }
}

impl State {
    /// The pointer is on neither the trigger shown nor the card.
    fn left(&self) -> bool {
        !self.on_card && self.on_trigger.is_none()
    }
}

#[derive(Debug, Clone)]
struct Shown {
    key: SharedString,
    place: Place,
}

/// A screen's hover cards: at most one shown.
#[derive(Debug, Clone, Default)]
pub struct HoverCards {
    state: Rc<RefCell<State>>,
}

impl HoverCards {
    /// No card shown.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `trigger` shows the card keyed `key` while the pointer is on it;
    /// `place` is the trigger's own cell for where it is drawn. When not
    /// `enabled` (touch, a narrow window, the trigger's menu open, a drag
    /// under way), the trigger is drawn alone.
    #[must_use]
    pub fn on(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        key: impl Into<SharedString>,
        enabled: bool,
    ) -> Stateful<gpui::Div> {
        if !enabled {
            return trigger;
        }
        let key = key.into();
        let hovering = Rc::clone(&self.state);
        let hover_place = Rc::clone(place);
        let pressing = Rc::clone(&self.state);
        trigger
            .child(anchor_probe(place))
            .on_hover(move |hovered, window, cx| {
                if *hovered {
                    open(&hovering, key.clone(), Rc::clone(&hover_place));
                    listen_for_escape(&hovering, cx);
                } else {
                    {
                        let mut state = hovering.borrow_mut();
                        if state.on_trigger.as_ref() == Some(&key) {
                            state.on_trigger = None;
                        }
                    }
                    close_soon(&hovering, window, cx);
                }
                window.refresh();
            })
            .on_mouse_down(MouseButton::Left, move |_event, window, _cx| {
                close_now(&pressing);
                window.refresh();
            })
    }

    /// The key of the card shown, if any.
    #[must_use]
    pub fn shown(&self) -> Option<SharedString> {
        self.state
            .borrow()
            .shown
            .as_ref()
            .map(|shown| shown.key.clone())
    }

    /// Close the card (its trigger's menu opening, a drag starting).
    pub fn close(&self) {
        close_now(&self.state);
    }

    /// The card shown, holding `content`, named `label` as a group, over
    /// everything; `width` is its own (Paseo's workspace card: 260).
    #[must_use]
    pub fn render(
        &self,
        content: impl IntoElement,
        label: impl Into<SharedString>,
        width: f32,
        tokens: &Tokens,
    ) -> Option<AnyElement> {
        let shown = self.state.borrow().shown.clone()?;
        let bounds = shown.place.get()?;
        let surfaces = Surfaces::of(tokens);
        let hovering = Rc::clone(&self.state);
        let card = div()
            .id(gpui::ElementId::NamedChild(
                std::sync::Arc::new(gpui::ElementId::Name(shown.key.clone())),
                "hover-card".into(),
            ))
            .debug_selector(|| "hover-card".to_owned())
            .w(px(width))
            .pt(px(TOP))
            .flex()
            .flex_col()
            .bg(surfaces.surface1)
            .border_1()
            .border_color(surfaces.border_accent)
            .rounded(px(CORNER))
            .shadow(shadow())
            .occlude()
            .role(gpui::Role::Group)
            .aria_label(label)
            .on_hover(move |hovered, window, cx| {
                hovering.borrow_mut().on_card = *hovered;
                if *hovered {
                    cancel_close(&hovering);
                } else {
                    close_soon(&hovering, window, cx);
                }
            })
            .child(content);
        Some(
            deferred(
                anchored()
                    .position(point(bounds.right(), bounds.center().y))
                    .anchor(gpui::Anchor::LeftCenter)
                    .offset(point(px(OFFSET), px(0.0)))
                    .snap_to_window_with_margin(px(MARGIN))
                    .child(card),
            )
            .with_priority(4)
            .into_any_element(),
        )
    }
}

/// Paseo's card shadow (`workspace-hover-card.tsx`: `#000` at 20%, 4
/// down, 8 of blur), the same in both themes.
fn shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: gpui::black().opacity(SHADOW_ALPHA),
        offset: point(px(0.0), px(4.0)),
        blur_radius: px(8.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

fn open(state: &Rc<RefCell<State>>, key: SharedString, place: Place) {
    let mut state = state.borrow_mut();
    let another = state.shown.as_ref().is_none_or(|shown| shown.key != key);
    if another {
        // A new card: the pointer is not on it yet.
        state.on_card = false;
    }
    state.closing = None;
    state.on_trigger = Some(key.clone());
    state.shown = Some(Shown { key, place });
}

fn cancel_close(state: &Rc<RefCell<State>>) {
    state.borrow_mut().closing = None;
}

fn close_now(state: &Rc<RefCell<State>>) {
    let mut state = state.borrow_mut();
    state.closing = None;
    state.shown = None;
    state.on_card = false;
}

/// Close after the grace, unless the pointer comes back first (its hover
/// cancels the timer). The timer holds only a weak reference.
fn close_soon(state: &Rc<RefCell<State>>, window: &Window, cx: &App) {
    {
        let state = state.borrow();
        if state.shown.is_none() || state.closing.is_some() || !state.left() {
            return;
        }
    }
    let weak = Rc::downgrade(state);
    let handle = window.window_handle();
    let task = cx.spawn(async move |cx| {
        cx.background_executor().timer(CLOSE_GRACE).await;
        let _closed = handle.update(cx, |_, window, _cx| {
            if let Some(state) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.closing = None;
                // The pointer came back without a hover event reaching the
                // timer's owner (it cancels): close only if it is still away.
                if state.left() {
                    state.shown = None;
                    state.on_card = false;
                }
                window.refresh();
            }
        });
    });
    state.borrow_mut().closing = Some(task);
}

/// Escape closes the card shown, whatever has the keys (Paseo's
/// `keyPressed` on the document). A listener painted with the card would
/// only hear keys bound for its own branch (it never holds focus), so the
/// screen's cards hold one app-wide interceptor, made when a card first
/// opens and kept while the cards live; it never consumes the key.
fn listen_for_escape(state: &Rc<RefCell<State>>, cx: &mut App) {
    if state.borrow().escape.is_some() {
        return;
    }
    let weak = Rc::downgrade(state);
    let escape = cx.intercept_keystrokes(move |event, window, _cx| {
        if event.keystroke.key != "escape" {
            return;
        }
        let Some(state) = weak.upgrade() else {
            return;
        };
        let shown = state.borrow().shown.is_some();
        if shown {
            close_now(&state);
            window.refresh();
        }
    });
    state.borrow_mut().escape = Some(escape);
}

#[cfg(test)]
#[path = "hover_card_tests.rs"]
mod tests;
