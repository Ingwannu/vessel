use super::{Shell, choose, field, hint, pick::Field};
use vessel_app_kit::{
    Tokens,
    theme::spacing::S3,
    ui::{Context, Div, ParentElement as _, Styled as _, div, px},
};
use vessel_app_view::schedules::{
    editor::{cadence::description, io::Form},
    load::Failure,
    locale,
};

pub(super) fn body(
    shell: &Shell,
    form: &Form,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.schedules();
    let editor = &shell.areas.schedules.editor;
    let draft = &form.draft.cadence;
    let display = if draft.is_elapsed() {
        words.form_select_cadence()
    } else {
        draft.choice().label(shell.words)
    };
    let mut out = div().flex().flex_col().gap(px(S3)).child(choose(
        shell,
        Field::Cadence,
        display,
        true,
        tokens,
        touch,
        cx,
    ));
    if let Some(inputs) = &editor.inputs {
        out = out.child(inputs.cron.element(tokens, touch, cx));
        if draft.needs_zone_field() && form.targets().is_some() {
            out = out.child(field(
                words.form_zone(),
                inputs.zone.element(tokens, touch, cx),
                None,
                tokens,
            ));
        }
    }
    out.children(feedback(shell, form, tokens))
}

fn feedback(shell: &Shell, form: &Form, tokens: &Tokens) -> Option<Div> {
    let words = shell.words.schedules();
    let rule = match form.draft.cadence.checked() {
        Ok(rule) => rule,
        Err(issue) => return Some(hint(issue.label(shell.words), true, tokens)),
    };
    let preview = &shell.areas.schedules.editor.preview_label;
    let describe = || {
        description(rule, shell.words)
            .ok()
            .map(|label| hint(label, false, tokens))
    };
    match form.preview() {
        Some(Ok(_)) => preview
            .as_ref()
            .map(|label| hint(label.clone(), false, tokens))
            .or_else(describe),
        Some(Err(Failure::Older)) if form.draft.cadence.is_elapsed() => {
            let summary = description(rule, shell.words).ok()?;
            let text = words.form_elapsed_without_preview(&summary).ok()?;
            Some(hint(text, false, tokens))
        }
        Some(Err(Failure::Older)) => Some(hint(words.preview_unavailable_older(), false, tokens)),
        Some(Err(Failure::Unexpected)) => Some(hint(words.preview_mismatched(), true, tokens)),
        Some(Err(error)) => Some(hint(locale::failure(error, shell.words), true, tokens)),
        None => describe(),
    }
}
