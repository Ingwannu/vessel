//! Rewinding the conversation from a person's message (slice R1, protocol
//! 14): Paseo's trailing row under the message (`components/message.tsx`
//! `UserMessage`: its time, the rewind menu's undo mark, copy) and its
//! rewind menu (`rewind/rewind-menu.tsx`: "This action cannot be undone",
//! then "Rewind conversation"). The rule for where a message can go is
//! the view's (`timeline::rewind_point`).
//!
//! [Decision Log]
//! - Purpose: a person takes the conversation back to before one of
//!   their messages, as in Paseo.
//! - The reference: the row shows on hover (always on a phone); the menu
//!   locks while its call goes and closes after; on success the rewound
//!   message goes back into the composer when it is empty
//!   (`composer-restore.tsx`); a failure is a toast with the error's words.
//! - Choice: the same, with the kit's menu (its warning a disabled first
//!   row, the action after a hairline) and the app's notice for the
//!   failure. The composer is set on the frame after the answer, which
//!   has a window (`restore`).
//! - Differences, kept: one mode, the conversation (the node does not put
//!   files back).
//! - Tooltip: the kit's, one per screen, with a place per message kept
//!   here (the mark is redrawn each frame; the tooltip must find where it
//!   was last drawn).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{DateTime, Local, Utc};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Point, Stateful,
    Styled, Window, div, px,
};
use vessel_app_kit::{CopyLook, MenuHandlers, MenuItem, MenuState, Tokens, Tooltips, copy_button};
use vessel_app_link::{Act, ActAnswer, ActFailed, ActKind};
use vessel_app_view::timeline::rewind_point;
use vessel_proto::EventSeq;

use super::entries::{Paint, id};
use crate::shell::Shell;

/// The trailing row's height (Paseo's 24-point icon buttons).
const ROW: f32 = 24.0;

/// The undo mark's corner: six points, between the kit's `R1` and `R2`;
/// the kit has no radius of this size yet.
const MARK_RADIUS: f32 = 6.0;

/// A rewind being chosen or sent.
pub(crate) struct Rewinding {
    /// The menu under the undo mark.
    pub(super) menu: MenuState,
    /// The message the menu is for.
    target: Option<Target>,
    /// The message whose rewind is on its way.
    pending: Option<usize>,
    /// A rewound message's words, to go back into the composer.
    restore: Option<String>,
    /// Retained across navigation while its frame notice may still be visible.
    notice: Option<vessel_app_view::rewind::FailureNotice>,
    /// The undo marks' tooltip (one shown at a time).
    pub(super) tips: Tooltips,
    /// Where each message's undo mark was last drawn, by entry.
    places: RefCell<HashMap<usize, Place>>,
}

/// The message a rewind menu is for.
#[derive(Debug, Clone)]
struct Target {
    entry: usize,
    after: EventSeq,
    text: String,
}

impl std::fmt::Debug for Rewinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rewinding")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Rewinding {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            pending: None,
            restore: None,
            notice: None,
            tips: Tooltips::new(),
            places: RefCell::default(),
        }
    }

    /// The undo mark's place for the message at `entry`, kept across
    /// frames so its tooltip stays where the mark is.
    fn place(&self, entry: usize) -> Place {
        Rc::clone(self.places.borrow_mut().entry(entry).or_default())
    }

    /// Whether a rewound message waits to go back into the composer.
    pub(super) const fn has_restore(&self) -> bool {
        self.restore.is_some()
    }

    /// Nothing chosen or on its way (another session shows).
    pub(super) fn clear(&mut self) {
        self.menu.close();
        self.target = None;
        self.pending = None;
        self.restore = None;
        self.tips = Tooltips::new();
        self.places.borrow_mut().clear();
    }
}

/// The person's message at `index` (`bubble`) with Paseo's trailing row
/// under it: its time, rewind where it can go, copy. The row shows while
/// the message is pointed at (always on a phone, and while its menu is
/// open or its rewind goes).
pub(super) fn with_row(
    paint: &Paint<'_>,
    index: usize,
    (bubble, text, at): (AnyElement, &str, DateTime<Utc>),
    rewinding: &Rewinding,
    narrow: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let group = format!("person-{index}");
    let held = rewinding.pending == Some(index)
        || (rewinding.menu.is_open()
            && rewinding
                .target
                .as_ref()
                .is_some_and(|target| target.entry == index));
    let muted = paint.tokens.neutral.foreground_muted;
    let after = paint
        .rewinds
        .and_then(|timeline| rewind_point(timeline, index));
    let row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1))
        .h(px(ROW))
        .when(!narrow && !held, |row| {
            row.invisible().group_hover(group.clone(), Styled::visible)
        })
        .child(
            div()
                .text_size(px(vessel_app_kit::fonts::ui(12.0)))
                .text_color(muted)
                .child(paint.words.when(at, Local::now().fixed_offset())),
        )
        .children(after.map(|after| {
            let mark = undo(
                paint.tokens,
                index,
                (after, text.to_owned()),
                rewinding.pending.is_some(),
                cx,
            );
            // Paseo's tooltip on the mark ("Rewind to this message"),
            // the kit's; none on a phone (no pointer).
            rewinding
                .tips
                .on(
                    mark,
                    &rewinding.place(index),
                    paint.catalog.rewind().tooltip(),
                    narrow,
                )
                .into_any_element()
        }))
        .child(copy_button(
            ElementId::NamedInteger("person-copy".into(), id(index)),
            text.to_owned().into(),
            CopyLook {
                glyph: muted,
                hover: paint.tokens.neutral.surface2,
            },
            window,
            cx,
        ));
    div()
        .group(group)
        .flex()
        .flex_col()
        .items_end()
        .gap(px(spacing::S0_5))
        .child(bubble)
        .child(row)
        .into_any_element()
}

/// The undo mark that opens the rewind menu for the message at `index`;
/// dimmed and still while a rewind goes (Paseo's locked trigger).
fn undo(
    tokens: &Tokens,
    index: usize,
    (after, text): (EventSeq, String),
    locked: bool,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    div()
        .id(ElementId::NamedInteger("rewind".into(), id(index)))
        .size(px(ROW))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(MARK_RADIUS))
        .when(locked, |mark| mark.opacity(0.5))
        .when(!locked, |mark| {
            mark.cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                        shell.rewind_open(
                            Target {
                                entry: index,
                                after,
                                text: text.clone(),
                            },
                            event.position,
                            window,
                            cx,
                        );
                    }),
                )
        })
        .child(glyph(Glyph::Undo2, 16.0, neutral.foreground_muted))
}

/// The rewind menu's rows: Paseo's warning (shown, not chosen), then the
/// one mode our node has.
pub(super) fn menu_items(shell: &Shell) -> Vec<MenuItem> {
    let words = shell.words.rewind();
    vec![
        MenuItem {
            disabled: true,
            ..MenuItem::new(words.warning())
        },
        MenuItem::new(words.conversation()).grouped(),
    ]
}

impl Shell {
    /// Reword only the frame notice still owned by the last rewind refusal.
    pub(crate) fn translate_rewind_notice(&mut self) {
        let Some(notice) = self.areas.conversation.rewinding.notice.as_mut() else {
            return;
        };
        let previous = notice.words();
        let visible = self.notice.as_deref() == Some(previous.as_ref());
        notice.translate(self.words);
        if visible {
            self.notice = Some(notice.words().to_string());
        }
    }

    /// Open the rewind menu for `target` at `at`.
    fn rewind_open(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rewinding = &mut self.areas.conversation.rewinding;
        if rewinding.pending.is_some() {
            return;
        }
        rewinding.target = Some(target);
        rewinding.menu.open(at, window, cx);
        cx.notify();
    }

    /// The rewind menu's handlers: its state, and what its row does.
    pub(super) fn rewind_menu_handlers() -> MenuHandlers<Self> {
        MenuHandlers {
            state: |shell: &mut Self| &mut shell.areas.conversation.rewinding.menu,
            chose: Rc::new(|shell: &mut Self, index, _window, cx| {
                // Row 0 is the warning; row 1 rewinds.
                if index == 1 {
                    shell.rewind_send(cx);
                }
            }),
        }
    }

    /// Rewind to the menu's message.
    fn rewind_send(&mut self, cx: &mut Context<'_, Self>) {
        let rewinding = &mut self.areas.conversation.rewinding;
        rewinding.menu.close();
        let Some(target) = rewinding.target.clone() else {
            return;
        };
        rewinding.pending = Some(target.entry);
        self.act(Act::Rewind(target.after));
        cx.notify();
    }

    /// A rewind's answer for the session shown: the lock lifts; on success
    /// the message's words go back into the composer (when it is empty,
    /// on the next frame); a refusal is the notice's, in Paseo's words.
    pub(super) fn rewind_answered(&mut self, answer: &ActAnswer) {
        if answer.kind != ActKind::Rewind {
            return;
        }
        let rewinding = &mut self.areas.conversation.rewinding;
        rewinding.pending = None;
        let target = rewinding.target.take();
        match &answer.result {
            Ok(_) => rewinding.restore = target.map(|target| target.text),
            Err(ActFailed {
                code: Some(code),
                message,
            }) => {
                tracing::warn!(?code, %message, "rewind was refused");
                let notice =
                    vessel_app_view::rewind::FailureNotice::new((*code).into(), self.words);
                self.notice = Some(notice.words().to_string());
                rewinding.notice = Some(notice);
            }
            // No answer at all: the session's lost connection says it.
            Err(ActFailed { code: None, .. }) => {}
        }
    }

    /// The rewound message into the composer, when it is empty (Paseo's
    /// `restoreComposerTextIfEmpty`); called on a frame with a window.
    pub(super) fn rewind_restore(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(text) = self.areas.conversation.rewinding.restore.take() else {
            return;
        };
        let composer = &self.areas.session.composer;
        if composer.text(cx).is_empty() {
            composer.set_text(&text, window, cx);
        }
    }
}

#[cfg(test)]
#[path = "rewind/language_tests.rs"]
mod language_tests;

// Owner snapshot preparation; no staged exemption in the product path.
#[cfg(test)]
impl Rewinding {
    pub(super) fn snapshot_busy(&self) -> bool {
        self.pending.is_some() || self.restore.is_some()
    }
}
