//! Compose the sheet from the existing dialog, text, switch, and picker kit.
mod cadence;
use super::{Shell, pick::{self, Field}};
use vessel_app_kit::{
    self as kit, ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, Tokens,
    Type, TypeExt as _,
    dialog::{CompactLayout, DesktopHeight, DialogLayout},
    sheet::snap::Percentage,
    theme::spacing::{S2, S4},
    ui::{AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
        SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px},
};
use vessel_app_view::schedules::{
    editor::{fields::Field as TextField, io::{Ask, Form}},
    load::Failure,
    locale,
};

const WIDTH: u16 = 560;
const DESKTOP_CAP: u8 = 85;
const COMPACT_CAP: u8 = 90;

pub(in crate::areas::schedules) fn new_button(shell: &Shell, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> AnyElement {
    let available = shell.fleet.machines.iter().any(|machine| machine.connection_context().is_some());
    let words = shell.words.schedules();
    let button = button("schedule-new", words.form_new_title(), ButtonKind::Primary, available, touch, tokens);
    if available {
        button.on_click(cx.listener(|shell, _, window, cx| shell.schedule_create_open(window, cx))).into_any_element()
    } else {
        button.aria_label(words.form_no_machine()).into_any_element()
    }
}

pub(in crate::areas::schedules) fn render(shell: &Shell, tokens: &Tokens, window: &Window, touch: bool, cx: &mut Context<'_, Shell>) -> Option<AnyElement> {
    let editor = &shell.areas.schedules.editor;
    let form = editor.model.as_ref()?;
    let edit = form.draft.opened().is_some();
    let words = shell.words.schedules();
    let body = div()
        .flex().flex_col().gap(px(S4))
        .children(text_fields(shell, form, tokens, touch, cx))
        .children(targets(shell, form, tokens, touch, cx))
        .child(archive(shell, form, tokens, touch, cx))
        .child(cadence::body(shell, form, tokens, touch, cx))
        .children(limit(shell, form, tokens, touch, cx))
        .children(status(shell, form, tokens, touch, cx))
        .children(edit.then(|| hint(words.form_ownership(), false, tokens)));
    let sheet = kit::dialog(
        &editor.dialog,
        DialogContent {
            title: if edit { words.form_edit_title() } else { words.form_new_title() }.into(),
            body: body.into_any_element(),
            footer: footer(shell, form, tokens, touch, cx),
            size: size(),
        },
        tokens, window, touch,
        |shell: &mut Shell, window, cx| {
            let editor = &mut shell.areas.schedules.editor;
            if editor.available() || editor.retired && !editor.model.as_ref().is_some_and(Form::saving) {
                editor.close(window, cx);
                cx.notify();
            }
        },
        cx,
    );
    Some(div().child(sheet).children(pick::popup(shell, tokens, window, cx)).into_any_element())
}

fn size() -> DialogSize {
    Percentage::new(DESKTOP_CAP)
        .zip(Percentage::new(COMPACT_CAP))
        .and_then(|(desktop, compact)| DialogLayout::new(
            WIDTH, DesktopHeight::AtMost(desktop), CompactLayout::Fixed(compact),
        ))
        .map_or(DialogSize::Normal, DialogSize::Configured)
}

fn footer(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Vec<AnyElement> {
    let words = shell.words.schedules();
    let idle = !form.saving();
    let cancel = button("schedule-form-cancel", words.cancel(), ButtonKind::Secondary, idle, touch, tokens);
    let cancel = if idle {
        cancel.on_click(cx.listener(|shell, _, window, cx| {
            if !shell.areas.schedules.editor.model.as_ref().is_some_and(Form::saving) {
                shell.areas.schedules.editor.close(window, cx);
                cx.notify();
            }
        }))
    } else { cancel };
    let text = if form.saving() { words.form_saving() } else if form.draft.opened().is_some() {
        words.form_save()
    } else { words.form_create() };
    let ready = shell.areas.schedules.editor.available() && form.save_request().is_some();
    let save = button("schedule-form-save", text, ButtonKind::Primary, ready, touch, tokens);
    let save = if ready {
        save.on_click(cx.listener(|shell, _, _, cx| shell.schedule_form_save(cx)))
    } else { save };
    vec![cancel.into_any_element(), save.into_any_element()]
}

pub(super) fn field(label: &'static str, input: AnyElement, error: Option<String>, tokens: &Tokens) -> Div {
    div().flex().flex_col().gap(px(S2))
        .child(div().type_(Type::Body).text_color(tokens.neutral.foreground_muted).child(label))
        .child(input)
        .children(error.map(|error| hint(error, true, tokens)))
}

fn text_fields(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Vec<Div> {
    let Some(inputs) = &shell.areas.schedules.editor.inputs else { return Vec::new() };
    let words = shell.words.schedules();
    let issue = |field| form.draft.fields.feedback(field).map(|error| error.label(shell.words));
    vec![
        field(words.form_name(), inputs.name.element(tokens, touch, cx), issue(TextField::Name), tokens),
        field(words.form_prompt(), inputs.prompt.element(tokens, touch, cx), issue(TextField::Prompt), tokens),
    ]
}

fn targets(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Vec<Div> {
    let mut rows = Vec::new();
    let ready = matches!(form.targets(), Some(Ok(_)));
    if shell.areas.schedules.data.page.multiple_machines() {
        rows.push(choose(shell, Field::Host, form.connection().machine().as_str().to_owned(),
            form.draft.opened().is_none(), tokens, touch, cx));
    }
    rows.push(choose(shell, Field::Project, pick::project_label(shell, form), ready, tokens, touch, cx));
    rows.push(choose(shell, Field::Model, pick::model_label(shell, &form.draft), ready, tokens, touch, cx));
    if form.draft.spec().is_some() {
        for field in [Field::Thinking, Field::Mode] {
            let control = pick::control_for(shell, form, field);
            let value = control.as_ref().map_or_else(
                || shell.words.schedules().form_default_choice().to_owned(),
                |control| control.current_name().to_owned(),
            );
            rows.push(choose(shell, field, value, control.is_some(), tokens, touch, cx));
        }
    }
    rows
}

pub(super) fn choose(shell: &Shell, name: Field, value: impl Into<SharedString>, enabled: bool, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Div {
    field(name.label(shell), pick::control(shell, name, value, enabled, tokens, touch, cx), None, tokens)
}

fn archive(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Div {
    let editor = &shell.areas.schedules.editor;
    let text = shell.words.schedules().form_archive();
    let on = form.draft.archive();
    let toggle = kit::settings::switch("schedule-form-archive", on, editor.available(), touch, editor.archive_turn, tokens)
        .role(gpui::Role::Switch).aria_label(text).aria_toggled(
            if on { gpui::Toggled::True } else { gpui::Toggled::False },
        );
    let toggle = if editor.available() {
        toggle.on_click(cx.listener(|shell, _, _, cx| {
            let editor = &mut shell.areas.schedules.editor;
            if editor.available() && let Some(form) = &mut editor.model {
                form.draft.set_archive(!form.draft.archive());
                editor.archive_turn = editor.archive_turn.saturating_add(1);
                cx.notify();
            }
        }))
    } else { toggle };
    div().flex().items_center().justify_between()
        .child(div().type_(Type::Body).text_color(tokens.neutral.foreground_muted).child(text))
        .child(toggle)
}

fn limit(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Option<Div> {
    let inputs = shell.areas.schedules.editor.inputs.as_ref()?;
    let issue = form.draft.fields.feedback(TextField::Limit).map(|error| error.label(shell.words));
    Some(field(shell.words.schedules().form_limit(), inputs.limit.element(tokens, touch, cx), issue, tokens))
}

fn status(shell: &Shell, form: &Form, tokens: &Tokens, touch: bool, cx: &mut Context<'_, Shell>) -> Option<AnyElement> {
    let editor = &shell.areas.schedules.editor;
    let words = shell.words.schedules();
    if editor.retired {
        return Some(hint(locale::failure(&Failure::ConnectionChanged, shell.words), true, tokens).into_any_element());
    }
    if let Some(Err(error)) = form.saved() {
        return Some(hint(locale::failure(error, shell.words), true, tokens).into_any_element());
    }
    match form.targets()? {
        Err(error) => Some(div().child(hint(locale::failure(error, shell.words), true, tokens)).child(
            button("schedule-form-retry-targets", words.retry(), ButtonKind::Ghost, true, touch, tokens)
                .on_click(cx.listener(|shell, _, _, cx| {
                    shell.schedule_form_issue(Ask::Targets);
                    cx.notify();
                })),
        ).into_any_element()),
        Ok(_) if !form.can_operate() => Some(hint(words.read_only(), true, tokens).into_any_element()),
        Ok(_) => None,
    }
}

pub(super) fn hint(text: impl Into<SharedString>, error: bool, tokens: &Tokens) -> Div {
    div().type_(Type::Meta)
        .text_color(if error { tokens.neutral.destructive } else { tokens.neutral.foreground_muted })
        .child(text)
}

pub(super) fn button(id: &'static str, label: &'static str, kind: ButtonKind, enabled: bool, touch: bool, tokens: &Tokens) -> kit::ui::Stateful<Div> {
    kit::button_with_kind_at(id, label, kind, if enabled { ButtonState::Enabled } else { ButtonState::Disabled },
        ControlSizing::from_compact(touch), tokens)
}
