//! The slot above the session's composer card, in the card's column: what
//! the conversation shows over the composer: a question the agent asks
//! (Paseo's question card), and the queue of prompts waiting for the turn
//! to end (Paseo's `composer/queue`).
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
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};

use crate::shell::Shell;

/// What shows right above the composer card; `None`: nothing. A
/// question the agent asks (`question`), then the queue of prompts waiting
/// their turn (`queue`), nearest the card.
pub(crate) fn dock(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let question = super::question::card(shell, tokens, window, cx);
    let queue = super::queue::queue(shell, tokens, window, cx);
    if question.is_none() && queue.is_none() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_col()
            .children(question.map(|card| div().pb(px(8.0)).child(card)))
            .children(queue)
            .into_any_element(),
    )
}
