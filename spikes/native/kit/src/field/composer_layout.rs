//! Height and font policy over the existing composer Entity, never another editor.
//! Readonly remains the shared Editability setter; this renderer never resets it.
//!
//! Reference input.tsx:218-219,1046-1049 caps compact input at160 and desktop at
//! half the window (at least160). Fullscreen fixes the same engine to its slot.

use std::num::NonZeroUsize;

use gpui::prelude::*;
use gpui::{AnyElement, App, Pixels, div, px};
use gpui_base::input::Textarea;

use super::{Kind, MultilineStyle, SendOn, TextField, styles::project};
use crate::{Tokens, Type, TypeExt as _, fonts};

/// Input mode changes the family, while content size still follows interface text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerFont {
    /// Agent messages.
    Chat,
    /// Terminal argv entry.
    Terminal,
}

/// The host supplies its existing responsive classification and current window height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComposerViewport {
    /// Touch layout submits by button only.
    Compact,
    /// Desktop input can grow through half this viewport.
    Desktop(Pixels),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightMode {
    Grow,
    Fill,
}

/// A presentation snapshot; no text, selection, focus or readonly state is copied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComposerLayout {
    height: Pixels,
    mode: HeightMode,
    font: ComposerFont,
    send: SendOn,
}

const DEFAULT_MAX_HEIGHT: f32 = 160.0;
const VIEWPORT_RATIO: f32 = 0.5;
const CONTENT_SIZE: f32 = 15.0;
const CONTENT_LINE: f32 = 21.0;
const MIN_ROWS: usize = 1;
/// A viewport cannot allocate unbounded editor rows, even with invalid geometry.
const MAX_ROWS: u16 = u16::MAX;

impl ComposerLayout {
    /// Compact policy wins over a persisted Enter/ModEnter preference.
    #[must_use]
    pub fn inline(viewport: ComposerViewport, font: ComposerFont, shortcut: SendOn) -> Self {
        let (height, send) = match viewport {
            ComposerViewport::Compact => (DEFAULT_MAX_HEIGHT, SendOn::ButtonOnly),
            ComposerViewport::Desktop(height) => {
                let height = f32::from(height);
                let height = if height.is_finite() && height > 0.0 {
                    height
                        .mul_add(VIEWPORT_RATIO, 0.0)
                        .floor()
                        .max(DEFAULT_MAX_HEIGHT)
                } else {
                    DEFAULT_MAX_HEIGHT
                };
                (height, shortcut)
            }
        };
        Self {
            height: px(height),
            mode: HeightMode::Grow,
            font,
            send,
        }
    }

    /// A fullscreen compact editor fills its available slot and keeps Enter as text.
    #[must_use]
    pub fn fullscreen(available: Pixels, font: ComposerFont) -> Self {
        let available = f32::from(available);
        Self {
            height: px(if available.is_finite() {
                available.max(0.0)
            } else {
                0.0
            }),
            mode: HeightMode::Fill,
            font,
            send: SendOn::ButtonOnly,
        }
    }

    pub(super) fn legacy(send: SendOn) -> Self {
        let count = u16::try_from(super::multiline::COMPOSER_LINES).unwrap_or(MAX_ROWS);
        Self {
            height: px(f32::from(count).mul_add(fonts::ui(CONTENT_LINE), 0.0)),
            mode: HeightMode::Grow,
            font: ComposerFont::Chat,
            send,
        }
    }
}

/// The retained height request prevents a notify/re-render loop on unchanged frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rows {
    min: usize,
    max: usize,
}

/// Measured wrapped content lines; viewport rows and newline counts are different units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedRows(NonZeroUsize);
impl RenderedRows {
    /// Read only after the editing engine has measured its display map.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

impl TextField {
    /// Editing admission comes from the same engine, including shared readonly/disabled state.
    #[must_use]
    pub fn composer_is_editable(&self, cx: &App) -> bool {
        match &self.kind {
            Kind::Line(state) => state.read(cx).presentation().is_editable(),
            Kind::Lines(state, _) => state.read(cx).presentation().is_editable(),
        }
    }

    /// None before layout or for another field kind; wrapping comes from the same engine.
    #[must_use]
    pub fn rendered_composer_rows(&self, cx: &App) -> Option<RenderedRows> {
        match &self.kind {
            Kind::Lines(state, MultilineStyle::Composer) => {
                NonZeroUsize::new(state.read(cx).display_map().wrap_row_count()).map(RenderedRows)
            }
            Kind::Line(_) | Kind::Lines(_, MultilineStyle::Form) => None,
        }
    }

    /// One renderer for docked/fullscreen chat and terminal input; same Entity and hooks.
    #[must_use]
    pub fn composer_element(
        &self,
        layout: ComposerLayout,
        tokens: &Tokens,
        cx: &mut App,
    ) -> AnyElement {
        let Kind::Lines(state, MultilineStyle::Composer) = &self.kind else {
            return self.element(tokens, false, cx);
        };
        let count = fitted_rows(layout.height, px(fonts::ui(CONTENT_LINE)));
        let rows = Rows {
            min: match layout.mode {
                HeightMode::Grow => MIN_ROWS,
                HeightMode::Fill => count,
            },
            max: count,
        };
        if self.composer_rows.get() != Some(rows) {
            self.composer_rows.set(Some(rows));
            state.update(cx, |state, cx| state.set_auto_grow(rows.min, rows.max, cx));
        }
        self.send.set(layout.send);
        project(state, tokens, tokens.neutral.surface4, cx);
        div()
            .w_full()
            .type_(Type::Body)
            .font_family(match layout.font {
                ComposerFont::Chat => fonts::SANS,
                ComposerFont::Terminal => fonts::MONO,
            })
            .text_size(px(fonts::ui(CONTENT_SIZE)))
            .line_height(px(fonts::ui(CONTENT_LINE)))
            .text_color(tokens.neutral.foreground)
            // The same engine emits PressEnter and propagates. The existing
            // submit listener already sent or inserted a line; native fallback
            // must not insert another, just as for the shared form renderer.
            .on_action(|_: &gpui_base::input::Enter, _window, cx| cx.stop_propagation())
            .map(|element| match layout.mode {
                HeightMode::Grow => element.min_h(px(super::COMPOSER_MIN)).max_h(layout.height),
                HeightMode::Fill => element.h(layout.height).min_h(px(0.0)).max_h(layout.height),
            })
            .child(Textarea::new(state))
            .into_any_element()
    }
}

// A bounded binary search avoids lossy float-to-integer casts or another numeric
// dependency. u16 -> f32 is exact; at most16 comparisons find the whole-row fit.
fn fitted_rows(height: Pixels, line: Pixels) -> usize {
    let height = f32::from(height);
    let line = f32::from(line);
    if !height.is_finite() || !line.is_finite() || line <= 0.0 {
        return MIN_ROWS;
    }
    let (mut low, mut high) = (0_u16, MAX_ROWS);
    while low < high {
        let middle = low.saturating_add(high.saturating_sub(low).div_ceil(2));
        if f32::from(middle).mul_add(line, 0.0) <= height {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    usize::from(low).max(MIN_ROWS)
}

#[cfg(test)]
mod tests;
