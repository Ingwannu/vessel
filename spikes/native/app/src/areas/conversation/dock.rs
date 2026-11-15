//! The slot above the session's composer card, in the card's column: what
//! the conversation shows over the composer: an ended session's banner
//! (the page's), a question the agent asks (Paseo's question card), and
//! the queue of prompts waiting for the turn to end (Paseo's
//! `composer/queue`).
//!
//! [Decision Log]
//! - Purpose: the composer card is the composer track's; what waits to be
//!   sent is the conversation's. One slot with one owner each side keeps
//!   both from editing the other's drawing.
//! - Choice: the session's layout (`areas/session.rs`) asks this module
//!   for the slot's content and puts it in the composer's column, right
//!   above the card, at the card's width; nothing shows while it is
//!   `None`.

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use super::{Place, SessionPlace};
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;

/// Preserve composer height while every pending card remains scrollable.
const REQUEST_HEIGHT_FRACTION: f32 = 0.5;

/// What shows right above the composer card; `None`: nothing. An ended
/// session's banner (`ended`), a permission the agent waits for, a question it asks ([`requests`]),
/// then the queue of prompts waiting their turn (`queue`), nearest the
/// card.
pub(crate) fn dock(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let ended = super::ended::banner(shell, tokens, window, cx);
    let requests = requests::<SessionPlace>(shell, tokens, window, cx);
    let queue = super::queue::queue(shell, tokens, window, cx);
    if ended.is_none() && requests.is_empty() && queue.is_none() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_col()
            .children(ended.map(|banner| div().pb(px(8.0)).child(banner)))
            .children(
                requests
                    .into_iter()
                    .map(|card| div().pb(px(8.0)).child(card)),
            )
            .children(queue)
            .into_any_element(),
    )
}

/// What waits on the person in the conversation at `P`, as its cards: the
/// permissions (Paseo's permission card, or its plan card),
/// then questions (Paseo's question card); none when nothing
/// waits. Each answers the session that conversation shows.
///
/// For another area answering a session's requests in place (the inbox):
/// keep a `ConversationArea` of your own at a [`Place`] of your own, open
/// it on the session, give it the session's records and what the node
/// says waits (`Act::Conversation`'s answer, `live`), and draw these. Put
/// them inside an element with an id of its own: the cards' ids are the
/// same wherever they are drawn.
pub(crate) fn requests<P: Place>(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    if (P::SEAT.get)(shell).is_none_or(|area| !area.response_online(&shell.fleet)) {
        return Vec::new();
    }
    match P::response_scope(shell) {
        vessel_app_view::responses::ResponseScope::All => {
            let permission = super::permission::card::<P>(shell, tokens, window, cx);
            let question = super::question::card::<P>(shell, tokens, window, cx);
            let cards: Vec<_> = permission.into_iter().chain(question).collect();
            if cards.is_empty() {
                return cards;
            }
            // The slot shares height with the composer; all cards remain reachable.
            vec![
                div()
                    .id("pending-request-stack")
                    .max_h(window.viewport_size().height * REQUEST_HEIGHT_FRACTION)
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::S2))
                    .children(cards)
                    .into_any_element(),
            ]
        }
        vessel_app_view::responses::ResponseScope::One(request) => {
            let card = match request {
                vessel_app_view::responses::Request::Permission(id) => {
                    super::permission::card_at::<P>(shell, id, tokens, window, cx)
                }
                vessel_app_view::responses::Request::Question(id) => {
                    super::question::card_at::<P>(shell, id, tokens, window, cx)
                }
            };
            card.into_iter().collect()
        }
    }
}
