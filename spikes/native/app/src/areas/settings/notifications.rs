//! Settings › Notifications (ADR 0114): the page's section with the
//! app's words, and Paseo's desktop rows (`desktop-notifications-section.tsx`).
//!
//! [Decision Log]
//! - Purpose: choose how a session that needs the person, or finished, is
//!   told, and check that the system shows it.
//! - Existing assets: the preferences the page already keeps
//!   (`notifications.alerts`, `notifications.title`; the prefs contract),
//!   the kit's select and switch, and the choices list.
//! - Rows:
//!   - Sessions: the four ways (the page's select) and the title count.
//!   - This system: where notifications go on this system, and Test
//!     notification.
//! - Differences from Paseo:
//!   - Paseo has one switch, "Play sound", because its notification
//!     sounds itself. Here the four ways are the page's, so "Sound only"
//!     exists.
//!   - Paseo's permission row has a button. Linux and Windows have no
//!     permission to ask for, so the row says where notifications go.
//!     Where this build shows none (macOS, a browser), it says so.
//!   - Send is off while the alerts are off, as on the page. Paseo's is
//!     off until the permission is granted.
//! - The test's result is Paseo's inline Alert under the section (small;
//!   success, or error with why). With Sound only it says that the sound
//!   was asked for, since no notification is handed over.

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_view::prefs::PreferenceChange;

use super::choices::List;
use super::general::select_for;
use crate::areas::notify::{Tested, Unavailable};
use crate::shell::Shell;

/// The Notifications page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.notifications();
    let kept = shell.prefs.settings.notifications;
    let on = kept.title;
    let turns = shell.areas.notify.turns;
    let count = settings::switch("notifications-title", on, true, compact, turns, tokens)
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.areas.notify.turns = shell.areas.notify.turns.wrapping_add(1);
            shell.prefer(&PreferenceChange::TitleCount(!on), window, cx);
        }))
        .into_any_element();
    let sessions = settings::card(tokens)
        .child(settings::row_at(
            words.alerts(),
            Some(words.alerts_hint().into()),
            None,
            Some(select_for(
                List::Alerts,
                words.alerts(),
                shell,
                compact,
                tokens,
                cx,
            )),
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.title(),
            Some(words.title_hint().into()),
            None,
            Some(count),
            false,
            compact,
            tokens,
        ));
    let wanted = kept.alerts.system() || kept.alerts.sound();
    let send = outline_button("notifications-test", words.test_send(), wanted, tokens)
        .on_click(cx.listener(|shell, _event, _window, cx| shell.notify_test(cx)))
        .into_any_element();
    let system = settings::card(tokens)
        .child(settings::row_at(
            words.delivery(),
            Some(delivery(shell).into()),
            None,
            None,
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.test(),
            Some(words.test_hint().into()),
            None,
            Some(send),
            false,
            compact,
            tokens,
        ));
    div()
        .id("settings-notifications")
        .flex()
        .flex_col()
        .child(settings::section(words.sessions(), sessions, tokens))
        .child(settings::section(words.system(), system, tokens))
        .children(
            shell
                .areas
                .notify
                .tested
                .map(|tested| result(shell, tested, tokens)),
        )
        .into_any_element()
}

/// Paseo's inline Alert for the last test.
fn result(shell: &Shell, tested: Tested, tokens: &Tokens) -> AnyElement {
    let words = shell.words.notifications();
    let (variant, title, description) = match tested {
        Tested::Handed => (Variant::Success, words.test_sent(), words.test_handed()),
        Tested::Sounded => (Variant::Success, words.test_sent(), words.test_sounded()),
        Tested::Failed(Unavailable::Build) => {
            (Variant::Error, words.test_failed(), words.delivery_none())
        }
        Tested::Failed(Unavailable::NoBus) => {
            (Variant::Error, words.test_failed(), words.test_no_bus())
        }
    };
    alert(
        Content {
            title: Some(title.to_owned().into()),
            description: Some(div().child(description.to_owned()).into_any_element()),
            action: None,
        },
        variant,
        Size::Small,
        tokens,
    )
    .into_any_element()
}

/// Where this build's system notifications go.
fn delivery(shell: &Shell) -> String {
    let words = shell.words.notifications();
    let said = if cfg!(target_arch = "wasm32") {
        words.delivery_none()
    } else if cfg!(target_os = "linux") {
        words.delivery_linux()
    } else if cfg!(target_os = "windows") {
        words.delivery_windows()
    } else {
        words.delivery_none()
    };
    said.to_owned()
}

#[cfg(test)]
#[path = "notifications/language_tests.rs"]
mod language_tests;
