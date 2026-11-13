//! The slot above the session's composer card, in the card's column: what
//! the conversation shows over the composer (the queue of prompts waiting
//! for the turn to end, as Paseo's `composer/queue`).
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
use vessel_app_kit::ui::{AnyElement, Context, Window};

use crate::shell::Shell;

/// What shows right above the composer card; `None`: nothing. Today the
/// queue of prompts waiting their turn (`queue`).
pub(crate) fn dock(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    super::queue::queue(shell, tokens, window, cx)
}
