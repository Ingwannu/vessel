//! Actual engine identity is paired with the current pure form visit. New host,
//! reopen and explicit rule reset create fresh engines rather than reuse ids.
use super::{Context, Shell, Visit, Window, kit};
use kit::field::{Editability, FieldId, FormMinimum, TextField};
use vessel_app_view::schedules::editor::session::FieldChange;
#[derive(Debug, Clone, Copy)]
enum Input {
    Name,
    Prompt,
    Limit,
    Rule,
}
#[derive(Debug)]
pub(super) struct Inputs {
    pub(super) visit: Visit,
    pub(super) name: TextField,
    pub(super) prompt: TextField,
    pub(super) limit: TextField,
    pub(super) rule: TextField,
    blocked: bool,
}
impl Inputs {
    pub(super) fn new(
        shell: &Shell,
        visit: Visit,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Option<Self> {
        let model = &shell.areas.schedules.data.editor;
        if model.visit() != Some(visit) {
            return None;
        }
        let draft = model.draft()?;
        let words = shell.words.schedules();
        let name = TextField::with_source(words.form_optional(), submit, change, window, cx);
        let prompt = TextField::form_multiline_with_source(
            words.form_prompt_placeholder(),
            submit,
            change,
            FormMinimum::Expanded,
            window,
            cx,
        );
        let limit = TextField::with_source(words.form_unlimited(), submit, change, window, cx);
        if let Some(fields) = draft.fields() {
            name.set_text(fields.name(), window, cx);
            prompt.set_text(fields.prompt(), window, cx);
            limit.set_text(fields.limit(), window, cx);
        }
        let rule = rule(shell, window, cx)?;
        let mut inputs = Self {
            visit,
            name,
            prompt,
            limit,
            rule,
            blocked: false,
        };
        inputs.hold(model.blocked(), cx);
        Some(inputs)
    }
    fn source(&self, id: FieldId) -> Option<Input> {
        if self.name.id() == id {
            Some(Input::Name)
        } else if self.prompt.id() == id {
            Some(Input::Prompt)
        } else if self.limit.id() == id {
            Some(Input::Limit)
        } else if self.rule.id() == id {
            Some(Input::Rule)
        } else {
            None
        }
    }
    pub(super) fn hold(&mut self, blocked: bool, cx: &mut Context<'_, Shell>) {
        if self.blocked == blocked {
            return;
        }
        self.blocked = blocked;
        let editability = if blocked {
            Editability::ReadOnly
        } else {
            Editability::Editable
        };
        for field in [&self.name, &self.prompt, &self.limit, &self.rule] {
            field.set_editability(editability, cx);
        }
    }
}
fn rule(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Option<TextField> {
    let draft = shell.areas.schedules.data.editor.draft()?;
    let field = TextField::with_source(
        if draft.cadence.choice() == vessel_app_view::schedules::editor::cadence::Choice::Custom {
            shell.words.schedules().form_cron_placeholder()
        } else {
            ""
        },
        submit,
        change,
        window,
        cx,
    );
    field.set_text(draft.cadence.text(), window, cx);
    Some(field)
}
pub(super) fn reset_rule(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if let Some(field) = rule(shell, window, cx)
        && let Some(inputs) = &mut shell.areas.schedules.ui.editor.inputs
    {
        inputs.rule = field;
        inputs.rule.set_editability(
            if inputs.blocked {
                Editability::ReadOnly
            } else {
                Editability::Editable
            },
            cx,
        );
    }
}
fn change(shell: &mut Shell, source: FieldId, text: String, cx: &mut Context<'_, Shell>) {
    let Some(inputs) = &shell.areas.schedules.ui.editor.inputs else {
        return;
    };
    let visit = inputs.visit;
    let Some(input) = inputs.source(source) else {
        return;
    };
    let model = &mut shell.areas.schedules.data.editor;
    if model.visit() != Some(visit) || model.blocked() {
        return;
    }
    let changed = match input {
        Input::Name => model.field(visit, FieldChange::Name(text)),
        Input::Prompt => model.field(visit, FieldChange::Prompt(text)),
        Input::Limit => model.field(visit, FieldChange::Limit(text)),
        Input::Rule => {
            let unit = model.draft().and_then(|draft| draft.cadence.unit());
            model.cadence(visit, move |draft| match unit {
                Some(unit) => draft.interval(text, unit),
                None => draft.cron(text),
            })
        }
    };
    if changed {
        if matches!(input, Input::Rule) {
            super::queue_preview(shell, visit, cx);
        }
        let _changed = shell.schedules_changed(cx);
        cx.notify();
    }
}
fn submit(
    shell: &mut Shell,
    source: FieldId,
    _text: String,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let Some(inputs) = &shell.areas.schedules.ui.editor.inputs else {
        return;
    };
    if inputs.source(source).is_none() {
        return;
    }
    let visit = inputs.visit;
    super::save(shell, visit, window, cx);
}
