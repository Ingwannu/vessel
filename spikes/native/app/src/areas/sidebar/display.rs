//! The sidebar's Display menu: its trigger at the list's heading and the
//! menu itself, Paseo's (`display-preferences/menu.tsx`: a 14-point
//! Settings2 mark at the heading's right, a 232-wide surface aligned to the
//! trigger's end). What each row is and does is the view's
//! (`vessel_app_view::sidebar::display`); this file draws it with the
//! kit's list (`combobox`). On a wide window a page opens beside the list
//! as Paseo's flyout (TRACKER 791, `kit::submenu`: by hover intent or a
//! press, the list staying open); below the compact width the list is a
//! sheet whose page replaces it in place with a way back (its title row),
//! Paseo's `compactMode="sheet"`.

use std::cell::Cell;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::Leading;
use vessel_app_kit::submenu::{Submenus, flyout};
use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Pixels, StatefulInteractiveElement as _, Styled as _, Window, div, px, svg,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Tokens, combobox, combobox_inline, select,
};
use vessel_app_view::prefs::SidebarView;
use vessel_app_view::sidebar::display::{DisplayAct, DisplayPage, OptionMark, RowKind};
use vessel_app_view::text::SidebarWords;

use crate::shell::Shell;

/// Paseo's `MENU_WIDTH`.
const WIDTH: f32 = 232.0;
/// The trigger's square, around its 14-point mark (Paseo's 28).
const TRIGGER: f32 = 28.0;
/// Its corners: Paseo's `borderRadius.md`; the kit has no 6 step.
const CORNER: f32 = 6.0;
/// Paseo's mark size in the heading.
const MARK: f32 = 14.0;

/// The menu's state: its list and the page it shows.
pub(crate) struct DisplayMenu {
    list: ComboboxState,
    page: DisplayPage,
    /// Where the trigger was drawn: the list opens aligned to its end.
    place: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Its pages beside it on a wide window (Paseo's flyouts).
    pages: Submenus,
    /// The open page's list there: it takes presses, the root the keys.
    page_list: ComboboxState,
}

impl DisplayMenu {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            list: ComboboxState::new(window, cx),
            page: DisplayPage::Root,
            place: Rc::new(Cell::new(None)),
            pages: Submenus::new(),
            page_list: ComboboxState::new(window, cx),
        }
    }
}

impl Shell {
    fn display_choose(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let menu = &mut self.areas.sidebar.display;
        match menu.page.choose(index, self.prefs.settings.sidebar) {
            Some(DisplayAct::Open(page)) => menu.page = page,
            Some(DisplayAct::Flip(change)) => self.prefer(&change, window, cx),
            Some(DisplayAct::Choose(change)) => {
                menu.list.close();
                menu.page = DisplayPage::Root;
                self.prefer(&change, window, cx);
            }
            None => {}
        }
        cx.notify();
    }

    /// An option chosen in `page`'s flyout: as on its page in place, and a
    /// final choice closes the menu with its pages.
    fn display_choose_beside(
        &mut self,
        page: DisplayPage,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match page.choose(index, self.prefs.settings.sidebar) {
            Some(DisplayAct::Flip(change)) => self.prefer(&change, window, cx),
            Some(DisplayAct::Choose(change)) => {
                let menu = &mut self.areas.sidebar.display;
                menu.list.close();
                menu.page_list.close();
                menu.pages.close_all();
                self.prefer(&change, window, cx);
            }
            // The Display menu's pages open no further page.
            Some(DisplayAct::Open(_)) | None => {}
        }
        cx.notify();
    }
}

/// A page's name among the menu's submenus.
const fn page_id(page: DisplayPage) -> Option<&'static str> {
    match page {
        DisplayPage::Root => None,
        DisplayPage::Grouping => Some("grouping"),
        DisplayPage::Sort => Some("sort"),
    }
}

/// The page named `id`.
fn page_named(id: &str) -> Option<DisplayPage> {
    [DisplayPage::Grouping, DisplayPage::Sort]
        .into_iter()
        .find(|page| page_id(*page) == Some(id))
}

/// The trigger at the heading's right.
pub(super) fn trigger(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let menu = &shell.areas.sidebar.display;
    div()
        .id("sidebar-display")
        .relative()
        .size(px(TRIGGER))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(CORNER))
        .cursor_pointer()
        .hover(|style| style.bg(tokens.bg_hover))
        .child(
            svg()
                .path(Glyph::Sliders.path())
                .size(px(MARK))
                .text_color(tokens.neutral.foreground_muted),
        )
        .child(vessel_app_kit::picker::anchor_probe(&menu.place))
        .on_click(cx.listener(|shell, _event, window, cx| {
            let menu = &mut shell.areas.sidebar.display;
            if menu.list.is_open() {
                menu.list.close();
            } else {
                menu.page = DisplayPage::Root;
                let bounds = menu
                    .place
                    .get()
                    .map(|bounds| select::end_aligned(bounds, WIDTH));
                menu.list.set_anchor(bounds);
                // Pages beside the list on a wide window; below the compact
                // width they replace it in place (its sheet's way back).
                let wide = window.viewport_size().width >= px(vessel_app_kit::picker::COMPACT);
                menu.list.set_submenus(wide.then_some(&menu.pages));
                menu.list.open(None, window, cx);
                menu.page_list.open_beside();
            }
            cx.notify();
        }))
        .into_any_element()
}

/// The menu, while open, and its open page beside it.
pub(super) fn menu(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let menu = &shell.areas.sidebar.display;
    if !menu.list.is_open() {
        return None;
    }
    let words = shell.t.lang.sidebar();
    let rows = rows(menu.page, shell.prefs.settings.sidebar, words);
    let title = menu.page.title(words);
    let header = ComboHeader {
        title: title.map(Into::into),
        back: title.is_some(),
        ..select::header()
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.sidebar.display.list,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.display_choose(index, window, cx);
        }),
        back: Some(Rc::new(|shell: &mut Shell, _window, cx| {
            shell.areas.sidebar.display.page = DisplayPage::Root;
            cx.notify();
        })),
    };
    let list = combobox(
        "sidebar-display-list",
        &menu.list,
        &header,
        &rows,
        &handlers,
        WIDTH,
        tokens,
        window,
        cx,
    );
    // The page is drawn after the list: same layer, so it paints on top.
    let beside = beside(shell, tokens, window, cx);
    Some(div().children(list).children(beside).into_any_element())
}

/// `page`'s rows as the kit's list rows; a row that opens a page names it.
fn rows(page: DisplayPage, view: SidebarView, words: &SidebarWords) -> Vec<ComboRow> {
    page.rows(view, words)
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let plain = ComboRow::new(row.label);
            match row.kind {
                RowKind::Opens { value } => ComboRow {
                    opens: true,
                    trailing: Some(value.into()),
                    page: if let Some(DisplayAct::Open(next)) = page.choose(index, view) {
                        page_id(next).map(Into::into)
                    } else {
                        None
                    },
                    ..plain
                },
                RowKind::Switch { on } => ComboRow {
                    switch: Some(on),
                    ..plain
                },
                RowKind::Choice { checked, mark } => ComboRow {
                    checked,
                    leading: mark.map(|mark| Leading::Glyph(glyph(mark))),
                    ..plain
                },
            }
        })
        .collect()
}

/// The open page beside the list on a wide window (Paseo's flyout, no
/// header of its own): its options, chosen as on the page in place.
fn beside(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let menu = &shell.areas.sidebar.display;
    let id = menu.pages.path().current()?.clone();
    let page = page_named(&id)?;
    let rows = rows(page, shell.prefs.settings.sidebar, shell.t.lang.sidebar());
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.sidebar.display.page_list,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            shell.display_choose_beside(page, index, window, cx);
        }),
        back: None,
    };
    let list = combobox_inline(
        "sidebar-display-page",
        &menu.page_list,
        &select::header(),
        &rows,
        &handlers,
        WIDTH,
        tokens,
        window,
        cx,
    )?;
    flyout(&menu.pages, &id, 0, 3, list)
}

/// An option's leading mark (Paseo's: a host, a dashed circle).
const fn glyph(mark: OptionMark) -> Glyph {
    match mark {
        OptionMark::Machine => Glyph::Server,
        OptionMark::Status => Glyph::CircleDot,
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
