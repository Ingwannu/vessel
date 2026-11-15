//! About's "Connected machines": each machine in the book with its node's
//! version, as Paseo's `ConnectedHostsSection` and `HostVersionRow`
//! (`screens/settings-screen.tsx`).
//!
//! [Decision Log]
//! - Purpose: Paseo's About tells which daemon version each host runs,
//!   the first thing asked when a host misbehaves.
//! - Choice: Paseo's section, one row per machine in the book's order,
//!   its name at the left and its version at the right in About's muted
//!   value: "v" and the version when it answered, "Offline" when it did
//!   not, "—" when it answered without one; no section without machines.
//! - Difference: Paseo marks a version that differs from the app's
//!   ("Version differs from this device"). Vessel's app builds and its
//!   node are numbered apart (0.0.0.0.x and 0.1.x), so they are not
//!   compared and nothing is marked.

use vessel_app_kit::ui::{Div, IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings};
use vessel_app_view::{Machine, Reach};

use crate::shell::Shell;

/// The section, while the book has a machine.
pub(crate) fn section(shell: &Shell, tokens: &Tokens, compact: bool) -> Option<Div> {
    let machines = &shell.fleet.machines;
    if machines.is_empty() {
        return None;
    }
    let words = shell.t.lang.settings();
    let mut card = settings::card(tokens);
    for (index, machine) in machines.iter().enumerate() {
        let value = div()
            .flex_shrink_0()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(version_text(machine, words.offline));
        card = card.child(settings::row_at(
            machine.name.clone(),
            None,
            None,
            Some(value.into_any_element()),
            index == 0,
            compact,
            tokens,
        ));
    }
    Some(settings::section(words.connected_machines, card, tokens))
}

/// What the row's value says (Paseo's `formatVersionWithPrefix`).
fn version_text(machine: &Machine, offline: &str) -> String {
    match &machine.reach {
        Reach::Unreachable(_) => offline.to_owned(),
        Reach::Online => machine
            .version
            .as_ref()
            .and_then(vessel_app_view::NodeVersion::label)
            .unwrap_or_else(|| "—".to_owned()),
    }
}
