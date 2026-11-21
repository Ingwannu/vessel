//! A page you fill in, inside a menu (TRACKER 792): Paseo's
//! `WorkspaceLabelCreatePage` frame (`workspace-labels/picker.tsx`) with its
//! `MenuTextField` and `MenuHint` (`components/ui/menu/menu-item.tsx`).
//!
//! [Decision Log]
//! - Purpose and intent: a menu page that takes words (Create label) drawn
//!   as the menu's own rows, not a dialog over it.
//! - Existing implementation and constraints: the kit menu draws rows from
//!   `MenuItem`s inside its own surface, bound to a `MenuState`; a submenu
//!   page is the screen's content placed by `submenu::flyout`. The kit had
//!   no field in a row's shape.
//! - Alternatives considered: a `MenuItem` kind holding a field (every
//!   menu would carry a field's focus and keys); the form field's own
//!   chrome (Paseo notes its 12 inset and 32 height land the text 4 off
//!   the rows' rail).
//! - Chosen approach: [`form_rows`] lays out Paseo's page from parts the
//!   menu already has: the field in a row's box (the same height, inset,
//!   padding and radius as `draw::row_body`, filled `surface2`, its border
//!   the accent while it holds the keys), the caller's body, the group
//!   separator, one action row drawn by `row_body` (its pending words and
//!   spinner from `MenuItem`'s status), and a muted hint. [`form_flyout`]
//!   puts them on the menu's popover surface; a phone's sheet is the
//!   caller's (a dialog sheet with its way back).
//! - Pros, cons and impact: the page's numbers follow the row's (change
//!   one, change both, as Paseo notes); the action row is not the menu's
//!   keyboard row: the field holds the keys and Enter acts.
//!
//! [Decision Log: the page on a phone]
//! - Purpose and intent: in Paseo's compact menu the page replaces the
//!   Labels page in the same sheet under `MenuSheetHeader` (a back chevron
//!   and the title, no close button), as Vessel's Labels page does.
//! - Existing implementation and constraints: the menu draws its root
//!   sheet and leaves a page's sheet to the screen (`surface::replaced`);
//!   the kit dialog's sheet has the adaptive-sheet header (an 18 arrow, a
//!   close button) and a 24 body inset, so a page drawn in it did not look
//!   like the Labels page it replaced.
//! - Alternatives considered: a header mode on the kit dialog (every
//!   dialog's header grows a menu case); the menu's own sheet surface (its
//!   pan takes the whole panel, which a page you type into cannot give).
//! - Chosen approach: [`form_sheet`] draws the menu sheet's look (surface0,
//!   the handle, the list's title row with its chevron leading back) over
//!   the sheet scrim, keeping the dialog's focus owner (`DialogState`) for
//!   its keys and its return of focus.
//! - Pros, cons and impact: the Labels page and its Create page share one
//!   header; the page does not pan down to close (the dialog's fitting
//!   sheet did not either): the scrim and Back close it.
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnimationExt as _, AnyElement, Context, Div, MouseButton, Pixels, SharedString, Stateful,
    Window, anchored, deferred, div, point, px, relative,
};

use super::MenuItem;
use super::draw::{ROW_LINE, Width, row_body};
use crate::TextField;
use crate::assets::Glyph;
use crate::dialog::{DialogState, sheet_handle};
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::theme::spacing;
use crate::widgets::TypeExt as _;

/// Where a row's words start from the page's edge (Paseo's
/// `menuRowContentInset`): the row's 4 margin, its 1 border and its 8
/// padding. A page's own content (a swatch row) sits on this rail.
pub const ROW_TEXT_INSET: f32 = 13.0;

/// What a page's action row runs when pressed.
pub type FormAct<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>;

/// What a form page holds, top to bottom: its field, its body, a
/// separator, its one action, and a hint under it.
pub struct FormPage<'a> {
    field: &'a TextField,
    body: Option<AnyElement>,
    action: MenuItem,
    hint: Option<SharedString>,
}

impl std::fmt::Debug for FormPage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormPage")
            .field("action", &self.action.label)
            .finish_non_exhaustive()
    }
}

impl<'a> FormPage<'a> {
    /// A page of `field` and the row that acts on it.
    #[must_use]
    pub fn new(field: &'a TextField, action: MenuItem) -> Self {
        Self {
            field,
            body: None,
            action,
            hint: None,
        }
    }

    /// Content between the field and the separator, on its own rail.
    #[must_use]
    pub fn with_body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }

    /// A muted line under the action (Paseo's `MenuHint`: a failure).
    #[must_use]
    pub fn with_hint(mut self, hint: Option<SharedString>) -> Self {
        self.hint = hint;
        self
    }
}

/// The page's rows without a surface: for [`form_flyout`] or a sheet the
/// caller draws. `act` runs when the action row is pressed (only while it
/// is selectable); `narrow` is a phone's 44-point rows.
pub fn form_rows<V: 'static>(
    page: FormPage<'_>,
    act: FormAct<V>,
    narrow: bool,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Div {
    let surfaces = Surfaces::of(tokens);
    let neutral = tokens.neutral;
    // Paseo's `field`: `item`'s box, filled as a hovered row.
    let field = div()
        .debug_selector(|| "menu-text-field".to_owned())
        .flex()
        .items_center()
        .min_h(px(popup::row_height(narrow)))
        .mx(px(spacing::S1))
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .rounded(px(popup::ROW_RADIUS))
        .border_1()
        .border_color(if page.field.holds_keys(window, cx) {
            neutral.border_accent
        } else {
            neutral.border
        })
        .bg(neutral.surface2)
        .child(page.field.menu_line(tokens, ROW_LINE, cx));
    let paint = super::item::paint(&page.action, false, &surfaces);
    let mut action = row_body(0, &page.action, paint, narrow, tokens);
    if page.action.selectable() {
        let wash = surfaces.surface2;
        action = action
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .active(move |style| style.bg(wash))
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                act(view, window, cx);
            }));
    }
    let hint = page.hint.map(|hint| {
        div()
            .debug_selector(|| "menu-page-hint".to_owned())
            .px(px(spacing::S3))
            .py(px(spacing::S2))
            .type_(Type::Meta)
            .text_color(surfaces.foreground_muted)
            .truncate()
            .child(hint)
    });
    div()
        .flex()
        .flex_col()
        .child(field)
        .children(page.body)
        .child(
            div()
                .my(px(spacing::S1))
                .h(px(1.0))
                .bg(surfaces.border_accent),
        )
        .child(action)
        .children(hint)
}

/// The page's rows on the menu's popover surface, `width` wide: the panel
/// to hand to `submenu::flyout`.
#[must_use]
pub fn form_flyout(id: &'static str, rows: Div, width: Pixels, tokens: &Tokens) -> Stateful<Div> {
    let surfaces = Surfaces::of(tokens);
    let panel = div()
        .id(id)
        .cursor_default()
        .flex()
        .flex_col()
        .py(px(spacing::S1))
        .child(rows);
    super::surface::popover(panel, Width::Fixed(width), &surfaces, tokens)
}

/// What a phone's form sheet says and does.
pub struct FormSheet<V> {
    /// Its title (the page's name).
    pub title: SharedString,
    /// The header's way back, named for assistive technology.
    pub back_label: SharedString,
    /// The header's press: back to the page it replaced.
    pub back: FormAct<V>,
    /// A press on the dimmed page: the whole menu closes.
    pub dismiss: FormAct<V>,
}

impl<V> std::fmt::Debug for FormSheet<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormSheet")
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

/// The page's rows in a phone's sheet: the menu sheet's look, its title
/// row leading back, over the sheet scrim. `state` owns its focus: call
/// its `focus` when the page opens and `dismissed` when it goes.
pub fn form_sheet<V: 'static>(
    id: &'static str,
    state: &DialogState,
    sheet: FormSheet<V>,
    rows: Div,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let surfaces = Surfaces::of(tokens);
    let neutral = tokens.neutral;
    let FormSheet {
        title,
        back_label,
        back,
        dismiss,
    } = sheet;
    // The list's sheet title row (`picker::rows::title_row`), the menu's.
    let header = div()
        .id("form-sheet-back")
        .debug_selector(|| "form-sheet-back".to_owned())
        .role(gpui::Role::Button)
        .aria_label(back_label)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(neutral.border)
        .type_(Type::Label)
        .text_color(surfaces.foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(neutral.surface1))
        .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
            back(view, window, cx);
            cx.notify();
        }))
        .child(crate::picker::glyph(
            Glyph::ChevronLeft,
            16.0,
            neutral.foreground_muted,
        ))
        .child(div().flex_1().min_w_0().truncate().child(title));
    // As the dialog's card: a press inside neither closes (the panel
    // occludes the scrim) nor takes the field's keys (the default focus
    // move is prevented; the field focuses itself first).
    let panel = div()
        .id(id)
        .track_focus(state.focus_handle())
        .key_context(popup::KEY_CONTEXT)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
            window.prevent_default();
        })
        .cursor_default()
        .flex()
        .flex_col()
        .w_full()
        .max_h(relative(0.8))
        .overflow_hidden()
        .bg(surfaces.surface0)
        .border_1()
        .border_color(neutral.border)
        .rounded_t(px(popup::SHEET_RADIUS))
        .pb(px(spacing::S4))
        .child(sheet_handle(neutral.foreground_extra_muted))
        .child(header)
        .child(div().pt(px(spacing::S1)).child(rows));
    let size = window.viewport_size();
    let shade = popup::sheet_scrim();
    let scrim = div()
        .id("form-sheet-scrim")
        .w(size.width)
        .h(size.height)
        .flex()
        .justify_center()
        .items_end()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                dismiss(view, window, cx);
                cx.notify();
            }),
        )
        .child(popup::appear(
            "form-sheet-appear",
            panel,
            crate::motion::STANDARD,
        ))
        .with_animation(
            "form-sheet-scrim-appear",
            crate::motion::transition(crate::motion::STANDARD),
            move |scrim, done| scrim.bg(shade.opacity(done)),
        );
    // The dialog's layer: the page took the dialog's place.
    deferred(anchored().position(point(px(0.0), px(0.0))).child(scrim))
        .with_priority(2)
        .into_any_element()
}

#[cfg(test)]
#[path = "page_tests.rs"]
mod tests;
