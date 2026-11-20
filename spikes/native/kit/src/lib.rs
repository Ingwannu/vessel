//! Vessel's own UI kit for the native app: the design system's tokens and
//! type, and the controls the screens are written against (a button, a
//! text field that takes input methods, a long list, a status dot).
//!
//! [Decision Log]
//! - Purpose: the owner's rule for milestone 12 is that Zed's framework is
//!   a reference, not something taken whole (ADR 0035). Screens therefore
//!   name this crate's controls and tokens, never `gpui-component`'s, so
//!   what is under the kit can change (a patched GPUI, other layers)
//!   without rewriting a screen.
//! - Constraint: the kit is drawn with GPUI today, and a screen is still a
//!   GPUI view: layout (`div`, flex, padding) goes through GPUI's element
//!   builders, which the kit re-exports in [`ui`] so that a screen's
//!   imports come from here. That is where this interface stops being
//!   ours for now, and the report says so (question 9).
//! - Alternatives: screens on `gpui-component` directly (fastest, and the
//!   whole app would then be written against a third party's API); a kit
//!   with its own element tree (a layout engine of our own: later, if the
//!   layers below GPUI are ever replaced).
//! - Choice: tokens, type and controls are ours; layout is GPUI's,
//!   re-exported. `gpui-component` is used inside [`field`] only, for its
//!   text input (selection, clipboard, input methods), hidden behind
//!   [`TextField`].

pub mod agent_mark;
pub mod alert;
pub mod assets;
pub mod autocomplete;
pub mod badge;
pub mod breakpoint;
pub mod card_button;
pub mod chat_markdown;
pub mod chrome;
pub mod code;
pub mod copy;
pub mod dialog;
pub mod field;
pub mod fonts;
pub mod frame_log;
pub mod menu;
pub mod motion;
pub mod output;
pub mod picker;
pub mod popup;
pub mod qr;

pub mod screen_header;
pub mod segmented;
pub mod select;
pub mod settings;
pub mod sheet;
pub mod skeleton;
pub mod slider;
pub mod tabs;
pub mod theme;
pub mod toast;
pub mod tooltip;
pub mod virtual_list;
mod widgets;

use gpui::{App, AppContext as _, Global, Window, WindowAppearance};

pub use assets::Assets;
pub use card_button::{CardLook, CardState, card_button};
pub use chat_markdown::{FindMarks, FindPalette, chat_markdown};
pub use code::CodeTokens;
pub use copy::{CopyLook, copy_button};
pub use dialog::{DialogContent, DialogSize, DialogState, dialog};
pub use field::{SendOn, TextField, composer_card};
pub use fonts::Type;
pub use menu::{MenuHandlers, MenuItem, MenuState, Tone, menu};
pub use output::{
    Line as OutputLine, Start, Tone as LineTone, diff_line, lines_box, output_box, version_of,
};
pub use picker::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, PillParts, SendKind, Side,
    anchor_probe, combobox, glyph, initial_of, initial_tile, pill, pill_parts, send_button,
};
pub use screen_header::{compact_header, screen_header};
pub use sheet::{Edge, sheet};
pub use theme::{Scheme, ThemeId, Tokens};
pub use toast::{ToastLook, toast, toasts};
pub use tooltip::Tooltips;
pub use virtual_list::{Reveal, VirtualList};
pub use widgets::{
    ButtonKind, ButtonState, TypeExt, button, button_with_kind, choice, dot, kicker, mark,
    mark_mono, markdown, outline_button,
};

/// What a screen needs of GPUI's layout and view types, re-exported so
/// screens import from the kit (see the module's decision log).
pub mod ui {
    pub use gpui::prelude::*;
    pub use gpui::{
        AnyElement, App, AsyncApp, Bounds, ClickEvent, ClipboardItem, Context, Div, DragMoveEvent,
        ElementId, Entity, HighlightStyle, Hsla, KeyDownEvent, ListAlignment, ListState,
        MouseButton, MouseDownEvent, Pixels, Point, SharedString, Stateful, StyledText,
        Subscription, WeakEntity, Window, canvas, div, list, point, px, relative, rgb, rgba, svg,
        uniform_list,
    };
}

/// The theme in use: the tokens of one scheme.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// The tokens.
    pub tokens: Tokens,
}

impl Global for Theme {}

/// Which scheme the app shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeChoice {
    /// Follow the system's appearance.
    System,
    /// Always this one.
    Fixed(Scheme),
}

/// The tokens in use.
#[must_use]
pub fn tokens(cx: &App) -> Tokens {
    cx.try_global::<Theme>()
        .map_or_else(|| Tokens::of(Scheme::Light), |theme| theme.tokens)
}

/// Prepare the kit in `cx`: the faces, the component layer under the
/// text field, and the theme for `choice` (the window's appearance when
/// following the system).
///
/// # Errors
/// The faces could not be registered.
pub fn init(cx: &mut App, choice: SchemeChoice) -> Result<(), fonts::FontError> {
    // The component layer first, the faces after: its `init` asks GPUI
    // which face `.SystemUIFont` lands on once any face is installed, and
    // in a browser (no system faces) that lookup panics when only ours are
    // there. Before them it skips the lookup; the theme then names Geist
    // (`set_scheme`), so the order changes nothing on the desktop
    // (docs/plans/backend-m12-browser.md, question 1).
    gpui_component::init(cx);
    fonts::register(cx)?;
    let scheme = match choice {
        SchemeChoice::Fixed(scheme) => scheme,
        SchemeChoice::System => scheme_of(cx.window_appearance()),
    };
    set_scheme(cx, scheme);
    Ok(())
}

/// The scheme that matches a window's appearance.
#[must_use]
pub const fn scheme_of(appearance: WindowAppearance) -> Scheme {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => Scheme::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Scheme::Dark,
    }
}

/// The type Settings applies in one turn: the two scaled sizes, the
/// absolute content size, and the two family names (empty means the
/// embedded face).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeChoice {
    /// The interface size, in points.
    pub interface: u8,
    /// The code size, in points.
    pub code: u8,
    /// Chat and Markdown text, in points. Not scaled by [`Self::interface`].
    pub content: u8,
    /// The interface face. Empty means Geist.
    pub interface_family: String,
    /// The code face. Empty means Geist Mono.
    pub code_family: String,
}

/// Draw text as `choice` says. The windows are drawn again when any of
/// it changed. Content size is the component layer's prose size; a
/// family leads the embedded faces.
pub fn set_type_choice(cx: &mut App, choice: TypeChoice) {
    let sizes = fonts::set_sizes(choice.interface, choice.code);
    let content = fonts::set_content(choice.content);
    let faces = fonts::set_families(&choice.interface_family, &choice.code_family);
    if sizes || content || faces {
        refresh_type(cx);
    }
}

/// Draw text at `interface` and `code` points (Settings › Appearance):
/// every step of the type scale, and the component layer's code size
/// with them. The content size and the families stay as they are. The
/// windows are drawn again when either size changed.
pub fn set_text_sizes(cx: &mut App, interface: u8, code: u8) {
    if fonts::set_sizes(interface, code) {
        refresh_type(cx);
    }
}

/// The component layer's type from the sizes and families now in force.
fn refresh_type(cx: &mut App) {
    let tokens = tokens(cx);
    field::apply_tokens(cx, &tokens);
    cx.refresh_windows();
}

/// Switch to one palette. The five tints share Dark's scheme and differ
/// in the tokens, so the palette, not the scheme, decides. The windows
/// are drawn again when the tokens change.
pub fn set_theme(cx: &mut App, id: ThemeId) {
    let next = Tokens::of_theme(id);
    if cx
        .try_global::<Theme>()
        .is_some_and(|theme| theme.tokens == next)
    {
        return;
    }
    cx.set_global(Theme { tokens: next });
    field::apply_tokens(cx, &next);
    cx.refresh_windows();
}

/// Switch the theme, and the component layer's colours with it.
pub fn set_scheme(cx: &mut App, scheme: Scheme) {
    set_theme(
        cx,
        match scheme {
            Scheme::Light => ThemeId::Light,
            Scheme::Dark => ThemeId::Dark,
        },
    );
}

/// Wrap a screen's root view as the component layer needs it (its
/// overlays and focus handling live in a root of its own).
pub fn root<V: 'static + gpui::Render>(
    view: gpui::Entity<V>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<gpui_component::Root> {
    cx.new(|cx| gpui_component::Root::new(view, window, cx))
}
