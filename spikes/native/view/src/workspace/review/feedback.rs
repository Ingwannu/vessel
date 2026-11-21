//! "Send feedback (N)" (Paseo's `review/feedback/index.ts`): whether the
//! review can be sent, why not, and what a press does.
//!
//! Paseo's rules: no recipient first, then no connection, then no comment
//! left in the diff; a send needs kept comments, no reason against it and
//! no send on its way; a press sends at once to the one recipient and
//! opens the chooser when there are several. Counts: the comments kept,
//! and the ones the review holds (what the button counts).

/// Why the review cannot be sent (Paseo's `disabledReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// No agent of this workspace to send it to.
    NoAgents,
    /// The machine is not connected.
    Disconnected,
    /// No kept comment's line is in the diff now.
    NoContext,
}

/// What is known when the button is drawn or pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackFacts {
    /// Comments kept for the diff shown.
    pub comments: usize,
    /// Comments the review would hold; `None` when it holds none.
    pub sendable: Option<usize>,
    /// Agents it can go to.
    pub recipients: usize,
    /// Whether the machine is connected.
    pub connected: bool,
    /// Whether a send is on its way.
    pub sending: bool,
}

/// The button's state (Paseo's `deriveState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackState {
    /// Why not, when it cannot be sent.
    pub unavailable: Option<Unavailable>,
    /// Whether a press may send.
    pub can_send: bool,
}

/// What a press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// Nothing: it cannot be sent.
    Nothing,
    /// Send to the one recipient.
    Send,
    /// Open the chooser of recipients.
    Choose,
}

impl FeedbackFacts {
    /// The button's state.
    #[must_use]
    pub const fn state(self) -> FeedbackState {
        let unavailable = if self.recipients == 0 {
            Some(Unavailable::NoAgents)
        } else if !self.connected {
            Some(Unavailable::Disconnected)
        } else if self.sendable.is_none() {
            Some(Unavailable::NoContext)
        } else {
            None
        };
        FeedbackState {
            unavailable,
            can_send: self.comments > 0 && unavailable.is_none() && !self.sending,
        }
    }

    /// What a press does now.
    #[must_use]
    pub const fn press(self) -> Press {
        if !self.state().can_send {
            Press::Nothing
        } else if self.recipients == 1 {
            Press::Send
        } else {
            Press::Choose
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY: FeedbackFacts = FeedbackFacts {
        comments: 2,
        sendable: Some(2),
        recipients: 1,
        connected: true,
        sending: false,
    };

    #[test]
    fn one_recipient_sends_at_once_and_several_open_the_chooser() {
        assert_eq!(READY.press(), Press::Send);
        let several = FeedbackFacts {
            recipients: 3,
            ..READY
        };
        assert_eq!(several.press(), Press::Choose);
        assert_eq!(READY.state().unavailable, None);
    }

    #[test]
    fn the_reasons_come_in_paseos_order() {
        let none = FeedbackFacts {
            recipients: 0,
            connected: false,
            sendable: None,
            ..READY
        };
        assert_eq!(none.state().unavailable, Some(Unavailable::NoAgents));
        let offline = FeedbackFacts {
            connected: false,
            sendable: None,
            ..READY
        };
        assert_eq!(offline.state().unavailable, Some(Unavailable::Disconnected));
        let gone = FeedbackFacts {
            sendable: None,
            ..READY
        };
        assert_eq!(gone.state().unavailable, Some(Unavailable::NoContext));
        assert_eq!(gone.press(), Press::Nothing);
    }

    #[test]
    fn nothing_is_sent_twice_or_without_comments() {
        let sending = FeedbackFacts {
            sending: true,
            ..READY
        };
        assert!(!sending.state().can_send);
        assert_eq!(sending.press(), Press::Nothing);
        let empty = FeedbackFacts {
            comments: 0,
            sendable: None,
            ..READY
        };
        assert!(!empty.state().can_send);
    }
}
