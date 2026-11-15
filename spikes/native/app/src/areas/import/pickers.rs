//! Current import scope stays visible while popup admission is unavailable.
//!
//! [Decision Log]
//! - Purpose: old popup input must never close/highlight a successor popup.
//! - Constraint: the shared engine currently mutates its state before the choice
//!   callback. An area callback cannot fence those effects.
//! - Choice: park popup first-use entirely until the shared owner supplies checked
//!   per-open admission. Keep the existing current-draft/default import boundary.
//! - Cost: this packet cannot change machine, harness or account in the sheet.
//!   A follow-up may restore the one shared engine after its admission seam lands.

use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::theme::spacing::S2;
use vessel_app_kit::ui::{Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Type, TypeExt as _};

/// Read-only labels deliberately register no popup, keyboard or outside handlers.
pub(super) fn controls(shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.words.history();
    let scope = shell.areas.import.model.scope();
    let machine = scope
        .map_or(words.choose_machine(), |scope| scope.machine.as_str())
        .to_owned();
    let agent = scope.map_or_else(
        || words.choose_agent().to_owned(),
        |scope| vessel_app_view::agent_name(&scope.harness),
    );
    let profile = scope
        .and_then(|scope| scope.profile.as_ref())
        .map_or_else(|| words.default_account().to_owned(), ToString::to_string);
    div().flex().flex_col().gap(px(S2)).children(
        [
            (words.machine(), machine),
            (words.agent(), agent),
            (words.account(), profile),
        ]
        .into_iter()
        .map(|(label, value)| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(S2))
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(label.to_owned()),
                )
                .child(
                    div()
                        .type_(Type::Body)
                        .text_color(tokens.neutral.foreground)
                        .child(value),
                )
        }),
    )
}
