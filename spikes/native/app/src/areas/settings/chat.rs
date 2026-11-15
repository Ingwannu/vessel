//! Chat, as Paseo's (`screens/settings/chat/chat-section.tsx`): the
//! "Detail level" section and its "Chat display" select (how much of an
//! agent's work a conversation shows).
//!
//! [Decision Log]
//! - Purpose: Paseo keeps how a chat reads in a section of its own; the
//!   display select moved here from Appearance, where the page had it.
//! - Choice: Paseo's section, row, label and hint; the kit's select.
//! - Reasoning defaults are consumed by the shared timeline; all three display
//!   choices use the one typed picker. New preferences default to Collapse turns;
//!   an explicit previously saved Full choice remains Full. W1 appends outline
//!   after display through its existing preference seam.

use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
};
use vessel_app_kit::{Tokens, settings};
use vessel_app_view::prefs::{PreferenceChange, ReasoningExpansion};

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
    let words = shell.words.chat();
    let detail = settings::card(tokens)
        .child(reasoning(shell, tokens, compact, cx))
        .child(settings::row_at(
            words.display(),
            Some(words.description().into()),
            None,
            Some(select_for(List::Display, shell, compact, tokens, cx)),
            false,
            compact,
            tokens,
        ));
    settings::section(words.detail_level(), detail, tokens).into_any_element()
}

fn reasoning(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    let words = shell.words.chat_reasoning();
    let on = shell.prefs.settings.appearance.reasoning.expanded();
    let switch = settings::switch(
        "chat-reasoning",
        on,
        true,
        compact,
        shell.areas.settings.reasoning_turns,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        let next = if on {
            ReasoningExpansion::Collapsed
        } else {
            ReasoningExpansion::Expanded
        };
        shell.areas.settings.reasoning_turns =
            shell.areas.settings.reasoning_turns.saturating_add(1);
        shell.prefer(&PreferenceChange::Reasoning(next), window, cx);
    }));
    settings::row_at(
        words.label(),
        Some(words.hint().into()),
        None,
        Some(switch.into_any_element()),
        true,
        compact,
        tokens,
    )
}
