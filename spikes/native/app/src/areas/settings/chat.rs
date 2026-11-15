//! Chat, as Paseo's (`screens/settings/chat/chat-section.tsx`): the
//! "Detail level" section and its "Chat display" select (how much of an
//! agent's work a conversation shows).
//!
//! [Decision Log]
//! - Purpose: Paseo keeps how a chat reads in a section of its own; the
//!   display select moved here from Appearance, where the page had it.
//! - Choice: Paseo's section, row, label and hint; the kit's select.
//! - The outline switch shares the persisted typed preference read by the rail.
//!   "Always expand reasoning" remains with the conversation renderer.
//! - Difference: two of Paseo's three displays, Full and Collapse tool
//!   calls; "Collapse turns" (Paseo's default) waits on the conversation.
//!   The default stays Full until it exists.

use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
};
use vessel_app_kit::{Tokens, settings};
use vessel_app_view::prefs::{ChatOutline, PreferenceChange};

use super::choices::List;
use super::general::select_for;
use crate::shell::Shell;

/// The kit's numeric animation revision stays private to this switch.
#[derive(Default)]
pub(super) struct SwitchTurn(u64);

/// The Chat page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.chat();
    let detail = settings::card(tokens)
        .child(settings::row_at(
            words.display(),
            Some(words.description().into()),
            None,
            Some(select_for(List::Display, shell, compact, tokens, cx)),
            true,
            compact,
            tokens,
        ))
        .child(outline_row(shell, tokens, compact, cx));
    settings::section(words.detail_level(), detail, tokens).into_any_element()
}

/// One setting reuses the normal preference/store path; no area-local copy.
fn outline_row(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    let words = shell.words.chat_outline();
    let on = shell.prefs.settings.appearance.chat_outline.on();
    let turn = if cx.reduce_motion() {
        0
    } else {
        shell.areas.settings.outline_turn.0
    };
    let control = settings::switch("chat-outline", on, true, compact, turn, tokens).on_click(
        cx.listener(move |shell, _event, window, cx| {
            shell.areas.settings.outline_turn.0 =
                shell.areas.settings.outline_turn.0.saturating_add(1);
            let next = if on {
                ChatOutline::Off
            } else {
                ChatOutline::On
            };
            shell.prefer(&PreferenceChange::ChatOutline(next), window, cx);
        }),
    );
    settings::row_at(
        words.label(),
        Some(words.hint().into()),
        None,
        Some(control.into_any_element()),
        false,
        compact,
        tokens,
    )
}
