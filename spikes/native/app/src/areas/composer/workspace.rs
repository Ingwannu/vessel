//! A scoped editor uses the same card/trays/hooks; the creation owner keeps Enter.
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, composer_card,
    ui::{Context, Div, ParentElement as _, Styled as _, Window},
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
    let touch = crate::areas::draft::compact(window);
    let input = super::workspace_text(shell, target, tokens, window, cx)?;
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
