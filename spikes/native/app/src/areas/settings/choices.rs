//! What each of Settings' lists holds: its rows (the value in effect
//! checked), the words on its trigger, and the change a row makes.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::SharedString;
use vessel_app_kit::{ComboRow, Leading, Tokens};
use vessel_app_view::prefs::{
    ConversationDisplay, PreferenceChange, SendKey, TurnDelivery, send_key_label,
};
use vessel_app_view::{Machine, Reach};

use crate::shell::Shell;

/// One of Settings' lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum List {
    /// The machine the per-machine sections are about.
    Machine,
    /// Which key sends.
    Send,
    /// What Send does while a turn runs.
    Delivery,
    /// How a conversation shows tool calls.
    Display,
    /// Syntax colours, independent of app palette.
    Syntax,
}

/// The default send's choices in the order Paseo's list offers them
/// (`sending-section.tsx` `SEND_BEHAVIORS`), the rows and their changes
/// read from the same table.
const DELIVERIES: [TurnDelivery; 3] = [
    TurnDelivery::Interrupt,
    TurnDelivery::Steer,
    TurnDelivery::Queue,
];

/// The platform's main modifier, as the send key's words name it.
pub(crate) const fn primary() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// The machine the per-machine sections are about: the one chosen, else
/// the first in the book.
pub(crate) fn chosen_machine(shell: &Shell) -> Option<&Machine> {
    let machines = &shell.fleet.machines;
    shell
        .areas
        .settings
        .machine
        .as_ref()
        .and_then(|name| machines.iter().find(|machine| &machine.name == name))
        .or_else(|| machines.first())
}

/// A machine's dot: ready when it answered, the danger colour when not.
pub(crate) const fn reach_color(machine: &Machine, tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    match machine.reach {
        Reach::Online => tokens.st_ready,
        Reach::Unreachable(_) => tokens.danger,
    }
}

/// The words on `list`'s trigger: the value in effect.
pub(crate) fn label(list: List, shell: &Shell) -> String {
    let words = shell.t.lang.settings();
    let appearance = &shell.prefs.settings.appearance;
    match list {
        List::Syntax => {
            super::appearance::syntax_label(appearance.syntax_theme, shell.words).to_owned()
        }
        List::Machine => chosen_machine(shell)
            .map_or_else(|| words.machine.to_owned(), |machine| machine.name.clone()),
        List::Send => send_key_label(shell.prefs.settings.composer.send, primary()),
        List::Delivery => shell
            .words
            .general()
            .delivery(shell.prefs.settings.composer.delivery)
            .to_owned(),
        List::Display => shell
            .words
            .chat()
            .display_choice(appearance.conversation)
            .to_owned(),
    }
}

/// The General delivery trigger borrows its cached message, including pseudo
/// labels too long for GPUI's inline-string representation.
pub(super) fn delivery_label(shell: &Shell) -> SharedString {
    SharedString::new_static(
        shell
            .words
            .general()
            .delivery(shell.prefs.settings.composer.delivery),
    )
}

/// `list`'s rows.
pub(crate) fn rows(list: List, shell: &Shell, tokens: &Tokens) -> Vec<ComboRow> {
    let settings = &shell.prefs.settings;
    let appearance = &settings.appearance;
    let row = |label: String, checked: bool| ComboRow {
        checked,
        ..ComboRow::new(label)
    };
    match list {
        List::Syntax => vessel_app_view::appearance::SyntaxTheme::ALL
            .into_iter()
            .map(|theme| {
                row(
                    super::appearance::syntax_label(theme, shell.words).to_owned(),
                    theme == appearance.syntax_theme,
                )
            })
            .collect(),
        List::Machine => {
            let chosen = chosen_machine(shell).map(|machine| machine.name.as_str());
            let mut rows: Vec<ComboRow> = shell
                .fleet
                .machines
                .iter()
                .map(|machine| ComboRow {
                    leading: Some(Leading::Dot(reach_color(machine, tokens))),
                    checked: chosen == Some(machine.name.as_str()),
                    ..ComboRow::new(machine.name.clone())
                })
                .collect();
            rows.push(ComboRow {
                leading: Some(Leading::Glyph(Glyph::Plus)),
                ..ComboRow::new(shell.t.add_machine)
            });
            rows
        }
        List::Send => SendKey::ALL
            .into_iter()
            .map(|key| {
                row(
                    send_key_label(key, primary()),
                    key == settings.composer.send,
                )
            })
            .collect(),
        List::Delivery => DELIVERIES
            .into_iter()
            .map(|delivery| ComboRow {
                checked: delivery == settings.composer.delivery,
                ..ComboRow::new(SharedString::new_static(
                    shell.words.general().delivery(delivery),
                ))
            })
            .collect(),
        List::Display => ConversationDisplay::ALL
            .into_iter()
            .map(|display| {
                row(
                    shell.words.chat().display_choice(display).to_owned(),
                    display == appearance.conversation,
                )
            })
            .collect(),
    }
}

/// The change row `index` of `list` makes; `None` for the machine picker
/// (not a preference) or a row that is not there.
pub(crate) fn change(list: List, index: usize) -> Option<PreferenceChange> {
    match list {
        List::Machine => None,
        List::Syntax => vessel_app_view::appearance::SyntaxTheme::ALL
            .get(index)
            .copied()
            .map(PreferenceChange::SyntaxTheme),
        List::Send => SendKey::ALL.get(index).copied().map(PreferenceChange::Send),
        List::Delivery => DELIVERIES
            .get(index)
            .copied()
            .map(PreferenceChange::Delivery),
        List::Display => ConversationDisplay::ALL
            .get(index)
            .copied()
            .map(PreferenceChange::Conversation),
    }
}

#[cfg(test)]
#[path = "choices/language_tests.rs"]
mod language_tests;
