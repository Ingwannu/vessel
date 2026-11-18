//! A session row's menu in the sidebar, as Paseo's
//! (`components/sidebar/sidebar-workspace-menu.tsx`): opened from the
//! row's dots (shown while the row is hovered, always on touch) or a right
//! press on the row; Copy path, Mark as read or unread, Pin or Unpin,
//! Archive, in Paseo's order.
//!
//! [Decision Log]
//! - Purpose: the person changes a session's marks where they see it, as
//!   Paseo's sidebar allows, without opening it or the board.
//! - Constraint: what a mark changes elsewhere shows at once (the board,
//!   the archive, this list), without a reload. The node answers a mark
//!   with the session's new state, and the board's bookkeeping of acts
//!   (`Shell::act_on_card`, `board_acted`) puts that state in the fleet,
//!   which every one of them draws from.
//! - Choice: the view's `mark_actions` decides which marks a session
//!   offers (protocol 10 and its state); the sidebar keeps Paseo's four
//!   (no start again or delete: a listed session is not archived, and
//!   Paseo's row menu has neither). Archive asks first, with the board's
//!   own question (a live session is stopped first), drawn here while the
//!   board is not on screen. "Mark as read" reads through the session's
//!   `next_seq`: the person says they have seen it all, as Paseo's item
//!   does. Marking unread tells the conversation (track C's seam), so an
//!   open session does not mark itself read again at once.
//! - Differences, named: the dots are the kit's horizontal ones (Paseo's
//!   are vertical; the kit has no vertical mark); no Rename, Copy branch
//!   name or Open in file manager (no native seam yet); the menu's
//!   rows have no leading glyphs (the kit's menu has none). Labels is a
//!   page of the menu (TRACKER 791, `kit::submenu`): beside it on a wide
//!   window, the workspace picker's list in its flyout; on a phone the menu
//!   is a sheet titled "Session actions" (Paseo's "Workspace actions"; a
//!   Vessel row is a session) and the picker's sheet replaces it, leading
//!   back. The picker's Create stays a dialog (Paseo: a page with a field).

use std::rc::Rc;

use vessel_app_kit::Tokens;
use vessel_app_kit::menu::{Content, MenuHandlers, MenuItem, MenuState, menu_with};
use vessel_app_kit::submenu::{Submenus, flyout};
use vessel_app_kit::ui::{AnyElement, App, Context, Pixels, Point, Window};
use vessel_app_link::Act;
use vessel_app_view::Card;
use vessel_app_view::session::{CONVERSATION_PROTOCOL, MarkAction};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::Mark;

use crate::areas::labels::{
    CREATE_PAGE, PickerPlace, RowCreate, RowPicker, create_beside, picker_beside,
};
use crate::routes::Route;
use crate::shell::Shell;

/// The Labels page's name among the menu's submenus.
const LABELS: &str = "labels";

/// The menu's state and the session it is for.
pub(crate) struct RowMenu {
    menu: MenuState,
    target: Option<Target>,
    /// Its Labels page.
    pages: Submenus,
}

/// A session's row menu: the session and what it offers.
struct Target {
    card: Card,
    acts: Vec<RowAct>,
}

/// What an item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowAct {
    /// Copy the session's folder.
    CopyPath,
    /// Change a mark.
    Mark(MarkAction),
    /// Its workspace's labels (the picker).
    Labels,
}

impl RowMenu {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            pages: Submenus::new(),
        }
    }

    /// Whether the menu is open for `card` (its row keeps its dots shown,
    /// as Paseo keeps the kebab mounted while its menu is up).
    pub(super) fn is_for(&self, card: &Card) -> bool {
        self.menu.is_open()
            && self.target.as_ref().is_some_and(|target| {
                target.card.machine == card.machine && target.card.session == card.session
            })
    }
}

/// What `card`'s menu offers, in Paseo's order: the view's rule for marks
/// (`session::mark_actions`: protocol 10, then the session's own marks)
/// read from the card the sidebar holds; a listed session is never
/// archived, so Archive is always last.
fn acts(shell: &Shell, card: &Card) -> Vec<RowAct> {
    let marks_taken = shell
        .fleet
        .machines
        .iter()
        .find(|machine| machine.name == card.machine)
        .and_then(|machine| machine.protocol)
        .is_some_and(|protocol| protocol >= CONVERSATION_PROTOCOL);
    let mut acts = Vec::new();
    if card.detail.cwd.is_some() {
        acts.push(RowAct::CopyPath);
    }
    if marks_taken {
        let marks = card.detail.marks;
        if !marks.unread {
            acts.push(RowAct::Mark(MarkAction::MarkUnread));
        } else if card.detail.next_seq.is_some() {
            acts.push(RowAct::Mark(MarkAction::MarkRead));
        }
        acts.push(RowAct::Mark(if marks.pinned {
            MarkAction::Unpin
        } else {
            MarkAction::Pin
        }));
    }
    // Paseo's order: after Pin, before Archive; only for a session whose
    // host knows its workspace.
    if shell
        .areas
        .labels
        .row(&card.machine, card.session, card.member_project())
        .is_some()
    {
        acts.push(RowAct::Labels);
    }
    if marks_taken {
        acts.push(RowAct::Mark(MarkAction::Archive));
    }
    acts
}

impl Shell {
    /// Open `card`'s menu at `at` (its dots, or where the row was pressed).
    pub(super) fn row_menu_open(
        &mut self,
        card: &Card,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let acts = acts(self, card);
        if acts.is_empty() {
            return;
        }
        let rows = &mut self.areas.sidebar.rows;
        rows.target = Some(Target {
            card: card.clone(),
            acts,
        });
        rows.menu.open(at, window, cx);
        cx.notify();
    }

    fn row_menu_chose(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(target) = self.areas.sidebar.rows.target.take() else {
            return;
        };
        let Some(act) = target.acts.get(index).copied() else {
            return;
        };
        let card = target.card;
        match act {
            RowAct::CopyPath => {
                if let Some(folder) = &card.detail.cwd {
                    cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(
                        folder.clone(),
                    ));
                    self.toast(self.t.lang.sidebar().path_copied, ToastTone::Copied, cx);
                }
            }
            RowAct::Mark(MarkAction::Archive) => self.ask_archive(&card, window, cx),
            RowAct::Mark(MarkAction::MarkRead) => {
                if let Some(through) = card.detail.next_seq {
                    self.act_on_card(&card, Act::Mark(Mark::Read { through }));
                }
            }
            RowAct::Mark(MarkAction::MarkUnread) => {
                self.act_on_card(&card, Act::Mark(Mark::Unread));
                self.conversation_marked_unread(&card.machine, card.session);
            }
            RowAct::Mark(MarkAction::Pin) => self.act_on_card(&card, Act::Mark(Mark::Pin)),
            RowAct::Mark(MarkAction::Unpin) => self.act_on_card(&card, Act::Mark(Mark::Unpin)),
            // Labels is a page of the menu: the menu opens it, never choosing
            // it.
            RowAct::Mark(MarkAction::StartAgain | MarkAction::Unarchive | MarkAction::Delete)
            | RowAct::Labels => {}
        }
        cx.notify();
    }

    /// Follow the menu's Labels page with the workspace picker: opened for
    /// the row's workspace when the page opens (beside the menu, or on a
    /// phone as the sheet that replaces it), the keys moved in when a press
    /// or Enter opened it, closed with the page, and the menu closed when
    /// the person dismisses the picker (Escape, its sheet's scrim).
    fn row_menu_labels_sync(
        &mut self,
        phone: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rows = &self.areas.sidebar.rows;
        let open = rows.menu.is_open() && rows.pages.is_open(LABELS, 0);
        match (open, self.areas.labels.row_picker()) {
            (true, RowPicker::None) => {
                let Some(card) = rows.target.as_ref().map(|target| target.card.clone()) else {
                    return;
                };
                let place = if phone {
                    PickerPlace::Page(Rc::new(|shell: &mut Shell, window, cx| {
                        shell.row_menu_labels_back(window, cx);
                    }))
                } else {
                    PickerPlace::Beside
                };
                let project = card.member_project();
                let opened =
                    self.labels_row_picker(&card.machine, card.session, project, place, window, cx);
                if opened && !phone {
                    let pages = self.areas.sidebar.rows.pages.clone();
                    self.labels_row_picker_pages(&pages);
                }
                if opened {
                    self.row_menu_labels_keys(phone, window, cx);
                } else {
                    self.areas.sidebar.rows.pages.close_all();
                }
            }
            (true, RowPicker::Open) => {
                self.row_menu_labels_keys(phone, window, cx);
                if !phone {
                    self.row_menu_create_sync(window, cx);
                }
            }
            (true, RowPicker::Dismissed) => {
                self.labels_row_picker_close();
                let rows = &mut self.areas.sidebar.rows;
                rows.pages.close_all();
                rows.target = None;
                rows.menu.close_and_restore(window, cx);
            }
            (false, RowPicker::Open | RowPicker::Dialog | RowPicker::Dismissed) => {
                self.labels_row_picker_close();
            }
            // The dialog's own Back returns to the list, which takes the keys.
            (true, RowPicker::Dialog) | (false, RowPicker::None) => {}
        }
        cx.notify();
    }

    /// A press or Enter opened the Labels page: beside the menu its list
    /// takes the keys (one the pointer opened leaves them with the menu); a
    /// phone's page sheet took them when it opened.
    fn row_menu_labels_keys(
        &mut self,
        phone: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.sidebar.rows.pages.take_pressed(LABELS) && !phone {
            self.labels_row_picker_keys(window, cx);
        }
    }

    /// Follow the Labels list's Create label page beside it (TRACKER 792):
    /// a press or Enter on its row opened it, so a draft starts; open with no
    /// draft left means a label was made and the list is back, so it
    /// closes; closed under a draft (a press outside, Escape in the list),
    /// the draft goes. A save under way is left to finish.
    fn row_menu_create_sync(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let pages = &self.areas.sidebar.rows.pages;
        match (
            pages.is_open(CREATE_PAGE, 1),
            self.areas.labels.row_create(),
        ) {
            (true, RowCreate::None) => {
                if pages.take_pressed(CREATE_PAGE) {
                    self.labels_row_picker_create(window, cx);
                } else {
                    let _closed = pages.back();
                }
            }
            (false, RowCreate::Draft) => self.labels_row_picker_create_closed(window, cx),
            (true, RowCreate::Draft | RowCreate::Saving)
            | (false, RowCreate::None | RowCreate::Saving) => {}
        }
    }

    /// The phone's Labels page led back: the menu's sheet again.
    fn row_menu_labels_back(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.labels_row_picker_close();
        let _back = self.areas.sidebar.rows.pages.back();
        cx.notify();
    }
}

/// Whether the menu's Labels page and its picker disagree, or a press
/// that opened the page waits to move the keys in.
fn labels_out_of_step(shell: &Shell, phone: bool) -> bool {
    let rows = &shell.areas.sidebar.rows;
    let open = rows.menu.is_open() && rows.pages.is_open(LABELS, 0);
    match (open, shell.areas.labels.row_picker()) {
        (true, RowPicker::None | RowPicker::Dismissed)
        | (false, RowPicker::Open | RowPicker::Dialog | RowPicker::Dismissed) => true,
        (true, RowPicker::Open) => {
            !phone
                && (rows.pages.pressed(LABELS)
                    || matches!(
                        (
                            rows.pages.is_open(CREATE_PAGE, 1),
                            shell.areas.labels.row_create()
                        ),
                        (true, RowCreate::None) | (false, RowCreate::Draft)
                    ))
        }
        (true, RowPicker::Dialog) | (false, RowPicker::None) => false,
    }
}

/// The menu while open, and the board's archive question while the board
/// is not on screen (it draws its own).
pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    phone: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let rows = &shell.areas.sidebar.rows;
    let words = shell.t.lang.sidebar();
    let said = shell.t.lang.conversation();
    let items: Vec<MenuItem> = rows
        .target
        .as_ref()
        .map(|target| {
            target
                .acts
                .iter()
                .map(|act| match act {
                    RowAct::CopyPath => MenuItem::new(words.copy_path),
                    RowAct::Mark(action) => MenuItem::new(said.mark_action(*action)),
                    RowAct::Labels => MenuItem::sub(words.labels, LABELS),
                })
                .collect()
        })
        .unwrap_or_default();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.sidebar.rows.menu,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.row_menu_chose(index, window, cx);
        }),
    };
    let content = Content::new("sidebar-row-menu", &items)
        .with_submenus(&rows.pages)
        .with_sheet(words.row_actions);
    // While a phone's Create label sheet is up the menu waits behind it
    // undrawn, so a press in the sheet is not a press outside the menu
    // (beside the menu the page is a flyout of the menu, TRACKER 792).
    let waiting = shell.areas.labels.row_picker() == RowPicker::Dialog;
    let mut drawn: Vec<AnyElement> = if waiting {
        Vec::new()
    } else {
        menu_with(content, &rows.menu, &handlers, tokens, phone, cx)
            .into_iter()
            .collect()
    };
    // Its Labels page beside it, drawn after it (same layer, on top); on a
    // phone the picker's own sheet replaces the menu's. The list's Create
    // label page beside the list, after it (TRACKER 792).
    if !phone && rows.menu.is_open() && rows.pages.is_open(LABELS, 0) {
        drawn.extend(
            picker_beside(shell, tokens, window, cx)
                .and_then(|list| flyout(&rows.pages, LABELS, 0, 1, list)),
        );
        if rows.pages.is_open(CREATE_PAGE, 1) {
            drawn.extend(
                create_beside(shell, tokens, cx)
                    .and_then(|page| flyout(&rows.pages, CREATE_PAGE, 1, 1, page)),
            );
        }
    }
    if labels_out_of_step(shell, phone) {
        cx.defer_in(window, move |shell, window, cx| {
            shell.row_menu_labels_sync(phone, window, cx);
        });
    }
    if shell.route != Route::Board {
        drawn.extend(crate::areas::board::card_question(
            shell, tokens, window, cx,
        ));
    }
    drawn
}

#[cfg(test)]
#[path = "row_menu_tests.rs"]
mod tests;
