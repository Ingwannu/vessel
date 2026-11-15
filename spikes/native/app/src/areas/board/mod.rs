//! The board (the page's `features/board/board-page.tsx` and `lane.tsx`,
//! DESIGN.md §6.12): the machine scope when the book has more than one
//! machine, four lanes of cards side by side, each scrolling by itself
//! (stacked, the page scrolling as one, below a phone's width), the
//! archive's bar under them, and the empty board. The archive's dialog is
//! `archive.rs`; a card is `card.rs`; what its actions ask, `acts.rs`.
//!
//! [Decision Log]
//! - Purpose: the page's board, whole, in the app.
//! - Choice: neither Paseo nor GOAT has a board, so the page's structure
//!   (its lanes, its card's lines) is kept and drawn in Paseo's neutrals
//!   and controls: hairlines in `border`, cards on `surface1` with 8
//!   corners, the hover a 4% wash of the foreground (the page's).
//! - Cost: a board that is neither Paseo's nor the page's pixel for
//!   pixel; it changes when the owner gives it a reference.

mod acts;

/// The board's question about a card (kill, archive, delete), for a screen
/// that asks it while the board is not drawn (the sidebar's row menu).
pub(crate) use acts::confirm as card_question;
pub(crate) mod card;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::Utc;
use gpui::FontWeight;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::breakpoint;
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _,
    IntoElement as _, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    DialogState, TextField, Tokens, Tooltips, Type, TypeExt as _, button, glyph, tooltip,
};
use vessel_app_view::Lane;
use vessel_app_view::board::{ArchivePage, Board, TICK};

use crate::routes::Route;
use crate::shell::Shell;

/// The least width of a lane. The page's `--lane-min-w` is 256; here it
/// is 240 so that the four lanes fit beside the 264 sidebar in the
/// default 1280 window (1016 left) and share it, as the page's grid does.
const LANE_MIN: f32 = 240.0;

/// Four lanes at their least: past it the board scrolls sideways.
const LANES_MIN: f32 = 960.0;

/// A lane's heading (the page's `--lane-head-h`).
const LANE_HEAD: f32 = 48.0;

/// The board's state: its scope, the confirm a card's action asked, the
/// archive's search, and the acts asked from it not yet answered.
pub(crate) struct BoardArea {
    /// The machine the board shows (the page's `?node=`); `None`: all.
    pub(crate) scope: Option<String>,
    /// The confirm open over the board.
    pub(crate) confirm: Option<acts::Asking>,
    /// The confirm's dialog.
    pub(crate) dialog: DialogState,
    /// The archive's dialog.
    pub(crate) archive: DialogState,
    /// The archive's search field.
    pub(crate) query: TextField,
    /// Its text as last typed.
    pub(crate) search: String,
    /// How much of the archive is shown.
    pub(crate) page: ArchivePage,
    /// Acts asked from a card, until the node answers.
    pub(crate) asked: Vec<acts::Asked>,
    /// The cards' buttons' tooltips (one shown at most).
    pub(crate) tips: Tooltips,
    /// Where each card's button was drawn, by the button's id.
    places: RefCell<HashMap<String, tooltip::Place>>,
}

impl BoardArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        // The page's shared clock: "6m ago" moves on every card at once.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let ticked = this.update(cx, |shell, cx| {
                    if matches!(shell.route, Route::Board | Route::Archive) {
                        cx.notify();
                    }
                });
                if ticked.is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            scope: None,
            confirm: None,
            dialog: DialogState::new(cx),
            archive: DialogState::new(cx),
            query: TextField::with_change("", |_, _, _, _| {}, Shell::archive_searched, window, cx),
            search: String::new(),
            page: ArchivePage::default(),
            asked: Vec::new(),
            tips: Tooltips::new(),
            places: RefCell::default(),
        }
    }

    /// The cell where the button `key` is drawn, kept across frames.
    pub(crate) fn place(&self, key: &str) -> tooltip::Place {
        let mut places = self.places.borrow_mut();
        if let Some(place) = places.get(key) {
            return Rc::clone(place);
        }
        let place = tooltip::Place::default();
        drop(places.insert(key.to_owned(), Rc::clone(&place)));
        place
    }
}

impl Shell {
    /// Show one machine's sessions, or every machine's (the board's own
    /// strip, and the sidebar's "N more" under a machine).
    pub(crate) fn scope_board(&mut self, scope: Option<String>, cx: &mut Context<'_, Self>) {
        self.areas.board.scope = scope;
        cx.notify();
    }

    /// The scope, while its machine is still in the book.
    pub(crate) fn board_scope(&self) -> Option<&str> {
        self.areas.board.scope.as_deref().filter(|scope| {
            self.fleet
                .machines
                .iter()
                .any(|machine| machine.name == *scope)
        })
    }

    /// A new session from the board: on the machine it shows, if one.
    fn start_from_board(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let scope = self.board_scope().map(str::to_owned);
        self.go(Route::Draft, window, cx);
        if let Some(scope) = scope {
            self.choose_machine(scope, window, cx);
        }
    }
}

/// Draw the board, and the archive over it on [`Route::Archive`].
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let narrow = breakpoint::phone(window);
    let scope = shell.board_scope();
    let board = shell.fleet.board(scope);
    let body = if board.is_empty() {
        empty(shell, scope, tokens, cx).into_any_element()
    } else {
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(lanes(shell, &board, scope.is_some(), narrow, tokens, cx))
            .child(archive_bar(shell, board.archive.len(), tokens, cx))
            .into_any_element()
    };
    let archive = (shell.route == Route::Archive)
        .then(|| super::archive::dialog(shell, &board, scope.is_some(), tokens, window, cx));
    let confirm = acts::confirm(shell, tokens, window, cx);
    div()
        .size_full()
        .flex()
        .flex_col()
        .children(scope_bar(shell, scope, tokens, cx))
        .child(body)
        .children(archive.map(|dialog| div().id("board-archive").child(dialog)))
        .children(confirm.map(|dialog| div().id("board-confirm").child(dialog)))
        .children(shell.areas.board.tips.render(tokens))
        .into_any_element()
}

/// A lane's colour (the page's `--st-*`).
pub(crate) const fn lane_color(lane: Lane, tokens: &Tokens) -> Hsla {
    match lane {
        Lane::Building => tokens.st_building,
        Lane::Validating => tokens.st_validating,
        Lane::InReview => tokens.st_review,
        Lane::Ready => tokens.st_ready,
    }
}

/// The machine scope, when the book has more than one machine: All
/// machines, then each by name (a row that scrolls sideways when it does
/// not fit).
fn scope_bar(
    shell: &Shell,
    scope: Option<&str>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Stateful<Div>> {
    if shell.fleet.machines.len() < 2 {
        return None;
    }
    let words = shell.t.lang.board();
    let mut row = segmented(SegmentSize::Small).child(
        segment(
            "scope-all",
            words.all_machines,
            scope.is_none(),
            true,
            SegmentSize::Small,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, _window, cx| shell.scope_board(None, cx))),
    );
    for machine in &shell.fleet.machines {
        let name = machine.name.clone();
        row = row.child(
            segment(
                ElementId::Name(format!("scope-{name}").into()),
                name.clone(),
                scope == Some(name.as_str()),
                true,
                SegmentSize::Small,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.scope_board(Some(name.clone()), cx);
            })),
        );
    }
    Some(
        div()
            .id("board-scope")
            .flex_none()
            .overflow_x_scroll()
            .px(px(12.0))
            .py(px(6.0))
            .border_b_1()
            .border_color(tokens.neutral.border)
            .child(row),
    )
}

/// The four lanes: side by side, past their least width scrolling
/// sideways; below a phone's width stacked, scrolling as one.
fn lanes(
    shell: &Shell,
    board: &Board<'_>,
    scoped: bool,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let columns: Vec<AnyElement> = Lane::ALL
        .into_iter()
        .map(|lane| lane_view(shell, board, lane, scoped, narrow, tokens, cx))
        .collect();
    if narrow {
        div()
            .id("board")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(div().flex().flex_col().children(columns))
    } else {
        div()
            .id("board")
            .flex_1()
            .min_h(px(0.0))
            .overflow_x_scroll()
            .restrict_scroll_to_axis()
            .child(
                div()
                    .h_full()
                    .w_full()
                    .min_w(px(LANES_MIN))
                    .flex()
                    .flex_row()
                    .children(columns),
            )
    }
}

/// One lane: its heading (a dot and the name in its colour, the count),
/// its cards; an empty Building lane offers a new session.
fn lane_view(
    shell: &Shell,
    board: &Board<'_>,
    lane: Lane,
    scoped: bool,
    narrow: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let color = lane_color(lane, tokens);
    let rows = board.lane(lane);
    let now = Utc::now();
    let place = card::Place {
        archive: false,
        scoped,
        now,
        narrow,
    };
    let cards: Vec<AnyElement> = rows
        .iter()
        .map(|row| card::card(shell, row, place, tokens, cx))
        .collect();
    let new = (rows.is_empty() && lane == Lane::Building).then(|| {
        div().flex().justify_center().py(px(8.0)).child(
            ghost(
                "lane-new",
                Some(Glyph::Plus),
                shell.t.lang.board().new_session,
                true,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| shell.start_from_board(window, cx))),
        )
    });
    let head = div()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .h(px(LANE_HEAD))
        .px(px(16.0))
        .border_b_1()
        .border_color(neutral.border)
        .child(div().flex_none().size(px(8.0)).rounded_full().bg(color))
        .child(
            div()
                .type_(Type::Meta)
                .font_weight(FontWeight::MEDIUM)
                .text_color(color)
                .child(shell.t.lane(lane)),
        )
        .child(
            div()
                .ml_auto()
                .type_(Type::Meta)
                .text_color(neutral.foreground_extra_muted)
                .child(rows.len().to_string()),
        );
    let list = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .pt(px(12.0))
        .pr(px(8.0))
        .pb(px(12.0))
        .pl(px(12.0))
        .children(cards)
        .children(new);
    let column = div().flex().flex_col().child(head);
    if narrow {
        column
            .border_b_1()
            .border_color(neutral.border)
            .child(list)
            .into_any_element()
    } else {
        column
            .flex_1()
            .min_w(px(LANE_MIN))
            .h_full()
            .when(lane != Lane::Ready, |column| {
                column.border_r_1().border_color(neutral.border)
            })
            .child(
                div()
                    .id(ElementId::Name(format!("lane-{lane:?}").into()))
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(list),
            )
            .into_any_element()
    }
}

/// The archive's bar under the lanes: a quiet count that opens it.
fn archive_bar(shell: &Shell, count: usize, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.board();
    let open = ghost(
        "archive-open",
        Some(Glyph::Archive),
        words.archive,
        count > 0,
        tokens,
    )
    .child(
        div()
            .text_color(tokens.neutral.foreground_extra_muted)
            .child(count.to_string()),
    )
    .when(count > 0, |open| {
        open.on_click(cx.listener(|shell, _event, window, cx| shell.go(Route::Archive, window, cx)))
    });
    div()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .h(px(36.0))
        .px(px(8.0))
        .border_t_1()
        .border_color(tokens.neutral.border)
        .child(open)
}

/// The empty board: what to do, and a button that does it.
fn empty(shell: &Shell, scope: Option<&str>, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.board();
    let neutral = tokens.neutral;
    let title = scope.map_or_else(
        || words.no_sessions.to_owned(),
        |machine| words.no_sessions_on(machine),
    );
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .p(px(24.0))
        .child(
            div()
                .type_(Type::Title)
                .text_color(neutral.foreground)
                .text_center()
                .child(title),
        )
        .child(
            div()
                .max_w(px(360.0))
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .text_center()
                .child(words.empty_body),
        )
        .child(div().pt(px(8.0)).child(
            button("board-new", words.new_session, true, tokens).on_click(
                cx.listener(|shell, _event, window, cx| shell.start_from_board(window, cx)),
            ),
        ))
}

/// A quiet button: a glyph and words in the muted foreground, the
/// hover's wash (Paseo's `Button variant="ghost"`, 28 tall).
///
/// A stand-in: the kit has no ghost button yet (the lead's base); this
/// is replaced by the kit's when it lands.
pub(crate) fn ghost(
    id: impl Into<ElementId>,
    icon: Option<Glyph>,
    label: impl Into<SharedString>,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let color = neutral.foreground_muted;
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(28.0))
        .px(px(10.0))
        .rounded(px(vessel_app_kit::popup::ROW_RADIUS))
        .type_(Type::Meta)
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .children(icon.map(|icon| glyph(icon, 14.0, color)))
        .child(label.into())
        .when(!enabled, |ghost| ghost.opacity(0.5))
        .when(enabled, |ghost| {
            ghost
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2).text_color(neutral.foreground))
        })
}
