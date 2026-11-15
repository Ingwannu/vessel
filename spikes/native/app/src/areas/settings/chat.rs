//! Chat, as Paseo's (`screens/settings/chat/chat-section.tsx`): the
//! "Detail level" section and its "Chat display" select (how much of an
//! agent's work a conversation shows).
//!
//! [Decision Log]
//! - Purpose: Paseo keeps how a chat reads in a section of its own; the
//!   display select moved here from Appearance, where the page had it.
//! - Choice: Paseo's section, row, label and hint; the kit's select.
//! - Not shown (nothing in the app reads them yet, the conversation
//!   track's to draw): "Always expand reasoning" and the "Chat outline"
//!   switch (the outline rail is always on).
//! - Difference: two of Paseo's three displays, Full and Collapse tool
//!   calls; "Collapse turns" (Paseo's default) waits on the conversation.
//!   The default stays Full until it exists.

use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, ParentElement as _};
use vessel_app_kit::{Tokens, settings};

use super::choices::List;
use super::general::select_for;
use crate::shell::Shell;

/// The Chat page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.settings();
    let detail = settings::card(tokens).child(settings::row_at(
        words.chat_display,
        Some(words.chat_display_note.into()),
        None,
        Some(select_for(List::Display, shell, compact, tokens, cx)),
        true,
        compact,
        tokens,
    ));
    settings::section(words.detail_level, detail, tokens).into_any_element()
}
