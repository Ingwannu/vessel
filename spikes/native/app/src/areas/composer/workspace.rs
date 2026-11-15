//! A scoped editor uses the same card/trays/hooks; the creation owner keeps Enter.
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, composer_card,
    field::composer_hooks::{KeyAnswer, hooked},
    ui::{App, Context, Div, ParentElement as _, Styled as _, Window},
};
use vessel_app_view::composer::drafts::DraftKey;

/// W3 passes an admitted exact draft key and its own creation editability.
pub(crate) fn workspace_card(
    shell: &Shell,
    target: DraftKey,
    editable: bool,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    if !matches!(target, DraftKey::Workspace { .. }) {
        return None;
    }
    let field = shell.composer_field(target)?;
    let touch = crate::areas::draft::compact(window);
    let weak = cx.entity().downgrade();
    let paste = weak.clone();
    let input = hooked(
        field,
        field.element(tokens, touch, cx),
        move |key, window, cx: &mut App| {
            weak.update(cx, |shell, cx| {
                if editable && shell.composer_destination() == Some(target) {
                    shell.composer_file_key(target, key, window, cx)
                } else {
                    KeyAnswer::Pass
                }
            })
            .unwrap_or(KeyAnswer::Pass)
        },
        move |pasted, _window, cx: &mut App| {
            paste
                .update(cx, |shell, cx| {
                    if editable && shell.composer_destination() == Some(target) {
                        shell.composer_pasted_at(target, pasted, cx)
                    } else {
                        KeyAnswer::Pass
                    }
                })
                .unwrap_or(KeyAnswer::Pass)
        },
    );
    let menu = editable
        .then(|| super::mentions::menu(shell, target, tokens, window, cx))
        .flatten();
    let attach = (editable && shell.areas.composer.attach_for == Some(target))
        .then(|| super::files::attach_menu(shell, tokens, touch, cx))
        .flatten();
    let card = composer_card(tokens, touch)
        .relative()
        .children(menu)
        .children(attach)
        .children(super::files::tray(shell, target, tokens, touch, cx))
        .children(super::context::tray(shell, target, tokens, touch, cx))
        .children(super::local::notice(shell, target, tokens))
        .child(input);
    let card = if editable {
        super::files::dropped_on(card, Some(target), tokens, cx)
            .child(super::files::attach_button_at(target, touch, tokens, cx))
    } else {
        card
    };
    Some(card)
}
