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
//! - Typed extension: the same controller also supports pane toolbar bottom/8/300ms;
//!   existing callers keep immediate/top/8. One owned timer cancels on leave/press,
//!   uses original trigger identity and holds no strong screen/controller reference.

mod config;
mod pending;
pub use config::{Configuration, Delay, InvalidOffset, Offset, Side};

use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, Bounds, MouseButton, Pixels, SharedString, Stateful, anchored, deferred, div, px,
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
    content: Content,
    place: Place,
    configuration: Configuration,
    // Weak ownership of the actually installed trigger listener, never a timer
    // keepalive. Rebuilding the same Place refreshes this existing tip in place.
    trigger: std::rc::Weak<()>,
}

/// Content keeps an exact path/hash in its own code face instead of
/// styling an interpolated fragment of an ordinary sentence.
#[derive(Debug, Clone)]
enum Content {
    Text(SharedString),
    Code(SharedString),
    /// Words and the bound shortcut's chords after them (Paseo's
    /// `ToolbarButton` tooltip: the label, then `<Shortcut chord>`, gap 8).
    Keyed(SharedString, Rc<[SharedString]>),
}

impl Content {
    const fn face(&self) -> Type {
        match self {
            Self::Text(_) | Self::Keyed(..) => Type::Body,
            Self::Code(_) => Type::Mono,
        }
    }

    const fn text(&self) -> &SharedString {
        match self {
            Self::Text(text) | Self::Code(text) | Self::Keyed(text, _) => text,
        }
    }

    fn keys(&self) -> Option<&[SharedString]> {
        match self {
            Self::Keyed(_, keys) if !keys.is_empty() => Some(keys),
            Self::Text(_) | Self::Code(_) | Self::Keyed(..) => None,
        }
    }
}

/// Resolved sizes stay on the same scale as their face.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    face: Type,
    size: f32,
    line: f32,
}

impl Content {
    fn metrics(&self) -> Metrics {
        self.metrics_with(crate::fonts::ui, crate::fonts::code)
    }

    // Separate callbacks let regressions exercise unequal UI/code settings
    // without changing the process-wide font settings of parallel tests.
    fn metrics_with(&self, interface: fn(f32) -> f32, code: fn(f32) -> f32) -> Metrics {
        match self {
            Self::Text(_) | Self::Keyed(..) => Metrics {
                face: self.face(),
                size: interface(14.0),
                line: interface(LINE),
            },
            Self::Code(_) => Metrics {
                face: self.face(),
                size: code(CODE_SIZE),
                line: code(CODE_LINE),
            },
        }
    }
}

/// A screen's tooltips: at most one shown.
#[derive(Debug, Clone, Default)]
pub struct Tooltips {
    state: Rc<RefCell<pending::State>>,
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
        self.attach(
            trigger,
            place,
            Content::Text(text.into()),
            Configuration::default(),
            if touch {
                crate::ControlSizing::Touch
            } else {
                crate::ControlSizing::Pointer
            },
        )
    }

    /// Configure this same tooltip/controller using the surface's pinned source
    /// profile. Existing `on` callers retain the immediate/top default.
    #[must_use]
    pub fn on_configured(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        text: impl Into<SharedString>,
        configuration: Configuration,
        sizing: crate::ControlSizing,
    ) -> Stateful<gpui::Div> {
        self.attach(
            trigger,
            place,
            Content::Text(text.into()),
            configuration,
            sizing,
        )
    }

    /// `on_configured` with the bound shortcut's chords after the words, as
    /// Paseo's `ToolbarButton` tooltip shows them; no chords draws the words
    /// alone.
    #[must_use]
    pub fn on_configured_keyed(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        text: impl Into<SharedString>,
        keys: Rc<[SharedString]>,
        configuration: Configuration,
        sizing: crate::ControlSizing,
    ) -> Stateful<gpui::Div> {
        self.attach(
            trigger,
            place,
            Content::Keyed(text.into(), keys),
            configuration,
            sizing,
        )
    }

    /// The same tooltip chrome and placement, with the supplied path/hash
    /// in the fixed-width face. GOAT's `StartTruncatedPath` uses a code
    /// tooltip for the full path; existing body-text callers stay unchanged.
    #[must_use]
    pub fn on_code(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        text: impl Into<SharedString>,
        touch: bool,
    ) -> Stateful<gpui::Div> {
        self.attach(
            trigger,
            place,
            Content::Code(text.into()),
            Configuration::default(),
            if touch {
                crate::ControlSizing::Touch
            } else {
                crate::ControlSizing::Pointer
            },
        )
    }

    fn attach(
        &self,
        trigger: Stateful<gpui::Div>,
        place: &Place,
        content: Content,
        configuration: Configuration,
        sizing: crate::ControlSizing,
    ) -> Stateful<gpui::Div> {
        if sizing == crate::ControlSizing::Touch {
            return trigger;
        }
        let lifetime = Rc::new(());
        self.state
            .borrow_mut()
            .refresh_trigger(place, &lifetime, &content, configuration);
        let hover_state = Rc::clone(&self.state);
        let hover_place = Rc::clone(place);
        let press_state = Rc::clone(&self.state);
        trigger
            .child(anchor_probe(place))
            .on_hover(move |hovered, window, cx| {
                if *hovered {
                    pending::entered(
                        &hover_state,
                        Tip {
                            content: content.clone(),
                            place: Rc::clone(&hover_place),
                            configuration,
                            trigger: Rc::downgrade(&lifetime),
                        },
                        window,
                        cx,
                    );
                } else {
                    hover_state.borrow_mut().leave(&hover_place);
                    window.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, move |_event, window, _cx| {
                press_state.borrow_mut().dismiss();
                window.refresh();
            })
    }

    /// The tooltip shown, over everything.
    #[must_use]
    pub fn render(&self, tokens: &Tokens) -> Option<AnyElement> {
        let tip = self.state.borrow().shown.clone()?;
        let _trigger = tip.trigger.upgrade()?;
        let bounds = tip.place.get()?;
        let neutral = tokens.neutral;
        let surface = match tokens.scheme {
            Scheme::Light => neutral.surface0,
            Scheme::Dark => neutral.surface2,
        };
        let (position, anchor, offset) = tip.configuration.placement(bounds);
        let metrics = tip.content.metrics();
        let card = div()
            .max_w(px(280.0))
            .py(px(4.0))
            .px(px(8.0))
            .rounded(px(12.0))
            .bg(surface)
            .border_1()
            .border_color(neutral.border_accent)
            .shadow(popup::shadow_md(tokens))
            .type_(metrics.face)
            .text_size(px(metrics.size))
            .line_height(px(metrics.line))
            .text_color(neutral.foreground)
            .map(|card| match tip.content.keys() {
                // Paseo's `tooltipRow`: the words, then the chords, gap 8.
                Some(keys) => card.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(div().children(lines(tip.content.text(), neutral.foreground)))
                        .child(crate::command::key_badges(keys, tokens)),
                ),
                None => card.children(lines(tip.content.text(), neutral.foreground)),
            });
        // Word 39: a tooltip arrives quickly (keyed by its words, so the
        // next trigger's plays again) and goes at once.
        let card = popup::appear(
            gpui::ElementId::NamedChild(
                std::sync::Arc::new(gpui::ElementId::Name(tip.content.text().clone())),
                "tip".into(),
            ),
            card,
            crate::motion::QUICK,
        );
        Some(
            deferred(
                anchored()
                    .position(position)
                    .anchor(anchor)
                    .offset(offset)
                    .snap_to_window_with_margin(px(8.0))
                    .child(card),
            )
            .with_priority(4)
            .into_any_element(),
        )
    }
}

/// A tooltip's words: its first line in the ink, the lines after it at
/// 65 % (the Codex app's two-line tooltips, "1.5x speed" over "More
/// usage"); a one-line tooltip is unchanged.
fn lines(text: &SharedString, ink: gpui::Hsla) -> Vec<gpui::Div> {
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            let colour = if index == 0 { ink } else { ink.opacity(SECOND) };
            div().text_color(colour).child(line.to_owned())
        })
        .collect()
}

/// The second line's opacity (the reference's 65 %).
const SECOND: f32 = 0.65;

/// Paseo's `fontSize.base * 1.4`.
const LINE: f32 = 19.6;

/// GOAT's code tooltip: text-[11px], leading-relaxed (11 × 1.625).
const CODE_SIZE: f32 = 11.0;
const CODE_LINE: f32 = 17.875;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_words_keep_the_body_face_and_show_chords_only_when_bound() {
        let keys: Rc<[SharedString]> = Rc::from(vec![SharedString::from("Ctrl+T")]);
        let keyed = Content::Keyed("New tab".into(), keys);
        assert_eq!(keyed.metrics(), Content::Text("New tab".into()).metrics());
        assert_eq!(keyed.keys().map(<[SharedString]>::len), Some(1));
        assert_eq!(keyed.text().as_ref(), "New tab");
        let unbound = Content::Keyed("New tab".into(), Rc::from(Vec::new()));
        assert!(unbound.keys().is_none());
        assert!(Content::Text("New tab".into()).keys().is_none());
    }

    #[test]
    fn a_code_tooltip_changes_only_the_face_of_its_exact_content() {
        let path: SharedString = "packages/긴-경로/src/sidebar-with-a-long-name.rs".into();
        let text = Content::Text(path.clone());
        let code = Content::Code(path.clone());
        assert_eq!(text.face(), Type::Body);
        assert_eq!(code.face(), Type::Mono);
        assert_eq!(text.text(), &path);
        assert_eq!(code.text(), &path);
    }
}

#[cfg(test)]
mod independent_scale_tests {
    use super::*;

    #[test]
    fn code_size_eighteen_does_not_take_its_line_height_from_interface_size_fourteen() {
        fn interface_fourteen(points: f32) -> f32 {
            points
        }
        fn code_eighteen(points: f32) -> f32 {
            points.mul_add(1.5, 0.0)
        }
        let code = Content::Code("a-long-unbroken-path.rs".into());
        let text = Content::Text("An ordinary tooltip.".into());
        assert_eq!(
            code.metrics_with(interface_fourteen, code_eighteen),
            Metrics {
                face: Type::Mono,
                size: 16.5,
                line: 26.8125,
            }
        );
        assert_eq!(
            text.metrics_with(interface_fourteen, code_eighteen),
            Metrics {
                face: Type::Body,
                size: 14.0,
                line: 19.6,
            }
        );
    }
}

#[cfg(test)]
mod profile_tests;
