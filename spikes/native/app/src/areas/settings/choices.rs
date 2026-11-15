//! What each of Settings' lists holds: its rows (the value in effect
//! checked), the words on its trigger, and the change a row makes.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::{ComboRow, Leading, Tokens};
use vessel_app_view::prefs::{
    CodeSize, ContentWidth, ConversationDisplay, InterfaceSize, PreferenceChange, SendKey,
    TurnDelivery, send_key_label,
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
    /// The interface's text size.
    Interface,
    /// Code's text size.
    Code,
    /// The conversation's width.
    Width,
    /// How a conversation shows tool calls.
    Display,
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
        List::Machine => chosen_machine(shell)
            .map_or_else(|| words.machine.to_owned(), |machine| machine.name.clone()),
        List::Send => send_key_label(shell.prefs.settings.composer.send, primary()),
        List::Delivery => words
            .delivery(shell.prefs.settings.composer.delivery)
            .to_owned(),
        List::Interface => words.points(appearance.interface_size.get()),
        List::Code => words.points(appearance.code_size.get()),
        List::Width => words.width(appearance.content_width).to_owned(),
        List::Display => words.display(appearance.conversation).to_owned(),
    }
}

/// `list`'s rows.
pub(crate) fn rows(list: List, shell: &Shell, tokens: &Tokens) -> Vec<ComboRow> {
    let words = shell.t.lang.settings();
    let settings = &shell.prefs.settings;
    let appearance = &settings.appearance;
    let row = |label: String, checked: bool| ComboRow {
        checked,
        ..ComboRow::new(label)
    };
    match list {
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
            .map(|delivery| {
                row(
                    words.delivery(delivery).to_owned(),
                    delivery == settings.composer.delivery,
                )
            })
            .collect(),
        List::Interface => InterfaceSize::all()
            .into_iter()
            .map(|size| row(words.points(size.get()), size == appearance.interface_size))
            .collect(),
        List::Code => CodeSize::all()
            .into_iter()
            .map(|size| row(words.points(size.get()), size == appearance.code_size))
            .collect(),
        List::Width => ContentWidth::ALL
            .into_iter()
            .map(|width| {
                row(
                    words.width(width).to_owned(),
                    width == appearance.content_width,
                )
            })
            .collect(),
        List::Display => ConversationDisplay::ALL
            .into_iter()
            .map(|display| {
                row(
                    words.display(display).to_owned(),
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
        List::Send => SendKey::ALL.get(index).copied().map(PreferenceChange::Send),
        List::Delivery => DELIVERIES
            .get(index)
            .copied()
            .map(PreferenceChange::Delivery),
        List::Interface => InterfaceSize::all()
            .get(index)
            .copied()
            .map(PreferenceChange::InterfaceSize),
        List::Code => CodeSize::all()
            .get(index)
            .copied()
            .map(PreferenceChange::CodeSize),
        List::Width => ContentWidth::ALL
            .get(index)
            .copied()
            .map(PreferenceChange::ContentWidth),
        List::Display => ConversationDisplay::ALL
            .get(index)
            .copied()
            .map(PreferenceChange::Conversation),
    }
}
