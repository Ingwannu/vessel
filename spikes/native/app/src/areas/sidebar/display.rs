//! The sidebar's Display menu: its trigger at the list's heading and the
//! menu itself, Paseo's (`display-preferences/menu.tsx`: a 14-point
//! Settings2 mark at the heading's right, a 232-wide surface aligned to the
//! trigger's end). What each row is and does is the view's
//! (`vessel_app_view::sidebar::display`); this file draws it with the
//! kit's list (`combobox`), whose title row leads back from a page.
//!
//! Difference, named: a page replaces the root in place with a way back
//! (Paseo's compact sheet) on a wide window too; Paseo's wide popover opens
//! it as a flyout by hover intent, which the kit's list does not draw.

use std::cell::Cell;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::Leading;
use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Pixels, StatefulInteractiveElement as _, Styled as _, Window, div, px, svg,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Tokens, combobox, select,
};
use vessel_app_view::sidebar::display::{DisplayAct, DisplayPage, OptionMark, RowKind};

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
}

impl DisplayMenu {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            list: ComboboxState::new(window, cx),
            page: DisplayPage::Root,
            place: Rc::new(Cell::new(None)),
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
                menu.list.open(None, window, cx);
            }
            cx.notify();
        }))
        .into_any_element()
}

/// The menu, while open.
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
    let view = shell.prefs.settings.sidebar;
    let rows: Vec<ComboRow> = menu
        .page
        .rows(view, words)
        .into_iter()
        .map(|row| {
            let plain = ComboRow::new(row.label);
            match row.kind {
                RowKind::Opens { value } => ComboRow {
                    opens: true,
                    trailing: Some(value.into()),
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
        .collect();
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
    combobox(
        "sidebar-display-list",
        &menu.list,
        &header,
        &rows,
        &handlers,
        WIDTH,
        tokens,
        window,
        cx,
    )
}

/// An option's leading mark (Paseo's: a host, a dashed circle).
const fn glyph(mark: OptionMark) -> Glyph {
    match mark {
        OptionMark::Machine => Glyph::Server,
        OptionMark::Status => Glyph::CircleDot,
    }
}
