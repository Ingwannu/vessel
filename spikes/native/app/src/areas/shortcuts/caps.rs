//! The keys bound to an action, as Paseo's `useShortcutKeys` hands them to
//! its `Shortcut` badges: the first binding's chords, one caption per chord,
//! as the person has set the keys. The command centre's rows, the pane
//! header's tooltips and its menus all read them here.
use crate::shell::Shell;
use vessel_app_kit::ui::SharedString;
use vessel_app_view::shortcuts::{Action, Platform, bindings_of, chord_caps};

/// The first key bound to `action`, one caption per chord; none while the
/// action has no key.
pub(crate) fn caps(action: Action, shell: &Shell) -> Vec<SharedString> {
    let platform = Platform::native();
    let joiner = match platform {
        Platform::Mac => "",
        Platform::Other => "+",
    };
    bindings_of(action, &shell.prefs.settings.shortcuts)
        .first()
        .map(|binding| {
            binding
                .chords()
                .map(|chord| chord_caps(chord, platform).join(joiner).into())
                .collect()
        })
        .unwrap_or_default()
}
