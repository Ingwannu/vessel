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
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, Pixels, SharedString, Stateful, Window, div, px};

use super::MenuItem;
use super::draw::{ROW_LINE, Width, row_body};
use crate::TextField;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::theme::spacing;
use crate::widgets::TypeExt as _;

/// Where a row's words start from the page's edge (Paseo's
/// `menuRowContentInset`): the row's 4 margin, its 1 border and its 8
/// padding. A page's own content (a swatch row) sits on this rail.
pub const ROW_TEXT_INSET: f32 = 13.0;

/// What a form page holds, top to bottom: its field, its body, a
/// separator, its one action, and a hint under it.
pub struct FormPage<'a> {
    field: &'a TextField,
    body: Option<AnyElement>,
    action: MenuItem,
    hint: Option<SharedString>,
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
    act: Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>,
    narrow: bool,
    tokens: &Tokens,
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
        .border_color(if page.field.holds_keys() {
            neutral.border_accent
        } else {
            neutral.border
        })
        .bg(neutral.surface2)
        .child(page.field.menu_line(tokens, ROW_LINE, cx));
    let paint = super::item::paint(&page.action, false, &surfaces);
    let mut action = row_body(0, &page.action, paint, narrow);
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

#[cfg(test)]
#[path = "page_tests.rs"]
mod tests;
