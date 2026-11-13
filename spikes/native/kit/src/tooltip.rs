//! A tooltip, as Paseo's (`components/ui/tooltip.tsx` with the composer's
//! `delayDuration={0}`, `side="top"`, `align="center"`, `offset={8}`,
//! `enabledOnMobile={false}`): words in a small card centred above the
//! control the pointer is on, shown at once, gone when the pointer leaves
//! or presses.
//!
//! [Decision Log]
//! - Purpose: the composer's controls say what they are when pointed at
//!   (the coordinator, for 0.0.0.0.7: a feature's control was "Off" and
//!   nothing told which feature).
//! - Constraints: GPUI's own `tooltip` waits before it shows and sits by
//!   the pointer; Paseo's shows at once, above the control, centred.
//! - Choice: one tooltip per screen ([`Tooltips`], kept by the screen):
//!   each control records where it is drawn and, on hover, makes itself
//!   the one shown; the screen draws [`Tooltips::render`] over
//!   everything. Paseo's values: 4 by 8 inside, round 12, the popover's
//!   surface (`surface0` light, `surface2` dark) edged with `borderAccent`,
//!   a medium shadow, 14 in the foreground, at most 280 wide, 8 above the
//!   control and kept 8 from the window's edges.
//! - Differences: no fade (Paseo's fades in); not on touch, as Paseo's.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Bounds, MouseButton, Pixels, SharedString, Stateful, anchored, deferred,
    div, point, px,
};

use crate::fonts::Type;
use crate::picker::anchor_probe;
use crate::popup;
use crate::theme::{Scheme, Tokens};
use crate::widgets::TypeExt as _;

/// Where a trigger was last drawn (one per trigger, kept by the screen).
pub type Place = Rc<std::cell::Cell<Option<Bounds<Pixels>>>>;

/// The tooltip shown, if any.
#[derive(Debug, Clone)]
struct Tip {
    text: SharedString,
    place: Place,
}

/// A screen's tooltips: at most one shown.
#[derive(Debug, Clone, Default)]
pub struct Tooltips {
    shown: Rc<RefCell<Option<Tip>>>,
}

impl Tooltips {
    /// No tooltip shown.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `trigger` shows `text` while the pointer is on it (nothing on
    /// touch, as Paseo's). `place` is the trigger's own cell for where it
    /// is drawn (kept by the screen across frames).
    #[must_use]
    pub fn on(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        text: impl Into<SharedString>,
        touch: bool,
    ) -> Stateful<gpui::Div> {
        if touch {
            return trigger;
        }
        let text = text.into();
        let hover_shown = Rc::clone(&self.shown);
        let hover_place = Rc::clone(place);
        let press_shown = Rc::clone(&self.shown);
        trigger
            .child(anchor_probe(place))
            .on_hover(move |hovered, window, _cx| {
                let mut shown = hover_shown.borrow_mut();
                if *hovered {
                    *shown = Some(Tip {
                        text: text.clone(),
                        place: Rc::clone(&hover_place),
                    });
                } else if shown
                    .as_ref()
                    .is_some_and(|tip| Rc::ptr_eq(&tip.place, &hover_place))
                {
                    *shown = None;
                }
                window.refresh();
            })
            .on_mouse_down(MouseButton::Left, move |_event, window, _cx| {
                *press_shown.borrow_mut() = None;
                window.refresh();
            })
    }

    /// The tooltip shown, over everything.
    #[must_use]
    pub fn render(&self, tokens: &Tokens) -> Option<AnyElement> {
        let tip = self.shown.borrow().clone()?;
        let bounds = tip.place.get()?;
        let neutral = tokens.neutral;
        let surface = match tokens.scheme {
            Scheme::Light => neutral.surface0,
            Scheme::Dark => neutral.surface2,
        };
        let top = point(bounds.center().x, bounds.top());
        let card = div()
            .max_w(px(280.0))
            .py(px(4.0))
            .px(px(8.0))
            .rounded(px(12.0))
            .bg(surface)
            .border_1()
            .border_color(neutral.border_accent)
            .shadow(popup::shadow_md(tokens))
            .type_(Type::Body)
            .line_height(px(LINE))
            .text_color(neutral.foreground)
            .child(tip.text);
        Some(
            deferred(
                anchored()
                    .position(top)
                    .anchor(Anchor::BottomCenter)
                    .offset(point(px(0.0), px(ABOVE)))
                    .snap_to_window_with_margin(px(8.0))
                    .child(card),
            )
            .with_priority(4)
            .into_any_element(),
        )
    }
}

/// 8 above the trigger (upward, so negative).
const ABOVE: f32 = -8.0;

/// Paseo's `fontSize.base * 1.4`.
const LINE: f32 = 19.6;
