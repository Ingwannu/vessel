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
pub mod attachment;
pub mod autocomplete;
pub mod badge;
pub mod breakpoint;
pub mod card_button;
pub mod chat_markdown;
pub mod chrome;
pub mod code;
pub mod code_editor;
pub mod command;
pub mod control_sizing;
pub mod copy;
pub mod dialog;
pub mod drag;
pub mod external_link;
pub mod field;
pub mod focus;
pub mod fonts;
pub mod form_summary;
pub mod frame;
pub mod frame_log;
pub mod host_rename;
pub mod loading;
pub mod menu;
pub mod motion;
pub mod output;
pub mod picker;
pub mod popover;
pub mod popup;
pub mod qr;
pub mod resize;

pub mod screen_header;
pub mod scroll_viewport;
pub mod segmented;
pub mod select;
pub mod settings;
pub mod sheet;
pub mod sidebar_callout;
pub mod sidebar_header_row;
pub mod sidebar_workspace_row;
pub mod skeleton;
pub mod slider;
pub mod split_button;
pub mod tabs;
pub mod terminal_grid;
pub mod theme;
pub mod toast;
pub mod toolbar_button;
pub mod toolbar_label;
pub mod tooltip;
pub mod vector;
pub mod virtual_list;
mod widgets;
pub mod window_glyph;
pub mod zoom;

use gpui::{App, AppContext as _, Global, IntoElement as _, Styled as _, Window, WindowAppearance};

pub use assets::Assets;
pub use card_button::{CardLook, CardState, card_button};
pub use chat_markdown::{FindMarks, FindPalette, chat_markdown};
pub use code::CodeTokens;
pub use control_sizing::ControlSizing;
pub use copy::{CopyLook, copy_button};
pub use dialog::{DialogContent, DialogSize, DialogState, dialog};
pub use field::{Editability, SearchField, SendOn, TextField, composer_card};
pub use fonts::Type;
pub use host_rename::host_rename_button;
pub use menu::{MenuHandlers, MenuItem, MenuState, Tone, menu, menu_right};
pub use output::{
    Line as OutputLine, Start, Tone as LineTone, diff_line, lines_box, output_box, version_of,
};
pub use picker::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, PillParts, SendKind, Side,
    anchor_probe, combobox, glyph, initial_of, initial_tile, pill, pill_parts, send_button,
};
pub use screen_header::{compact_header, screen_header};
pub use scroll_viewport::{ScrollState, ScrollViewport, SectionIndex};
pub use sheet::{Edge, sheet};
pub use theme::{Palette, Scheme, Tokens};
pub use toast::{
    ToastContent, ToastDismiss, ToastHandlers, ToastHover, ToastHovered, ToastLook, toast,
    toast_with, toasts,
};
pub use tooltip::Tooltips;
pub use virtual_list::{Reveal, VirtualList};
pub use widgets::{
    Button, ButtonConfig, ButtonContent, ButtonDesign, ButtonIcon, ButtonKind, ButtonSize,
    ButtonState, CircleConfig, CircleTone, TypeExt, button, button_control, button_with_content,
    button_with_design, button_with_kind, button_with_kind_at, choice, dot, kicker, mark,
    mark_mono, markdown, outline_button, outline_button_with_glyph,
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

/// The palette in use, shared by every control and the component layer.
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

/// `view` filling its parent's box, its last frame reused until its own
/// entity is notified (GPUI's cached view; the app's `region.rs` says why).
/// A cached view is not measured from its content: its parent gives it
/// its size.
#[must_use]
pub fn cached_view<V: gpui::Render>(view: &gpui::Entity<V>) -> gpui::AnyElement {
    view.clone()
        .cached(gpui::StyleRefinement::default().size_full())
        .into_any_element()
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

/// Draw text at `interface` and `code` points (Settings › Appearance):
/// every step of the type scale, and the component layer's text (the
/// replies' Markdown) with them; the windows are drawn again when either
/// changed.
pub fn set_text_sizes(cx: &mut App, interface: u8, code: u8) {
    if fonts::set_sizes(interface, code) {
        let tokens = tokens(cx);
        field::apply_tokens(cx, &tokens);
        cx.refresh_windows();
    }
}

/// Switch the theme, and the component layer's colours with it.
pub fn set_scheme(cx: &mut App, scheme: Scheme) {
    set_palette(cx, Palette::from(scheme));
}

/// Apply one concrete palette to controls and the component layer together,
/// refreshing every window even when the light/dark scheme is unchanged.
pub fn set_palette(cx: &mut App, palette: Palette) {
    let tokens = Tokens::palette(palette);
    cx.set_global(Theme { tokens });
    field::apply_tokens(cx, &tokens);
    cx.refresh_windows();
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

mod root_owner;
pub use root_owner::root_contains_view;
