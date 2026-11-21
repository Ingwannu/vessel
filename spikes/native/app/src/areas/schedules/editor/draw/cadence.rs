use super::{Shell, button, choose, field, hint, pick::{self, Field}};
use vessel_app_kit::{
    ButtonKind, Tokens, theme::spacing::{S2, S4},
    ui::{Context, Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px},
};
use vessel_app_view::schedules::{
    editor::{cadence::Choice, io::Form},
    load::Failure,
    locale,
};

pub(super) fn body(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.schedules();
    let editor = &shell.areas.schedules.editor;
    let draft = &form.draft.cadence;
    let choice = draft.choice();
    let error = draft.checked().err().map(|error| error.label(shell.words));
    let mut out = div().flex().flex_col().gap(px(S4)).child(choose(
        shell, Field::Cadence, choice.label(shell.words), true, tokens, touch, cx,
    ));
    if let Some(inputs) = &editor.inputs {
        if choice == Choice::Interval {
            out = out.child(div().flex().gap(px(S2))
                .child(div().flex_1().child(field(words.form_count(), inputs.elapsed.element(tokens, touch, cx), None, tokens)))
                .child(div().flex_1().child(choose(shell, Field::Unit, pick::unit_label(shell, draft.unit()), true, tokens, touch, cx))));
        } else {
            if choice == Choice::Custom {
                out = out.child(field(words.form_script(), inputs.cron.element(tokens, touch, cx), None, tokens));
            }
            out = out.child(field(words.form_zone(), inputs.zone.element(tokens, touch, cx), None, tokens));
            if let Some(Ok(facts)) = form.targets()
                && let Some(zone) = &facts.machine_zone
                && zone.as_str() != draft.zone()
                && let Ok(note) = words.form_host_zone(zone.as_str())
            {
                out = out.child(hint(note, false, tokens));
            }
        }
    }
    if let Some(error) = error {
        out.child(hint(error, true, tokens))
    } else {
        out.child(preview(shell, form, tokens, touch, cx))
    }
}

fn preview(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.schedules();
    let editor = &shell.areas.schedules.editor;
    let body = div().flex().flex_col().gap(px(S2));
    match form.preview() {
        Some(Ok(_)) => body.child(hint(words.form_next_runs(), false, tokens)).children(
            editor.preview_labels.iter().cloned().map(|text| hint(text, false, tokens)),
        ),
        Some(Err(Failure::Older)) => body.child(hint(words.preview_unavailable_older(), false, tokens)),
        Some(Err(error)) => {
            let text = if *error == Failure::Unexpected { words.preview_mismatched() } else { locale::failure(error, shell.words) };
            body.child(hint(text, true, tokens)).child(
                button("schedule-form-retry-preview", words.retry(), ButtonKind::Ghost, true, touch, tokens)
                    .on_click(cx.listener(|shell, _, _, cx| shell.schedule_preview(cx)))
                    .into_any_element(),
            )
        }
        None if form.previewing() => body.child(hint(words.form_previewing(), false, tokens)),
        None => body,
    }
}
