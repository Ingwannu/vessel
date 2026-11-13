//! The archive (the page's `features/board/archive.tsx`): not a lane, a
//! dialog over the board that the archive's bar opens, holding the ended
//! and archived sessions' cards newest first, with a search, thirty at a
//! time; each archived card restores (starts again, or comes back to its
//! lane) or is deleted for good. A phone's width makes it a sheet, its
//! cards one to a row.
//!
//! It is [`Route::Archive`]: the sidebar's Archive row and the bar both
//! go there, and closing it goes back to the board.

use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{DialogContent, DialogSize, Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::board::{ArchivePage, Board, search_archive};

use super::board::{card, ghost};
use crate::routes::Route;
use crate::shell::Shell;

/// The share of the window's height the cards may take before they
/// scroll (the page's `60vh`).
const LIST_SHARE: f32 = 0.6;

impl Shell {
    /// The search changed: from the first page again.
    pub(crate) fn archive_searched(&mut self, text: String, cx: &mut Context<'_, Self>) {
        self.areas.board.search = text;
        self.areas.board.page = ArchivePage::default();
        cx.notify();
    }

    /// Close the archive, back to the board.
    fn close_archive(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.go(Route::Board, window, cx);
    }
}

/// Arriving at the archive: an empty search, the first page, the field
/// focused.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let area = &mut shell.areas.board;
    area.search.clear();
    area.page = ArchivePage::default();
    area.query.clear(window, cx);
    area.query
        .set_placeholder(shell.t.lang.board().archive_search, window, cx);
    area.query.focus(window, cx);
}

/// The archive's dialog over `board`.
pub(crate) fn dialog(
    shell: &Shell,
    board: &Board<'_>,
    scoped: bool,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let narrow = breakpoint::phone(window);
    let area = &shell.areas.board;
    let words = shell.t.lang.board();
    let neutral = tokens.neutral;
    let found = search_archive(&board.archive, &area.search);
    let shown = area.page.visible(found.len());
    let place = card::Place {
        archive: true,
        scoped,
        now: chrono::Utc::now(),
        narrow,
    };
    let list = if found.is_empty() {
        div()
            .type_(Type::Body)
            .text_color(neutral.foreground_muted)
            .child(if board.archive.is_empty() {
                words.no_archived
            } else {
                words.no_results
            })
            .into_any_element()
    } else {
        let height = f32::from(window.viewport_size().height).mul_add(LIST_SHARE, 0.0);
        let cards: Vec<AnyElement> = found
            .iter()
            .take(shown)
            .map(|row| card::card(shell, row, place, tokens, cx))
            .collect();
        div()
            .id("archive-cards")
            .max_h(px(height))
            .overflow_y_scroll()
            .child(
                div()
                    .grid()
                    .grid_cols(if narrow { 1 } else { 3 })
                    .gap(px(10.0))
                    .children(cards),
            )
            .into_any_element()
    };
    let more = area.page.more(found.len()).map(|count| {
        div().flex().justify_center().child(
            ghost(
                ElementId::Name("archive-more".into()),
                None,
                words.show_more(count),
                true,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, _window, cx| {
                shell.areas.board.page = shell.areas.board.page.next();
                cx.notify();
            })),
        )
    });
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(area.query.element(tokens, narrow, cx))
        .child(list)
        .children(more)
        .when(narrow, |body| body.min_w(px(0.0)));
    vessel_app_kit::dialog(
        &area.archive,
        DialogContent {
            title: words.archive.into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Wide,
        },
        tokens,
        window,
        narrow,
        Shell::close_archive,
        cx,
    )
}
