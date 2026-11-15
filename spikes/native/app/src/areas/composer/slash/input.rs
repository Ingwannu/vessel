//! Existing composer input wrappers: menu/recall/attachment hook order stays shared.

use vessel_app_kit::Tokens;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, hooked};
use vessel_app_kit::ui::{AnyElement, App, Context, Window};
use vessel_app_view::composer::drafts::DraftKey;

use super::Over;
use crate::shell::Shell;

/// The session composer's text with its keys offered to the menu first
/// (`field::composer_hooks`); pasted images and files are attached
/// (`files.rs`).
pub(crate) fn text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    hooked_text(shell, Over::Session, tokens, touch, cx)
}

/// The new chat's composer text, its keys offered to its `/` menu first
/// (the draft's card draws it in place of the bare field).
pub(crate) fn draft_text(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    hooked_text(shell, Over::Draft, tokens, touch, cx)
}

fn hooked_text(
    shell: &Shell,
    over: Over,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let field = shell.slash_field(over);
    let element = field.element(tokens, touch, cx);
    let weak = cx.entity().downgrade();
    let pasting = weak.clone();
    let new_chat = over == Over::Draft;
    let target = match over {
        Over::Draft => Some(DraftKey::New),
        Over::Session => shell
            .areas
            .session
            .open
            .as_ref()
            .map(|open| DraftKey::Session {
                session: open.session,
            }),
    };
    hooked(
        field,
        element,
        move |key, window: &mut Window, cx: &mut App| {
            weak.update(cx, |shell, cx| {
                if let Some(target) = target
                    && shell.composer_file_key(target, key, window, cx) == KeyAnswer::Taken
                {
                    return KeyAnswer::Taken;
                }
                shell.slash_key(over, key, window, cx)
            })
            .unwrap_or(KeyAnswer::Pass)
        },
        // Pasted images and files are the composer's attachments.
        move |pasted, _window, cx: &mut App| {
            pasting
                .update(cx, |shell, cx| shell.composer_pasted(new_chat, pasted, cx))
                .unwrap_or(KeyAnswer::Pass)
        },
    )
}
