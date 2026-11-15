//! Paseo focus-entry/input/send/blur clear policy. Attention arriving while
//! already viewed is deferred; requests and explicit unread are never erased
//! merely by repainting the same screen.
use vessel_proto::{EventSeq, NodeId, SessionId};
/// A session's location, independent of the route's reused display name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// Authenticated owning node.
    pub node: NodeId,
    /// Session whose current view was observed.
    pub session: SessionId,
}
/// Facts read from one current conversation and its containing window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// Exact source location.
    pub target: Target,
    /// The pane remains selected, even while the app is in the background.
    pub focused: bool,
    /// The selected pane is visible in the active app window.
    pub active: bool,
    /// Current node-reported attention/read state.
    pub unread: bool,
    /// A permission or question is still waiting for a person.
    pub pending: bool,
    /// Exclusive watermark of the observed log, never a future event.
    pub through: EventSeq,
}
/// A clear uses only the watermark actually observed at this source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// Source to resolve through the authenticated registry.
    pub target: Target,
    /// Events strictly before this watermark may be marked read.
    pub through: EventSeq,
}
/// A navigation change can leave one session and enter another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    /// Clear the pane being left, including deferred attention.
    pub leaving: Option<Action>,
    /// Clear existing attention on focus entry; never new in-place attention.
    pub entering: Option<Action>,
}
/// Explicit composer events which can clear deferred attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// The person focuses the composer.
    InputFocus,
    /// The person sends a prompt.
    PromptSend,
}
/// One current pane's clear state. All timing is driven by real UI events.
#[derive(Debug, Default)]
pub struct Reader {
    current: Option<Observation>,
    deferred: bool,
    held: Option<EventSeq>,
    sent: Option<EventSeq>,
}
impl Reader {
    /// Update observed state without synthesizing focus/input from a redraw.
    #[must_use]
    pub fn observe(&mut self, next: Option<Observation>) -> Change {
        let old = self.current;
        let mut change = Change::default();
        if old.map(|view| view.target) != next.map(|view| view.target) {
            if old.is_some_and(|view| view.focused) {
                change.leaving = self.action(false);
            }
            self.current = next;
            self.deferred = false;
            self.held = None;
            self.sent = None;
            if next.is_some_and(|view| view.active) {
                change.entering = self.action(true);
            }
            return change;
        }
        self.current = next;
        let (Some(old), Some(next)) = (old, next) else {
            return change;
        };
        if !next.unread {
            self.deferred = false;
        }
        if !old.unread && next.unread && old.active && next.active {
            self.deferred = true;
        }
        if old.focused && !next.focused {
            change.leaving = self.action(false);
        }
        if next.active && (!old.active || !old.focused) {
            if !old.focused {
                self.held = None;
            }
            change.entering = self.action(true);
        }
        change
    }
    /// Input/send can clear deferred attention, but never pending requests.
    #[must_use]
    pub fn trigger(&mut self, trigger: Trigger) -> Option<Action> {
        match trigger {
            Trigger::InputFocus | Trigger::PromptSend => {}
        }
        if !self.current.is_some_and(|view| view.active) {
            return None;
        }
        self.action(false)
    }
    /// Keep a person's explicit unread choice while that same pane stays open.
    pub fn hold(&mut self, target: Target, through: EventSeq) {
        if self.current.is_some_and(|view| view.target == target) {
            self.held = Some(through);
            self.sent = None;
        }
    }
    /// The source presently observed, for safe late-result feedback.
    #[must_use]
    pub fn target(&self) -> Option<Target> {
        self.current.map(|view| view.target)
    }
    fn action(&mut self, focus_entry: bool) -> Option<Action> {
        let view = self.current?;
        if !view.unread
            || view.pending
            || view.through == EventSeq::ZERO
            || (focus_entry && self.deferred)
            || self.held.is_some_and(|held| view.through <= held)
            || self.sent.is_some_and(|sent| view.through <= sent)
        {
            return None;
        }
        self.sent = Some(view.through);
        self.deferred = false;
        Some(Action {
            target: view.target,
            through: view.through,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn view(unread: bool) -> Observation {
        Observation {
            target: Target {
                node: NodeId::random(),
                session: SessionId::random(),
            },
            focused: true,
            active: true,
            unread,
            pending: false,
            through: EventSeq::new(10),
        }
    }
    #[test]
    fn entering_clears_old_attention_but_in_place_attention_waits_for_input_send_or_blur() {
        let mut reader = Reader::default();
        let initial = view(true);
        assert!(reader.observe(Some(initial)).entering.is_some());
        let read = Observation {
            unread: false,
            ..initial
        };
        assert_eq!(reader.observe(Some(read)), Change::default());
        let new = Observation {
            unread: true,
            through: EventSeq::new(20),
            ..read
        };
        assert_eq!(reader.observe(Some(new)), Change::default());
        assert_eq!(
            reader.observe(Some(Observation {
                active: false,
                ..new
            })),
            Change::default()
        );
        assert_eq!(reader.observe(Some(new)), Change::default());
        assert_eq!(
            reader.trigger(Trigger::InputFocus).unwrap().through,
            EventSeq::new(20)
        );
        assert_eq!(reader.trigger(Trigger::PromptSend), None);
        let read = Observation {
            unread: false,
            ..new
        };
        let _change = reader.observe(Some(read));
        let next = Observation {
            unread: true,
            through: EventSeq::new(30),
            ..read
        };
        let _change = reader.observe(Some(next));
        assert_eq!(
            reader.observe(None).leaving.unwrap().through,
            EventSeq::new(30)
        );
    }
    #[test]
    fn pending_permissions_and_questions_never_clear_on_view_or_input_or_leave() {
        for trigger in [Trigger::InputFocus, Trigger::PromptSend] {
            let mut reader = Reader::default();
            let pending = Observation {
                pending: true,
                ..view(true)
            };
            assert_eq!(reader.observe(Some(pending)), Change::default());
            assert_eq!(reader.trigger(trigger), None);
            assert_eq!(reader.observe(None), Change::default());
        }
    }
    #[test]
    fn explicit_unread_survives_own_blur_and_repainting_but_a_returned_pane_can_clear_it() {
        let mut reader = Reader::default();
        let current = view(false);
        let _change = reader.observe(Some(current));
        reader.hold(current.target, current.through);
        let unread = Observation {
            unread: true,
            ..current
        };
        let _change = reader.observe(Some(unread));
        assert_eq!(reader.trigger(Trigger::PromptSend), None);
        assert_eq!(reader.observe(None), Change::default());
        assert!(reader.observe(Some(unread)).entering.is_some());
    }
}
