//! Compose only the fields a new-session definition owns.
use super::{Shell, archive, cadence, hint, limit, status, targets, text_fields};
use vessel_app_kit::{
    Tokens,
    theme::spacing::S4,
    ui::{Context, Div, ParentElement as _, Styled as _, div, px},
};
use vessel_app_view::schedules::editor::io::Form;

pub(super) fn body(
    shell: &Shell,
    form: &Form,
    edit: bool,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .children(text_fields(shell, form, tokens, touch, cx))
        .children(targets(shell, form, tokens, touch, cx))
        .child(archive(shell, form, tokens, touch, cx))
        .child(cadence::body(shell, form, tokens, touch, cx))
        .children(limit(shell, form, tokens, touch, cx))
        .children(status(shell, form, tokens, touch, cx))
        .children(edit.then(|| hint(shell.words.schedules().form_ownership(), false, tokens)))
}
