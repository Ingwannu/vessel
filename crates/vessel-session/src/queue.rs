//! A structured session's queue (protocol 10, ADR 0024): the prompts that
//! wait for their turn.
//!
//! Owns: the entries, their order, their bounds, and whether the queue is
//! held. Must not: send or publish anything; the session's actor does
//! both (`delivery`), and publishes the whole queue after each change.
//!
//! An empty queue is never held (the contract's review, item 7): a hold
//! is about prompts that wait, so a message typed after a stop, with
//! nothing waiting, still starts.

use chrono::Utc;
use vessel_proto::{
    ContentBlock, Creator, Delivery, PromptQueue, QueueChange, QueueEntryId, QueueHold,
    QueuedPrompt,
};

/// Why a queue change, or a prompt for the queue, was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueueError {
    /// No room: [`PromptQueue::MAX_ENTRIES`] prompts wait already.
    #[error("the queue is full (at most {} prompts)", PromptQueue::MAX_ENTRIES)]
    Full,
    /// The prompts would weigh more than [`PromptQueue::MAX_BYTES`].
    #[error(
        "the queue's prompts would weigh more than {} bytes",
        PromptQueue::MAX_BYTES
    )]
    TooLarge,
    /// The entry named is not in the queue (sent, or removed).
    #[error("no such prompt in the queue; it was sent or removed")]
    NoEntry,
    /// The entry was edited since the revision the edit was made from.
    #[error("the prompt was edited meanwhile (now revision {current}); edit that one")]
    Stale {
        /// Its revision now.
        current: u32,
    },
    /// `send_now` with `queue`, which is not a way to send now.
    #[error("a prompt is sent now with start, steer or interrupt, not queue")]
    NotNow,
}

/// What a change did.
#[derive(Debug)]
pub(crate) enum Applied {
    /// The queue is as it now is (possibly unchanged).
    Changed,
    /// An entry was taken out to be sent now; it goes back to `place` if
    /// sending fails.
    Taken {
        entry: QueuedPrompt,
        place: usize,
        delivery: Delivery,
    },
    /// It may move again: between turns, its first entry starts.
    Resumed,
}

/// The queue and the rules that change it.
#[derive(Debug, Default)]
pub(crate) struct Queue(PromptQueue);

impl Queue {
    /// A queue as a session's log left it (`session.resume` brings one
    /// back held).
    pub(crate) const fn from(queue: PromptQueue) -> Self {
        Self(queue)
    }

    /// The queue as published.
    pub(crate) const fn get(&self) -> &PromptQueue {
        &self.0
    }

    pub(crate) const fn is_held(&self) -> bool {
        self.0.held.is_some()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.0.entries.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn push(
        &mut self,
        content: Vec<ContentBlock>,
        by: Creator,
    ) -> Result<QueueEntryId, QueueError> {
        self.push_admitted(content, by, None, None)
    }

    /// Preserve an already-admitted entry's exact local presentation link.
    /// This private constructor performs no permission or receipt admission.
    pub(crate) fn push_admitted(
        &mut self,
        content: Vec<ContentBlock>,
        by: Creator,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
    ) -> Result<QueueEntryId, QueueError> {
        let entry = QueuedPrompt {
            provenance,
            id: QueueEntryId::random(),
            content,
            queued_at: Utc::now(),
            revision: 0,
            by,

            client_message_id,
        };
        let id = entry.id;
        self.insert(self.0.entries.len(), entry)?;
        Ok(id)
    }

    /// Put a prompt first; the new entry's id.
    pub(crate) fn push_first_identified(
        &mut self,
        content: Vec<ContentBlock>,
        by: Creator,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        provenance: Option<vessel_proto::timeline::PromptProvenance>,
    ) -> Result<QueueEntryId, QueueError> {
        let id = self.push_admitted(content, by, client_message_id, provenance)?;
        let _first = self.relocate(id, self.0.entries.first().map(|first| first.id))?;
        Ok(id)
    }

    /// Put `entry` at `place` (the end if past it), within the bounds.
    pub(crate) fn insert(&mut self, place: usize, entry: QueuedPrompt) -> Result<(), QueueError> {
        if self.0.entries.len() >= PromptQueue::MAX_ENTRIES {
            return Err(QueueError::Full);
        }
        let place = place.min(self.0.entries.len());
        self.0.entries.insert(place, entry);
        if self.too_heavy() {
            let _refused = self.0.entries.remove(place);
            return Err(QueueError::TooLarge);
        }
        Ok(())
    }

    /// Apply one `session.queue` change.
    pub(crate) fn apply(&mut self, change: QueueChange) -> Result<Applied, QueueError> {
        match change {
            QueueChange::Edit {
                entry,
                revision,
                content,
            } => self.edit(entry, revision, content),
            QueueChange::Move { entry, before } => self.relocate(entry, before),
            QueueChange::Remove { entry } => {
                let place = self.find(entry)?;
                let _removed = self.0.entries.remove(place);
                self.settle();
                Ok(Applied::Changed)
            }
            QueueChange::SendNow { entry, delivery } => {
                if delivery == Delivery::Queue {
                    return Err(QueueError::NotNow);
                }
                let place = self.find(entry)?;
                let entry = self.0.entries.remove(place);
                Ok(Applied::Taken {
                    entry,
                    place,
                    delivery,
                })
            }
            QueueChange::Hold => {
                let _held = self.hold(QueueHold::Asked);
                Ok(Applied::Changed)
            }
            QueueChange::Resume => {
                self.0.held = None;
                Ok(Applied::Resumed)
            }
            QueueChange::Clear => {
                self.0.entries.clear();
                self.0.held = None;
                Ok(Applied::Changed)
            }
        }
    }

    fn edit(
        &mut self,
        entry: QueueEntryId,
        revision: u32,
        content: Vec<ContentBlock>,
    ) -> Result<Applied, QueueError> {
        let place = self.find(entry)?;
        let Some(found) = self.0.entries.get_mut(place) else {
            return Err(QueueError::NoEntry);
        };
        if found.revision != revision {
            return Err(QueueError::Stale {
                current: found.revision,
            });
        }
        let provenance = found.provenance.take();
        let before = std::mem::replace(&mut found.content, content);
        let queued_at = std::mem::replace(&mut found.queued_at, Utc::now());
        found.revision = revision.saturating_add(1);
        if self.too_heavy() {
            if let Some(found) = self.0.entries.get_mut(place) {
                found.content = before;
                found.provenance = provenance;
                found.queued_at = queued_at;
                found.revision = revision;
            }
            return Err(QueueError::TooLarge);
        }
        Ok(Applied::Changed)
    }

    /// `move`: before another entry, or last. Both are checked before
    /// anything moves, so a refused move changes nothing.
    fn relocate(
        &mut self,
        entry: QueueEntryId,
        before: Option<QueueEntryId>,
    ) -> Result<Applied, QueueError> {
        let from = self.find(entry)?;
        if let Some(before) = before {
            let _there = self.find(before)?;
        }
        if before == Some(entry) {
            return Ok(Applied::Changed);
        }
        let moving = self.0.entries.remove(from);
        let to = match before {
            Some(before) => self.find(before)?,
            None => self.0.entries.len(),
        };
        self.0.entries.insert(to, moving);
        Ok(Applied::Changed)
    }

    /// Hold it for `why`, if anything waits and it moves; whether it
    /// changed.
    pub(crate) fn hold(&mut self, why: QueueHold) -> bool {
        if self.is_empty() || self.is_held() {
            return false;
        }
        self.0.held = Some(why);
        true
    }

    /// The session ended: whatever waits is held `ended`, whatever held
    /// it before; whether it changed.
    pub(crate) fn end(&mut self) -> bool {
        if self.is_empty() || self.0.held == Some(QueueHold::Ended) {
            return false;
        }
        self.0.held = Some(QueueHold::Ended);
        true
    }

    /// The first entry, taken out, when the queue moves.
    pub(crate) fn next(&mut self) -> Option<QueuedPrompt> {
        if self.is_held() || self.is_empty() {
            return None;
        }
        Some(self.0.entries.remove(0))
    }

    /// After an entry left: an empty queue is never held.
    pub(crate) fn settle(&mut self) {
        if self.is_empty() {
            self.0.held = None;
        }
    }

    fn find(&self, entry: QueueEntryId) -> Result<usize, QueueError> {
        self.0
            .entries
            .iter()
            .position(|waiting| waiting.id == entry)
            .ok_or(QueueError::NoEntry)
    }

    /// Whether the entries weigh more than [`PromptQueue::MAX_BYTES`] as
    /// JSON (what each `queue_changed` record carries).
    fn too_heavy(&self) -> bool {
        serde_json::to_vec(&self.0.entries).map_or(true, |json| json.len() > PromptQueue::MAX_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_committed_content_edit_clears_the_local_notice_association() {
        let mut queue = Queue::default();
        let id = queue.push(text("notice"), Creator::Owner).unwrap();
        let other = queue.push(text("ordinary"), Creator::Owner).unwrap();
        let provenance = Some(vessel_proto::timeline::PromptProvenance::InternalNotice {
            notice: vessel_proto::EventSeq::new(5),
        });
        queue.0.entries[0].provenance = provenance;
        assert!(matches!(
            queue.apply(QueueChange::Move {
                entry: id,
                before: None,
            }),
            Ok(Applied::Changed)
        ));
        assert_eq!(queue.get().entries[1].provenance, provenance);
        let Applied::Taken { entry, place, .. } = queue
            .apply(QueueChange::SendNow {
                entry: id,
                delivery: Delivery::Start,
            })
            .unwrap()
        else {
            panic!("taken");
        };
        assert_eq!(entry.provenance, provenance);
        queue.insert(place, entry).unwrap();
        assert!(matches!(
            queue.apply(QueueChange::Edit {
                entry: id,
                revision: 0,
                content: text(&"x".repeat(PromptQueue::MAX_BYTES)),
            }),
            Err(QueueError::TooLarge)
        ));
        assert_eq!(queue.get().entries[1].provenance, provenance);
        assert!(matches!(
            queue.apply(QueueChange::Edit {
                entry: id,
                revision: 1,
                content: text("stale"),
            }),
            Err(QueueError::Stale { .. })
        ));
        assert_eq!(queue.get().entries[1].provenance, provenance);
        assert!(matches!(
            queue.apply(QueueChange::Edit {
                entry: id,
                revision: 0,
                content: text("person edited"),
            }),
            Ok(Applied::Changed)
        ));
        assert_eq!(queue.get().entries[1].provenance, None);
        assert_eq!(queue.get().entries[0].id, other);
    }

    fn text(text: &str) -> Vec<ContentBlock> {
        vec![ContentBlock::Text {
            text: text.to_owned(),
        }]
    }

    fn texts(queue: &Queue) -> Vec<String> {
        queue
            .get()
            .entries
            .iter()
            .flat_map(|entry| entry.content.iter())
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.clone()),
                ContentBlock::ResourceLink { .. }
                | ContentBlock::Opaque { .. }
                | ContentBlock::Attachment { .. } => None,
            })
            .collect()
    }

    #[test]
    fn entries_move_edit_and_leave_by_id() {
        let mut queue = Queue::default();
        let a = queue.push(text("a"), Creator::Owner).unwrap();
        let b = queue.push(text("b"), Creator::Owner).unwrap();
        let c = queue.push(text("c"), Creator::Owner).unwrap();
        let _moved = queue
            .apply(QueueChange::Move {
                entry: c,
                before: Some(a),
            })
            .unwrap();
        assert_eq!(texts(&queue), ["c", "a", "b"]);
        let _moved = queue
            .apply(QueueChange::Move {
                entry: c,
                before: None,
            })
            .unwrap();
        assert_eq!(texts(&queue), ["a", "b", "c"]);
        let edit = |revision| QueueChange::Edit {
            entry: b,
            revision,
            content: text("B"),
        };
        let _edited = queue.apply(edit(0)).unwrap();
        assert_eq!(
            queue.apply(edit(0)).unwrap_err(),
            QueueError::Stale { current: 1 }
        );
        assert_eq!(texts(&queue), ["a", "B", "c"]);
        let _removed = queue.apply(QueueChange::Remove { entry: a }).unwrap();
        assert_eq!(
            queue.apply(QueueChange::Remove { entry: a }).unwrap_err(),
            QueueError::NoEntry
        );
        assert_eq!(
            queue
                .apply(QueueChange::Move {
                    entry: b,
                    before: Some(a),
                })
                .unwrap_err(),
            QueueError::NoEntry,
            "a refused move changes nothing"
        );
        assert_eq!(texts(&queue), ["B", "c"]);
    }

    #[test]
    fn a_hold_needs_something_waiting_and_an_empty_queue_lets_go() {
        let mut queue = Queue::default();
        assert!(!queue.hold(QueueHold::Cancelled), "nothing waits");
        let a = queue.push(text("a"), Creator::Owner).unwrap();
        assert!(queue.hold(QueueHold::Cancelled));
        assert!(!queue.hold(QueueHold::Failed), "the first reason stays");
        assert!(queue.end(), "except for the session's end");
        assert_eq!(queue.next(), None, "held");
        let Applied::Taken { entry, place, .. } = queue
            .apply(QueueChange::SendNow {
                entry: a,
                delivery: Delivery::Start,
            })
            .unwrap()
        else {
            panic!("taken");
        };
        assert_eq!(place, 0);
        queue.settle();
        assert!(!queue.is_held(), "empty, so not held");
        queue.insert(place, entry).unwrap();
        assert_eq!(queue.next().map(|entry| entry.id), Some(a));
        assert_eq!(
            queue
                .apply(QueueChange::SendNow {
                    entry: a,
                    delivery: Delivery::Queue,
                })
                .unwrap_err(),
            QueueError::NotNow
        );
    }

    #[test]
    fn the_queue_is_bounded_in_entries_and_bytes() {
        let mut queue = Queue::default();
        for _ in 0..PromptQueue::MAX_ENTRIES {
            let _queued = queue.push(text("x"), Creator::Owner).unwrap();
        }
        assert_eq!(
            queue.push(text("x"), Creator::Owner).unwrap_err(),
            QueueError::Full
        );
        let mut heavy = Queue::default();
        let half = "y".repeat(PromptQueue::MAX_BYTES.div_ceil(2));
        let _queued = heavy.push(text(&half), Creator::Owner).unwrap();
        assert_eq!(
            heavy.push(text(&half), Creator::Owner).unwrap_err(),
            QueueError::TooLarge
        );
        assert_eq!(heavy.get().entries.len(), 1, "the refused one is not kept");
    }
}
