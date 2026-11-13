//! General, as the page's (`features/settings/general.tsx`) in Paseo's
//! layout (`screens/settings/general/sending-section.tsx`): the composer's
//! send shortcut and default send, each a select in a settings row, the
//! default send's row saying what the value in effect does; then where
//! the preferences are kept.
//!
//! [Decision Log]
//! - Purpose: the page's General, every row of it.
//! - Choice: Paseo's `SettingsSelect` for both (the kit's select); the
//!   page's words. Where they are kept is a row whose words say why when
//!   they last this launch only (the page's banner, `kept-note.tsx`, folded
//!   into the row: Paseo has no banner on a settings page).
//! - Differences: no lead sentence over the sections (Paseo's pages have
//!   none).

use vessel_app_kit::select;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings};
use vessel_app_view::prefs::{Kept, SendKey};

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
    let send_hint = match composer.send {
        SendKey::Enter => words.enter_sends,
        SendKey::ModEnter => words.shortcut_sends,
    };
    let sending = settings::card(tokens)
        .child(settings::row_at(
            words.send_shortcut,
            Some(send_hint.into()),
            None,
            Some(select_for(List::Send, shell, compact, tokens, cx)),
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.default_send,
            Some(words.delivery_note(composer.delivery).into()),
            None,
            Some(select_for(List::Delivery, shell, compact, tokens, cx)),
            false,
            compact,
            tokens,
        ));
    let kept = &shell.prefs.kept;
    let (hint, value) = match kept {
        Kept::File { path } => (words.kept_note.to_owned(), path.clone()),
        Kept::ThisLaunch { .. } => (
            words.kept_why(kept).unwrap_or_default(),
            words.this_launch.to_owned(),
        ),
    };
    let mono = matches!(kept, Kept::File { .. });
    let kept_card = settings::card(tokens).child(settings::row_at(
        words.preferences,
        Some(hint.into()),
        None,
        Some(
            div()
                .min_w_0()
                .max_w(px(280.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(if mono { Type::Mono } else { Type::Body })
                .text_color(tokens.neutral.foreground_muted)
                .child(value)
                .into_any_element(),
        ),
        true,
        compact,
        tokens,
    ));
    div()
        .flex()
        .flex_col()
        .child(settings::section(words.composer, sending, tokens))
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
