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
//!   name, Labels or Open in file manager (no native seam yet); the menu's
//!   rows have no leading glyphs (the kit's menu has none).

use std::rc::Rc;

use vessel_app_kit::Tokens;
use vessel_app_kit::menu::{MenuHandlers, MenuItem, MenuState, menu};
use vessel_app_kit::ui::{AnyElement, App, Context, Pixels, Point, Window};
use vessel_app_link::Act;
use vessel_app_view::Card;
use vessel_app_view::session::{CONVERSATION_PROTOCOL, MarkAction};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::Mark;

use crate::routes::Route;
use crate::shell::Shell;

/// The menu's state and the session it is for.
pub(crate) struct RowMenu {
    menu: MenuState,
    target: Option<Target>,
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
}

impl RowMenu {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
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
                    self.toast(self.t.lang.settings().path_copied, ToastTone::Ok, cx);
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
            RowAct::Mark(MarkAction::StartAgain | MarkAction::Unarchive | MarkAction::Delete) => {}
        }
        cx.notify();
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
    let mut drawn: Vec<AnyElement> = menu(
        "sidebar-row-menu",
        &rows.menu,
        &items,
        &handlers,
        tokens,
        phone,
        cx,
    )
    .into_iter()
    .collect();
    if shell.route != Route::Board {
        drawn.extend(crate::areas::board::card_question(
            shell, tokens, window, cx,
        ));
    }
    drawn
}
