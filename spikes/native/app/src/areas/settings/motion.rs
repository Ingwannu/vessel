//! Settings › Appearance › Animations (the owner's word 39: "a setting
//! turns animation off, and then every change is instant").
//!
//! [Decision Log]
//! - Purpose: the person chooses whether the app moves; off gives every
//!   change its end state at once, as reduced motion and a software
//!   renderer do.
//! - Constraint: every transition reads one flag (GPUI's
//!   `App::reduce_motion`, kept by `crate::motion` from all its reasons),
//!   so the setting is one more reason there, not a check in each surface.
//! - Choice: Paseo's settings row with its switch (`kit::settings`), in a
//!   section of its own under the theme; the choice is kept with the
//!   preferences and applied at once (`PreferenceChange::Animations`) and
//!   at start (`main.rs`).
//! - Reference: Paseo has no such setting (it follows the system); the
//!   owner asked for one. The words are the app's own.

use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
};
use vessel_app_kit::{Tokens, settings};
use vessel_app_view::prefs::{Animations, PreferenceChange};

use crate::shell::Shell;

/// The Motion section: its one row.
pub(super) fn section(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.motion();
    let on = shell.prefs.settings.appearance.animations.on();
    let switch = settings::switch(
        "settings-animations",
        on,
        true,
        compact,
        shell.areas.settings.animations_turns,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        let next = if on { Animations::Off } else { Animations::On };
        shell.areas.settings.animations_turns =
            shell.areas.settings.animations_turns.saturating_add(1);
        shell.prefer(&PreferenceChange::Animations(next), window, cx);
    }));
    let card = settings::card(tokens).child(settings::row_at(
        words.animations,
        Some(words.note.into()),
        None,
        Some(switch.into_any_element()),
        true,
        compact,
        tokens,
    ));
    settings::section(words.section, card, tokens).into_any_element()
}
