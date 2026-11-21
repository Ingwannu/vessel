use super::Shell;
use vessel_app_kit::{
    FormMinimum, FormText, FormTextareaPresentation, TextField,
    field::FieldId,
    ui::{Context, Window},
};
use vessel_app_view::{localization::Translator, schedules::editor::Draft};

pub(super) struct Inputs {
    pub(super) name: TextField,
    pub(super) prompt: TextField,
    pub(super) limit: TextField,
    pub(super) cron: TextField,
    pub(super) zone: TextField,
    pub(super) elapsed: TextField,
}

impl Inputs {
    pub(super) fn new(
        words: &Translator,
        draft: &Draft,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        let words = words.schedules();
        let name = line(words.form_optional(), words.form_name(), window, cx);
        let prompt = TextField::form_multiline_with_source_presentation(
            words.form_prompt_placeholder(),
            submitted,
            changed,
            FormTextareaPresentation::Md,
            FormMinimum::Expanded,
            window,
            cx,
        );
        prompt.set_accessible_label(words.form_prompt(), cx);
        let cron = TextField::with_source_text(
            words.form_cron_placeholder(),
            submitted,
            changed,
            FormText::Code,
            window,
            cx,
        );
        cron.set_accessible_label(words.form_script(), cx);
        let inputs = Self {
            name,
            prompt,
            limit: line(words.form_unlimited(), words.form_limit(), window, cx),
            cron,
            zone: line(words.form_zone_placeholder(), words.form_zone(), window, cx),
            elapsed: line("", words.form_count(), window, cx),
        };
        inputs.name.set_text(draft.fields.name(), window, cx);
        inputs.prompt.set_text(draft.fields.prompt(), window, cx);
        inputs.limit.set_text(draft.fields.limit(), window, cx);
        inputs.sync(draft, window, cx);
        inputs
    }

    pub(super) fn sync(&self, draft: &Draft, window: &mut Window, cx: &mut Context<'_, Shell>) {
        for (field, value) in [
            (&self.cron, draft.cadence.expression()),
            (&self.zone, draft.cadence.zone()),
            (&self.elapsed, draft.cadence.count()),
        ] {
            if field.text(cx) != value {
                field.set_text(value, window, cx);
            }
        }
    }

    fn set(&self, draft: &mut Draft, id: FieldId, text: String) -> Option<bool> {
        if id == self.name.id() {
            draft.fields.set_name(text);
        } else if id == self.prompt.id() {
            draft.fields.set_prompt(text);
        } else if id == self.limit.id() {
            draft.fields.set_limit(text);
        } else if id == self.cron.id() {
            draft.cadence.set_expression(text);
            return Some(true);
        } else if id == self.zone.id() {
            draft.cadence.set_zone(text);
            return Some(true);
        } else if id == self.elapsed.id() {
            draft.cadence.set_count(text);
            return Some(true);
        } else {
            return None;
        }
        Some(false)
    }
}

fn line(
    placeholder: &str,
    label: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> TextField {
    let input = TextField::with_source(placeholder, submitted, changed, window, cx);
    input.set_accessible_label(label, cx);
    input
}

fn submitted(
    _shell: &mut Shell,
    _id: FieldId,
    _text: String,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) {
}

fn changed(shell: &mut Shell, id: FieldId, text: String, cx: &mut Context<'_, Shell>) {
    let editor = &mut shell.areas.schedules.editor;
    if !editor.available() {
        return;
    }
    if let Some(form) = &mut editor.model
        && let Some(inputs) = &editor.inputs
        && let Some(cadence) = inputs.set(&mut form.draft, id, text)
    {
        if cadence {
            shell.schedule_preview_after_edit(cx);
        }
        cx.notify();
    }
}
