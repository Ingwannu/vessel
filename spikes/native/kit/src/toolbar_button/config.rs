//! Source small icon chrome geometry and caller-owned state, with no area sizes.
use crate::{ButtonState, ControlSizing, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Pixels, Svg, px, svg};

const POINTER_FRAME: Pixels = px(20.0);
/// Paseo's large chrome on a wide window: `HEADER_CONTROL_HEIGHT`.
const LARGE_POINTER_FRAME: Pixels = px(26.0);
const COMPACT_FRAME: Pixels = px(32.0);
const FIXED_GLYPH: Pixels = crate::loading::Points::FOURTEEN.pixels();
const COMPACT_GLYPH: Pixels = crate::loading::Points::EIGHTEEN.pixels();
/// Paseo's `iconButtonChromeGlyphSize("large")`: `ICON_SIZE.md`.
const LARGE_GLYPH: Pixels = crate::loading::Points::SIXTEEN.pixels();
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
    chrome: Chrome,
}
/// Paseo's icon button chrome sizes (`icon-button-chrome.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chrome {
    /// The toolbar's own: 20 or 32, a 14 (or adaptive 18) glyph.
    Small,
    /// `size: "large"`: 26 on a wide window, 32 on a phone, a 16 glyph.
    Large,
    /// The header island: the large chrome, round.
    Island,
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
            chrome: Chrome::Small,
        }
    }
    /// Paseo's large chrome (`iconButtonChromeStyle({ size: "large" })`,
    /// TRACKER 795: the wide workspace "…"): 26 on a pointer, 32 on touch,
    /// a 16 glyph, square corners.
    #[must_use]
    pub const fn large(mut self) -> Self {
        self.chrome = Chrome::Large;
        self
    }
    /// Paseo's header island (`IconButtonChromeProvider value="header-island"`,
    /// TRACKER 789): its icon actions take the large chrome, a 16 glyph, and a
    /// round hitbox and highlight (32 on touch, Paseo's
    /// `COMPACT_LARGE_ICON_BUTTON_SIZE`); every other toolbar keeps its 14
    /// glyph and its square corners.
    #[must_use]
    pub const fn header_island(mut self) -> Self {
        self.chrome = Chrome::Island;
        self
    }
    pub(crate) const fn island(self) -> bool {
        matches!(self.chrome, Chrome::Island)
    }
    /// The large chrome's 16 glyph (large and island).
    pub(crate) const fn large_glyph(self) -> bool {
        !matches!(self.chrome, Chrome::Small)
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
    pub(crate) const fn sizing(self) -> ControlSizing {
        self.sizing
    }
    pub(super) const fn disabled(mut self) -> Self {
        self.availability = ButtonState::Disabled;
        self
    }
    pub(crate) const fn frame(self) -> Pixels {
        match (self.chrome, self.sizing) {
            (Chrome::Small, ControlSizing::Pointer) => POINTER_FRAME,
            (Chrome::Large | Chrome::Island, ControlSizing::Pointer) => LARGE_POINTER_FRAME,
            (Chrome::Small | Chrome::Large | Chrome::Island, ControlSizing::Touch) => COMPACT_FRAME,
        }
    }
    pub(crate) const fn highlighted(self) -> bool {
        matches!(self.selection, Selection::Selected) || matches!(self.disclosure, Disclosure::Open)
    }
    pub(crate) const fn opacity(self) -> f32 {
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
impl ToolbarInk {
    pub(super) fn colour(self, tokens: &Tokens) -> gpui::Hsla {
        match self {
            Self::Foreground => tokens.neutral.foreground,
            Self::Muted => tokens.neutral.foreground_muted,
            Self::ExtraMuted => tokens.neutral.foreground_extra_muted,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlyphSizing {
    Fixed,
    Adaptive,
    /// The header island's large chrome: 16 at both sizings.
    Large,
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
    /// The header island's 16 (`ToolbarConfig::header_island`).
    pub(super) const fn large(mut self) -> Self {
        self.sizing = GlyphSizing::Large;
        self
    }
    pub(super) const fn size(self, sizing: ControlSizing) -> Pixels {
        match (self.sizing, sizing) {
            (GlyphSizing::Large, ControlSizing::Pointer | ControlSizing::Touch) => LARGE_GLYPH,
            (GlyphSizing::Adaptive, ControlSizing::Touch) => COMPACT_GLYPH,
            (GlyphSizing::Fixed, ControlSizing::Pointer | ControlSizing::Touch)
            | (GlyphSizing::Adaptive, ControlSizing::Pointer) => FIXED_GLYPH,
        }
    }
    pub(crate) fn render(self, sizing: ControlSizing, tokens: &Tokens) -> Svg {
        svg()
            .path(self.glyph.path())
            .size(self.size(sizing))
            .flex_none()
            .text_color(self.ink.colour(tokens))
    }
}
