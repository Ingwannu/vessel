//! Paseo focus-entry/input/send/blur clear policy. Attention arriving while
//! already viewed is deferred; requests and explicit unread are never erased
//! merely by repainting the same screen.
use super::Read;
use vessel_proto::{EventSeq, NodeId, SessionId};
/// Pane selection and window visibility are one state, not independent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Selected in the visible active window.
    Active,
    /// Selected while the app is hidden or unfocused.
    Inactive,
    /// The pane remains resident but is not selected.
    Unfocused,
}
impl Focus {
    fn active(self) -> bool {
        self == Self::Active
    }
    fn focused(self) -> bool {
        matches!(self, Self::Active | Self::Inactive)
    }
}
/// Live requests veto passive attention clearing independently of read state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requests {
    /// No current permission or question.
    None,
    /// A live permission or question still requires a response.
    Waiting,
}

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
    /// Pane selection and active visibility.
    pub focus: Focus,
    /// Current node-reported attention/read state.
    pub read: Read,
    /// Live permission/question state.
    pub requests: Requests,
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
            if old.is_some_and(|view| view.focus.focused()) {
                change.leaving = self.action(false);
            }
            self.current = next;
            self.deferred = false;
            self.held = None;
            self.sent = None;
            if next.is_some_and(|view| view.focus.active()) {
                change.entering = self.action(true);
            }
            return change;
        }
        self.current = next;
        let (Some(old), Some(next)) = (old, next) else {
            return change;
        };
        if next.read == Read::Read {
            self.deferred = false;
        }
        if old.read == Read::Read
            && next.read == Read::Unread
            && old.focus.active()
            && next.focus.active()
        {
            self.deferred = true;
        }
        if old.focus.focused() && !next.focus.focused() {
            change.leaving = self.action(false);
        }
        if next.focus.active() && (!old.focus.active() || !old.focus.focused()) {
            if !old.focus.focused() {
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
        if !self.current.is_some_and(|view| view.focus.active()) {
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
        if view.read == Read::Read
            || view.requests == Requests::Waiting
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
            focus: Focus::Active,
            read: if unread { Read::Unread } else { Read::Read },
            requests: Requests::None,
            through: EventSeq::new(10),
        }
    }
    #[test]
    fn entering_clears_old_attention_but_in_place_attention_waits_for_input_send_or_blur() {
        let mut reader = Reader::default();
        let initial = view(true);
        assert!(reader.observe(Some(initial)).entering.is_some());
        let read = Observation {
            read: Read::Read,
            ..initial
        };
        assert_eq!(reader.observe(Some(read)), Change::default());
        let new = Observation {
            read: Read::Unread,
            through: EventSeq::new(20),
            ..read
        };
        assert_eq!(reader.observe(Some(new)), Change::default());
        assert_eq!(
            reader.observe(Some(Observation {
                focus: Focus::Inactive,
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
            read: Read::Read,
            ..new
        };
        let _change = reader.observe(Some(read));
        let next = Observation {
            read: Read::Unread,
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
                requests: Requests::Waiting,
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
            read: Read::Unread,
            ..current
        };
        let _change = reader.observe(Some(unread));
        assert_eq!(reader.trigger(Trigger::PromptSend), None);
        assert_eq!(reader.observe(None), Change::default());
        assert!(reader.observe(Some(unread)).entering.is_some());
    }
}
