//! Source small icon chrome geometry and caller-owned state, with no area sizes.
use crate::{ButtonState, ControlSizing, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Pixels, Svg, px, svg};

const POINTER_FRAME: Pixels = px(20.0);
const COMPACT_FRAME: Pixels = px(32.0);
const FIXED_GLYPH: Pixels = px(14.0);
const COMPACT_GLYPH: Pixels = px(18.0);
const DISABLED_OPACITY: f32 = 0.5;
const ENABLED_OPACITY: f32 = 1.0;

/// An owner's selected pane/action, independent from a temporarily open menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Ordinary idle action.
    Unselected,
    /// Its content or pane is currently selected.
    Selected,
}
/// A menu owner's current disclosure state; the toolbar allocates no menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disclosure {
    /// No open menu highlight.
    Closed,
    /// Retain the highlight while the owner's menu is open.
    Open,
}
/// Input facts supplied once at the app edge. Ordinary Button sizes are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolbarConfig {
    sizing: ControlSizing,
    availability: ButtonState,
    selection: Selection,
    disclosure: Disclosure,
}
impl ToolbarConfig {
    /// Create an unselected, closed small toolbar action for the actual layout.
    #[must_use]
    pub const fn new(sizing: ControlSizing, availability: ButtonState) -> Self {
        Self {
            sizing,
            availability,
            selection: Selection::Unselected,
            disclosure: Disclosure::Closed,
        }
    }
    /// Reflect the owner's current selection without allocating local state.
    #[must_use]
    pub const fn selected(mut self, selection: Selection) -> Self {
        self.selection = selection;
        self
    }
    /// Reflect the owner's current menu state, including keyboard-opened menus.
    #[must_use]
    pub const fn disclosure(mut self, disclosure: Disclosure) -> Self {
        self.disclosure = disclosure;
        self
    }
    /// The normal handler's availability; appearance alone never authorizes it.
    #[must_use]
    pub const fn availability(self) -> ButtonState {
        self.availability
    }
    pub(super) const fn sizing(self) -> ControlSizing {
        self.sizing
    }
    pub(super) const fn frame(self) -> Pixels {
        match self.sizing {
            ControlSizing::Pointer => POINTER_FRAME,
            ControlSizing::Touch => COMPACT_FRAME,
        }
    }
    pub(super) const fn highlighted(self) -> bool {
        matches!(self.selection, Selection::Selected) || matches!(self.disclosure, Disclosure::Open)
    }
    pub(super) const fn opacity(self) -> f32 {
        match self.availability {
            ButtonState::Enabled => ENABLED_OPACITY,
            ButtonState::Disabled => DISABLED_OPACITY,
        }
    }
}
/// Source icon color mapping, with no per-area color literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarInk {
    /// Source foreground icon mapping.
    Foreground,
    /// Source muted icon mapping.
    Muted,
    /// Source workspace rail's extra-muted icon mapping.
    ExtraMuted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlyphSizing {
    Fixed,
    Adaptive,
}
/// One shared vector and its explicit source mapping. Workspace rail icons are
/// fixed14; the source helper alone adapts to18 in the compact layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolbarIcon {
    glyph: Glyph,
    sizing: GlyphSizing,
    ink: ToolbarInk,
}
impl ToolbarIcon {
    /// The workspace rail's explicit14-point source icon.
    #[must_use]
    pub const fn new(glyph: Glyph, ink: ToolbarInk) -> Self {
        Self {
            glyph,
            sizing: GlyphSizing::Fixed,
            ink,
        }
    }
    /// Opt into the source helper's14/18 layout mapping, not frame scaling.
    #[must_use]
    pub const fn adaptive(mut self) -> Self {
        self.sizing = GlyphSizing::Adaptive;
        self
    }
    pub(super) const fn size(self, sizing: ControlSizing) -> Pixels {
        match (self.sizing, sizing) {
            (GlyphSizing::Adaptive, ControlSizing::Touch) => COMPACT_GLYPH,
            (GlyphSizing::Fixed, ControlSizing::Pointer | ControlSizing::Touch)
            | (GlyphSizing::Adaptive, ControlSizing::Pointer) => FIXED_GLYPH,
        }
    }
    pub(super) fn render(self, sizing: ControlSizing, tokens: &Tokens) -> Svg {
        let ink = match self.ink {
            ToolbarInk::Foreground => tokens.neutral.foreground,
            ToolbarInk::Muted => tokens.neutral.foreground_muted,
            ToolbarInk::ExtraMuted => tokens.neutral.foreground_extra_muted,
        };
        svg()
            .path(self.glyph.path())
            .size(self.size(sizing))
            .flex_none()
            .text_color(ink)
    }
}
