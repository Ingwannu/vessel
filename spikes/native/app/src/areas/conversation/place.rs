//! Where a conversation's state is kept ([`Seat`]) and how the person's
//! answers on a request's card are sent from there ([`Place`]): the
//! session area's own conversation, or another area's (a side question's
//! panel, the inbox). See the parent module's second Decision Log.

use vessel_app_kit::ui::Context;
use vessel_app_link::Act;
use vessel_app_view::timeline::ToolView;
use vessel_proto::{PermissionId, PermissionOptionId, QuestionAnswer, QuestionId, ToolCallId};

use super::ConversationArea;
use crate::shell::Shell;

/// Where a conversation's state is kept, for the rows drawn from it and
/// the toggles they answer: the session area's ([`Seat::SESSION`]), or
/// another area's own [`ConversationArea`] (a side question's panel).
/// `get` and `get_mut` must reach the same state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seat {
    /// The state, to draw from; `None` once it is gone (a panel closed).
    pub(crate) get: fn(&Shell) -> Option<&ConversationArea>,
    /// The same state, to change.
    pub(crate) get_mut: fn(&mut Shell) -> Option<&mut ConversationArea>,
}

impl Seat {
    /// The session area's conversation.
    pub(crate) const SESSION: Self = Self {
        get: session_seat,
        get_mut: session_seat_mut,
    };

    /// The session area's fork's origin, once read (`origin`): its
    /// rows are the list's first, read-only.
    pub(crate) const ORIGIN: Self = Self {
        get: origin_seat,
        get_mut: origin_seat_mut,
    };
}

fn origin_seat(shell: &Shell) -> Option<&ConversationArea> {
    shell.areas.conversation.origin.area()
}

fn origin_seat_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    shell.areas.conversation.origin.area_mut()
}

/// An answer the person gave on a request's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Answer {
    /// A permission request's option.
    Permit(PermissionId, PermissionOptionId),
    /// A question's form, filled in or declined.
    Question(QuestionId, QuestionAnswer),
}

/// A [`Seat`] known by its type, for the parts whose controls take plain
/// functions (a question card's text box calls a function, which cannot
/// carry a seat): each place gets its own copy of those functions. The
/// session area's is [`SessionPlace`]; another area answering a
/// conversation's requests in place (the inbox) names its own, and may
/// send the answers its own way.
pub(crate) trait Place: 'static {
    /// Where its conversation is kept.
    const SEAT: Seat;

    /// Send `answer` for the session the conversation shows. The card has
    /// already marked it as going (`permit_sending`, the question's
    /// `sent`); a failure is said on the card by setting the
    /// conversation's `refused` to the act's kind and its words. By
    /// default, the session's own act.
    fn answer(shell: &mut Shell, answer: Answer, cx: &mut Context<'_, Shell>) {
        let act = match answer {
            Answer::Permit(id, option) => Act::Permit(id, option),
            Answer::Question(id, answer) => Act::Answer(id, answer),
        };
        shell.act_at(Self::SEAT, act);
        cx.notify();
    }

    /// Whether the cards hold every answer back (one of the place's own
    /// is still on its way). By default, never.
    fn busy(_shell: &Shell) -> bool {
        false
    }

    /// The tool call a request is for, as its card shows it (its title,
    /// its command or diff): by default the conversation's own fold of
    /// it. An area holding a request without its session's log (the
    /// inbox) gives the call it keeps, never a record made up for it.
    fn tool<'a>(shell: &'a Shell, id: &ToolCallId) -> Option<&'a ToolView> {
        (Self::SEAT.get)(shell).and_then(|area| area.timeline.tool_call(id))
    }
}

/// The session area's conversation, as a [`Place`].
pub(crate) struct SessionPlace;

impl Place for SessionPlace {
    const SEAT: Seat = Seat::SESSION;
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session_seat(shell: &Shell) -> Option<&ConversationArea> {
    Some(&shell.areas.conversation)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session_seat_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    Some(&mut shell.areas.conversation)
}
