//! The prompts waiting their turn (protocol 10's queue, ADR 0024), above
//! the composer: Paseo's queue track (`composer/index.tsx`,
//! `QueuedMessageRow`) with what GOAT adds (`QueuedRunsControl.tsx`).
//!
//! [Decision Log]
//! - Purpose: a message written while the agent works waits its turn and
//!   can be changed there: edited, moved, taken out, sent into the
//!   running turn, or sent now in place of it.
//! - Constraint: the queue is the node's (every client sees one queue);
//!   the composer's card is the lead's and is not touched: the queue is
//!   drawn in the slot above it.
//! - The references: Paseo shows each waiting message as a row (surface1,
//!   a hairline, radius 8, 12 by 8 inside, the text on two lines) with
//!   two round buttons, edit (surface2) and send now (the accent); its
//!   edit moves the text back into the composer, losing its place. GOAT
//!   adds a grip to reorder by dragging, a steer for agents that take one
//!   during a turn, and remove; its edit keeps the place.
//! - Choice: Paseo's row and buttons, GOAT's actions as more round
//!   buttons of the same kind: edit in the row itself, keeping the
//!   place (the node's `edit`, checked against the revision it was made
//!   from); steer while a turn runs, for an agent that takes it; send now,
//!   which interrupts the running turn when one runs (its own choice, not
//!   steer's); remove; a grip to drag a row before another (the node's
//!   `move`, named by entry). A held queue says why and offers resume and
//!   clear.
//! - Keys, GOAT's: a click on the grip gives it the keys, and the arrow
//!   keys then move its row one place (the node's `move`), the grip
//!   keeping the keys as the row moves; the grip shows it has them in the
//!   foreground on `surface2`. GOAT's steer shortcut (Mod+Shift+Enter)
//!   and its edit of the latest message (Alt+Up from the composer's
//!   start) are the app's shortcuts' (track S2's registry), which call
//!   [`Shell::conversation_steer_next`] and
//!   [`Shell::conversation_edit_latest_queued`].
//! - Not yet: a queued message's attachments show as their names in its
//!   text (`[name]`), not as GOAT's thumbnails, which need the node's
//!   files read.

use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Act;
use vessel_app_view::timeline::block_text;
use vessel_proto::{ContentBlock, Delivery, QueueChange, QueueEntryId, QueuedPrompt};

use crate::shell::Shell;

mod rows;

pub(super) use rows::queue;

/// A queued message being edited in its row.
pub(crate) struct QueueEdit {
    entry: QueueEntryId,
    revision: u32,
    field: TextField,
}

impl std::fmt::Debug for QueueEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueEdit")
            .field("entry", &self.entry)
            .finish_non_exhaustive()
    }
}

/// A queued message's text, as its row shows it.
fn text_of(prompt: &QueuedPrompt) -> String {
    prompt
        .content
        .iter()
        .map(block_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Shell {
    /// Refresh the retained edit's hint without touching its text or keys.
    pub(crate) fn refresh_queue_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(edit) = &self.areas.conversation.queue_edit {
            edit.field
                .set_placeholder(self.t.lang.conversation().queue_edit(), window, cx);
        }
    }

    /// Ask the node to change the queue of the session shown.
    fn queue_change(&self, change: QueueChange) {
        self.act(Act::Queue(change));
    }

    /// Send the first queued message (GOAT's `thread.steerQueuedMessage`,
    /// Mod+Shift+Enter by default, in the app's shortcuts): into the
    /// running turn when the agent takes a steer, as a turn of its own
    /// when none runs; never in place of a running turn. Whether one
    /// went.
    pub(crate) fn conversation_steer_next(&self) -> bool {
        let facts = self.areas.conversation.timeline.facts();
        let Some(first) = facts.queue.as_ref().and_then(|queue| queue.entries.first()) else {
            return false;
        };
        let running = self.areas.session.conversation.turn_running();
        let delivery = if !running {
            Delivery::Start
        } else if facts
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.steer)
        {
            Delivery::Steer
        } else {
            return false;
        };
        self.queue_change(QueueChange::SendNow {
            entry: first.id,
            delivery,
        });
        true
    }

    /// Edit the latest queued message in its row (GOAT's
    /// `thread.editQueuedMessage`, Alt+Up from the composer's start, in
    /// the app's shortcuts); not while another is edited. Whether one
    /// opened.
    pub(crate) fn conversation_edit_latest_queued(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let area = &self.areas.conversation;
        if area.queue_edit.is_some() {
            return false;
        }
        let Some(latest) = area
            .timeline
            .facts()
            .queue
            .as_ref()
            .and_then(|queue| queue.entries.last())
            .cloned()
        else {
            return false;
        };
        self.queue_edit(&latest, window, cx);
        true
    }

    /// Edit `prompt` in its row.
    fn queue_edit(
        &mut self,
        prompt: &QueuedPrompt,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.conversation();
        let field = TextField::new(words.queue_edit(), Self::queue_edit_entered, window, cx);
        field.set_text(&text_of(prompt), window, cx);
        field.focus(window, cx);
        self.areas.conversation.queue_edit = Some(QueueEdit {
            entry: prompt.id,
            revision: prompt.revision,
            field,
        });
        cx.notify();
    }

    fn queue_edit_entered(
        &mut self,
        text: String,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(edit) = self.areas.conversation.queue_edit.take() else {
            return;
        };
        if !text.trim().is_empty() {
            self.queue_change(QueueChange::Edit {
                entry: edit.entry,
                revision: edit.revision,
                content: vec![ContentBlock::Text { text }],
            });
        }
        cx.notify();
    }

    fn queue_edit_cancelled(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.queue_edit.take().is_some() {
            cx.notify();
        }
    }
}

#[cfg(test)]
mod language_tests;
