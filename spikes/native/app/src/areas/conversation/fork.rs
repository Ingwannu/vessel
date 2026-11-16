//! Paseo's fork in a turn's footer (`components/assistant-fork-menu.tsx`,
//! `agent-stream/turn-footer.tsx`, `hooks/use-fork-agent.ts`): a mark
//! between the reply's copy button and "Worked for", its menu, the fork
//! asked of the node (`session.fork`, protocol 14), and the new session
//! opened.
//!
//! [Decision Log]
//! - Purpose: a person continues a conversation from one of its turns in
//!   a new session beside it (slice F1, `docs/plans/backend-m12-conversation.md`).
//! - The reference: Paseo's mark is lucide's `Split` (16), muted, in the
//!   foreground while pointed at or open, with no surface behind it; its
//!   tooltip "Fork chat from here"; its menu "Fork in a new tab" / "Fork
//!   in a new workspace"; the mark is locked (50 %) while a fork goes; a
//!   failure is a toast with the error's words, else "Failed to fork
//!   chat"; success opens the fork (Paseo's is a draft tab seeded with the
//!   conversation).
//! - Constraints: our node forks into a new session at once (no draft),
//!   at a turn's end the agent named (`view::timeline::fork_point`); the
//!   answer is the new session, not the one acted on; opening a session
//!   needs a window, which an act's answer does not have; the kit's menu
//!   closes when a row is chosen and draws no pending row.
//! - Choice: Paseo's mark, tooltip and menu with our one target ("Fork in
//!   a new session"); the menu closes when chosen and the mark stays
//!   locked until the answer; the new session is listed at once
//!   (`Fleet::add_session`) and opened on the next frame
//!   ([`Shell::forked_open`], as a rewound message goes back into the
//!   composer); a refusal is the notice's, in Paseo's words.
//! - Differences, kept: no pending row in the menu; no fork from the
//!   working turn (the node refuses a whole-conversation fork while a turn
//!   runs); the tooltip shows at once (the kit's).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, App, Context, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Point, Styled,
    Window, div, px,
};
use vessel_app_kit::{MenuHandlers, MenuItem, MenuState, Tooltips};
use vessel_app_link::{Act, ActAnswer, ActFailed, ActKind, Acted};
use vessel_app_view::Card;
use vessel_app_view::fork::{Headers, Notice, PreparedNotice};
use vessel_app_view::timeline::{fork_point, origin_of};
use vessel_proto::EventSeq;

use super::entries::{Paint, id};
use crate::areas::session::Open;
use crate::shell::Shell;

/// The mark's size (Paseo's 16-point icon in a 4-point pad).
const MARK: f32 = 24.0;

/// A fork being chosen in a turn's footer, sent, or opening.
pub(crate) struct Forking {
    /// The menu under the mark.
    pub(super) menu: MenuState,
    /// The turn's footer (its entry) and the turn's end the menu is for.
    target: Option<(usize, EventSeq)>,
    /// The footer whose fork is on its way.
    pending: Option<usize>,
    /// The new session, to open on the next frame (or, once an empty
    /// fork is removed, the session it was forked from).
    opening: Option<Open>,
    /// An empty fork's Remove is on its way (F2).
    removing: bool,
    /// The marks' tooltip (one shown at a time).
    pub(super) tips: Tooltips,
    /// Where each footer's mark was last drawn, by entry.
    places: RefCell<HashMap<usize, Place>>,
    /// Captions prepared at fleet/language events, shared with origin rows.
    pub(super) headers: Headers,
    /// Retained while its global frame notice may survive navigation.
    notice: Option<PreparedNotice>,
}

impl std::fmt::Debug for Forking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Forking")
            .field("target", &self.target)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Forking {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            pending: None,
            opening: None,
            removing: false,
            tips: Tooltips::new(),
            places: RefCell::default(),
            headers: Headers::default(),
            notice: None,
        }
    }

    /// Whether an empty fork's Remove is on its way.
    pub(super) const fn removing(&self) -> bool {
        self.removing
    }

    /// Whether a new session waits to be opened.
    pub(super) const fn has_opening(&self) -> bool {
        self.opening.is_some()
    }

    /// Nothing chosen or on its way (another session shows). A fork
    /// waiting to open stays: it is what the person asked to see next.
    pub(super) fn clear(&mut self) {
        self.menu.close();
        self.target = None;
        self.pending = None;
        self.removing = false;
        self.tips = Tooltips::new();
        self.places.borrow_mut().clear();
    }

    /// The mark's place for the footer at `entry`, kept across frames so
    /// its tooltip stays where the mark is.
    fn place(&self, entry: usize) -> Place {
        Rc::clone(self.places.borrow_mut().entry(entry).or_default())
    }
}

/// The fork mark in the footer of the turn ending at `index`, where it can
/// be forked and the conversation is the session area's own; `None`
/// elsewhere.
pub(super) fn mark(
    paint: &Paint<'_>,
    index: usize,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (timeline, forking) = paint.forks?;
    let after = fork_point(timeline, index)?;
    let neutral = paint.tokens.neutral;
    let locked = forking.pending.is_some();
    let open = forking.menu.is_open() && forking.target.is_some_and(|(entry, _)| entry == index);
    let group = format!("fork-{index}");
    let ink = if open {
        neutral.foreground
    } else {
        neutral.foreground_muted
    };
    let trigger = div()
        .id(ElementId::NamedInteger("fork".into(), id(index)))
        .group(group.clone())
        .size(px(MARK))
        .flex()
        .items_center()
        .justify_center()
        .when(locked, |trigger| trigger.opacity(0.5))
        .when(!locked, |trigger| {
            trigger.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                    shell.fork_open((index, after), event.position, window, cx);
                }),
            )
        })
        .child(
            glyph(Glyph::Split, 16.0, ink)
                .group_hover(group, move |style| style.text_color(neutral.foreground)),
        );
    Some(
        forking
            .tips
            .on(
                trigger,
                &forking.place(index),
                paint.catalog.fork().tooltip(),
                narrow,
            )
            .into_any_element(),
    )
}

/// The fork menu's rows: our node's one target.
pub(super) fn menu_items(shell: &Shell) -> Vec<MenuItem> {
    vec![MenuItem::new(shell.words.fork().in_new_session())]
}

impl Shell {
    /// Prepare only changed source titles after fleet events, before a frame.
    pub(crate) fn refresh_fork_headers(&mut self) {
        match self
            .areas
            .conversation
            .forking
            .headers
            .refresh(&self.fleet, self.words)
        {
            Ok(true) => {
                let list = &self.areas.conversation.list;
                list.changed(0..list.rows());
                self.areas.session.entries.remeasure();
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "preparing fork source captions"),
        }
    }

    /// Refresh retained notices without reissuing an act or changing inputs.
    pub(crate) fn translate_fork_words(&mut self) {
        self.refresh_fork_headers();
        let forking = &mut self.areas.conversation.forking;
        if let Some(notice) = &mut forking.notice {
            let owned = self.notice.as_deref() == Some(&*notice.words());
            notice.translate(self.words);
            if owned {
                self.notice = Some(notice.words().to_string());
            }
        }
        self.translate_origin_notice();
    }

    /// Open the fork menu for the footer at `target` (its entry and its
    /// turn's end), at `at`.
    fn fork_open(
        &mut self,
        target: (usize, EventSeq),
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let forking = &mut self.areas.conversation.forking;
        if forking.pending.is_some() {
            return;
        }
        forking.target = Some(target);
        forking.menu.open(at, window, cx);
        cx.notify();
    }

    /// The fork menu's handlers: its state, and what its row does.
    pub(super) fn fork_menu_handlers() -> MenuHandlers<Self> {
        MenuHandlers {
            state: |shell: &mut Self| &mut shell.areas.conversation.forking.menu,
            chose: Rc::new(|shell: &mut Self, _index, _window, cx| shell.fork_send(cx)),
        }
    }

    /// Fork at the menu's turn's end.
    fn fork_send(&mut self, cx: &mut Context<'_, Self>) {
        let forking = &mut self.areas.conversation.forking;
        forking.menu.close();
        let Some((entry, after)) = forking.target else {
            return;
        };
        forking.pending = Some(entry);
        self.act(Act::Fork(Some(after)));
        cx.notify();
    }

    /// A fork's answer for the session shown: the lock lifts; the new
    /// session opens on the next frame; a refusal is the notice's, in
    /// Paseo's words.
    pub(super) fn fork_answered(&mut self, answer: &ActAnswer) {
        if answer.kind != ActKind::Fork {
            return;
        }
        let forking = &mut self.areas.conversation.forking;
        forking.pending = None;
        forking.target = None;
        match &answer.result {
            Ok(Acted::Forked(info)) => {
                forking.opening = Some(Open::of(&Card::from_session(&answer.machine, info)));
            }
            Err(ActFailed {
                code: Some(code),
                message,
            }) => {
                tracing::warn!(?code, %message, "fork refused");
                let notice = PreparedNotice::new(Notice::Refused((*code).into()), self.words);
                self.notice = Some(notice.words().to_string());
                forking.notice = Some(notice);
            }
            // No answer at all: the session's lost connection says it.
            Err(ActFailed { code: None, .. })
            | Ok(
                Acted::Session(_)
                | Acted::Deleted(_)
                | Acted::Done
                | Acted::Conversation(_)
                | Acted::Log(_),
            ) => {}
        }
    }

    /// Remove the empty fork shown (its banner's button, F2): it never
    /// held a conversation, so nothing is lost and no confirm is asked.
    pub(super) fn empty_fork_remove(&mut self, cx: &mut Context<'_, Self>) {
        let forking = &mut self.areas.conversation.forking;
        if forking.removing {
            return;
        }
        forking.removing = true;
        self.act(Act::Delete);
        cx.notify();
    }

    /// An empty fork's Remove answered, for the session shown: once it is
    /// gone, the session it was forked from opens on the next frame, when
    /// the list holds it (where the person forked from); a refusal is
    /// said under the banner.
    pub(super) fn empty_fork_removed(&mut self, answer: &ActAnswer) {
        if answer.kind != ActKind::Delete || !self.areas.conversation.forking.removing {
            return;
        }
        self.areas.conversation.forking.removing = false;
        match &answer.result {
            Ok(Acted::Deleted(_)) => {
                let origin =
                    origin_of(self.areas.conversation.timeline.entries()).map(|(from, _)| from);
                let card = self
                    .fleet
                    .machines
                    .iter()
                    .filter(|machine| machine.name == answer.machine)
                    .flat_map(|machine| machine.sessions.iter())
                    .find(|card| Some(card.session) == origin);
                self.areas.conversation.forking.opening = card.map(Open::of);
            }
            // A refusal is said under the banner (the conversation's
            // `refused`); a lost connection is the bar's.
            Err(_)
            | Ok(
                Acted::Session(_)
                | Acted::Done
                | Acted::Conversation(_)
                | Acted::Forked(_)
                | Acted::Log(_),
            ) => {}
        }
    }

    /// The new session opened, its composer focused (Paseo opens the
    /// fork); called on a frame with a window.
    pub(super) fn forked_open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(open) = self.areas.conversation.forking.opening.take() {
            self.open_session(open, window, cx);
        }
    }
}

// Owner snapshot preparation; no staged exemption in the product path.
#[cfg(test)]
impl Forking {
    pub(super) fn snapshot_busy(&self) -> bool {
        self.pending.is_some() || self.opening.is_some() || self.removing
    }
}

#[cfg(test)]
mod language_tests;
