//! General, as Paseo's (`screens/settings-screen.tsx` `GeneralSection`
//! and `screens/settings/general/sending-section.tsx` `SendingSection`):
//! the "General" section (Language), the "Sending" section (the send
//! shortcut, and the default send with a hint for the value in effect);
//! then where the preferences are kept, which Paseo has not.
//!
//! [Decision Log]
//! - Purpose: Paseo's General, section for section, label for label.
//! - Choice: Paseo's two sections and `SettingsSelect` rows (the kit's
//!   select). The send shortcut has no hint, as Paseo's; the default
//!   send's hint is the conversation track's sentence for what the value
//!   does in Vessel (it falls back where Paseo's alternate key and steer
//!   differ). The default send's list is in Paseo's order (Interrupt,
//!   Steer, Queue; `choices.rs`).
//! - Not shown: Paseo's "Show setup again" (Vessel has no first-run
//!   walkthrough yet) and "Open location" (no side panes).
//! - Difference: "Kept" is Vessel's own: where the preferences are kept,
//!   and why they last this launch only when no file can hold them (the
//!   page's banner folded into a row; Paseo keeps its settings in the
//!   browser's storage and says nothing).

use vessel_app_kit::select;
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings};
use vessel_app_view::prefs::Kept;
use vessel_app_view::settings::Section;

use super::applied;
use super::choices::{self, List};
use crate::shell::Shell;

/// The General page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.settings();
    let composer = &shell.prefs.settings.composer;
    let general = settings::card(tokens).child(super::language::row(shell, tokens, compact, cx));
    // Each row once the composer reads its value (`super::applied`). The
    // send shortcut only where keyboard shortcuts are (Paseo's
    // `useKeyboardShortcutsAvailable`: not on a compact layout).
    let send_shown = applied(List::Send) && !compact;
    let sending = settings::card(tokens)
        .when(send_shown, |card| {
            card.child(settings::row_at(
                words.send_shortcut,
                None,
                None,
                Some(select_for(List::Send, shell, compact, tokens, cx)),
                true,
                compact,
                tokens,
            ))
        })
        .when(applied(List::Delivery), |card| {
            card.child(settings::row_at(
                words.default_send,
                Some(words.delivery_note(composer.delivery).into()),
                None,
                Some(select_for(List::Delivery, shell, compact, tokens, cx)),
                !send_shown,
                compact,
                tokens,
            ))
        });
    let kept = &shell.prefs.kept;
    // The file's path under the note, its copy at the right (as About's log
    // file); this launch only, said with why.
    let (hint, control) = match kept {
        Kept::File { path } => {
            let copied = path.clone();
            let copy = outline_button(
                "settings-copy-prefs-path",
                shell.words.update().copy_path(),
                true,
                tokens,
            )
            .on_click(cx.listener(move |shell, _event, _window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()));
                shell.notice = Some(shell.t.lang.settings().path_copied.to_owned());
                cx.notify();
            }));
            (
                format!("{}\n{path}", words.kept_note),
                copy.into_any_element(),
            )
        }
        Kept::ThisLaunch { .. } => (
            words.kept_why(kept).unwrap_or_default(),
            div()
                .flex_shrink_0()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.this_launch)
                .into_any_element(),
        ),
    };
    let kept_card = settings::card(tokens).child(settings::row_at(
        words.preferences,
        Some(hint.into()),
        None,
        Some(control),
        true,
        compact,
        tokens,
    ));
    let shown = send_shown || applied(List::Delivery);
    div()
        .flex()
        .flex_col()
        .child(settings::section(
            words.section(Section::General),
            general,
            tokens,
        ))
        .when(shown, |page| {
            page.child(settings::section(words.sending, sending, tokens))
        })
        .child(settings::section(words.kept, kept_card, tokens))
        .into_any_element()
}

/// The select for `list`, opening its list on a click.
pub(crate) fn select_for(
    list: List,
    shell: &Shell,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = &shell.areas.settings;
    select::trigger(
        gpui::ElementId::Name(format!("settings-select-{list:?}").into()),
        choices::label(list, shell),
        state.anchors.of(list),
        state.showing() == Some(list),
        true,
        compact,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        shell.open_list(list, window, cx);
    }))
    .into_any_element()
}
