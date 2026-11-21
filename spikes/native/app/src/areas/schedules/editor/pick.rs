//! One picker owns all select fields. The model picker drills into a provider
//! without changing the stored model until a person chooses a value.
mod choices;
use choices::{Change, Choice};
use super::{Editor, Shell};
use std::rc::Rc;
use vessel_app_kit::{
    self as kit, ComboHandlers, ComboHeader, Tokens,
    ui::{AnyElement, Context, InteractiveElement as _, IntoElement as _, SharedString,
        StatefulInteractiveElement as _, Styled as _, Window},
};
use vessel_app_view::schedules::editor::{Draft, io::Ask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
    Host,
    Project,
    Model,
    Thinking,
    Mode,
    Cadence,
    Unit,
}
impl Field {
    pub(super) const COUNT: usize = 7;

    fn id(self) -> &'static str {
        match self {
            Self::Host => "schedule-form-host",
            Self::Project => "schedule-form-project",
            Self::Model => "schedule-form-model",
            Self::Thinking => "schedule-form-thinking",
            Self::Mode => "schedule-form-mode",
            Self::Cadence => "schedule-form-cadence",
            Self::Unit => "schedule-form-unit",
        }
    }

    pub(super) fn label(self, shell: &Shell) -> &'static str {
        let words = shell.words.schedules();
        match self {
            Self::Host => words.form_host(),
            Self::Project => words.form_project(),
            Self::Model => words.form_model(),
            Self::Thinking => words.form_thinking(),
            Self::Mode => words.form_mode(),
            Self::Cadence => words.form_cadence(),
            Self::Unit => words.form_unit(),
        }
    }

    fn search(self, shell: &Shell) -> Option<&'static str> {
        let words = shell.words.schedules();
        match self {
            Self::Project => Some(words.form_search_projects()),
            Self::Model => Some(words.form_search_models()),
            _ => None,
        }
    }
}

pub(super) fn control(
    shell: &Shell, field: Field, value: impl Into<SharedString>, enabled: bool, tokens: &Tokens,
    touch: bool, cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let editor = &shell.areas.schedules.editor;
    let anchor = &editor.anchors[field as usize];
    let open = editor.current_picker == Some(field) && editor.picker.is_open();
    let enabled = enabled && editor.available();
    let trigger = kit::select::trigger(field.id(), value, anchor, open, enabled, touch, tokens)
        .w_full()
        .justify_between()
        .role(gpui::Role::Button)
        .aria_label(field.label(shell))
        .aria_expanded(open);
    if !enabled {
        return trigger.into_any_element();
    }
    trigger.on_click(cx.listener(move |shell, _, window, cx| {
        let search = field.search(shell);
        let editor = &mut shell.areas.schedules.editor;
        if !editor.available() {
            return;
        }
        editor.current_picker = Some(field);
        editor.browsing = None;
        editor.picker.set_anchor(editor.anchors[field as usize].get());
        editor.picker.open(search, window, cx);
        cx.notify();
    })).into_any_element()
}

pub(super) fn popup(shell: &Shell, tokens: &Tokens, window: &Window, cx: &mut Context<'_, Shell>) -> Option<AnyElement> {
    let editor = &shell.areas.schedules.editor;
    let field = editor.current_picker?;
    let form = editor.model.as_ref()?;
    if !editor.picker.is_open() || !editor.available() {
        return None;
    }
    let query = editor.picker.query(cx).to_lowercase();
    let choices = choices::items(shell, form, field)
        .into_iter()
        .filter(|choice| query.is_empty() || choice.row.label.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    let rows = choices.iter().map(|choice| choice.row.clone()).collect::<Vec<_>>();
    let back = editor.browsing.is_some();
    let title = editor.browsing.as_ref().map_or_else(
        || field.label(shell).into(),
        |provider| vessel_app_view::board::agent_name(&provider.harness).into(),
    );
    let header = ComboHeader {
        title: Some(title),
        back,
        search: field.search(shell).is_some(),
        empty: choices::empty(shell, field).into(),
        note: None,
        side: kit::picker::Side::Below,
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.schedules.editor.picker,
        chose: Rc::new(move |shell, index, window, cx| {
            if let Some(Choice { row, change }) = choices.get(index)
                && !row.disabled && shell.areas.schedules.editor.current_picker == Some(field)
            {
                apply(shell, change.clone(), window, cx);
            }
        }),
        back: back.then(|| Rc::new(go_back) as kit::picker::Back<Shell>),
    };
    kit::combobox(
        "schedule-form-picker", &editor.picker, &header, &rows, &handlers,
        kit::select::WIDTH, tokens, window, cx,
    )
}

fn go_back(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let search = Field::Model.search(shell);
    let editor = &mut shell.areas.schedules.editor;
    editor.browsing = None;
    editor.picker.open(search, window, cx);
    cx.notify();
}

fn apply(shell: &mut Shell, change: Change, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let editor = &mut shell.areas.schedules.editor;
    if !editor.available() {
        return;
    }
    let Some(form) = editor.model.as_mut() else { return };
    match change {
        Change::OpenProvider(provider) => {
            editor.restore_prefs = false;
            editor.browsing = Some(provider.clone());
            editor.picker.open(None, window, cx);
            shell.schedule_form_issue(Ask::Options(provider));
        }
        Change::Host(connection) => {
            if form.draft.opened().is_none() && form.connection() != &connection
                && let Some(form) = editor.model.take()
            {
                let mut draft = form.draft;
                draft.for_other_host();
                shell.schedule_form_open(connection, draft, window, cx);
            } else {
                close_picker(editor, window, cx);
            }
        }
        change => {
            let project = matches!(change, Change::Project(_));
            let cadence = apply_value(&mut form.draft, change);
            close_picker(editor, window, cx);
            if cadence {
                editor.sync_inputs = true;
                shell.schedule_preview_after_edit(cx);
            } else if project {
                shell.schedule_initial_provider();
            }
        }
    }
    cx.notify();
}

fn close_picker(editor: &mut Editor, window: &mut Window, cx: &mut Context<'_, Shell>) {
    editor.current_picker = None;
    editor.browsing = None;
    editor.picker.close_and_restore(window, cx);
}

fn apply_value(draft: &mut Draft, change: Change) -> bool {
    match change {
        Change::Host(_) | Change::OpenProvider(_) | Change::None => {}
        Change::Project(place) => draft.set_place(place),
        Change::SelectModel { provider, model } => draft.select_model(provider.harness, model),
        Change::Option(choice) => draft.set_option(choice),
        Change::Cadence(rule) => {
            draft.cadence.select(rule);
            return true;
        }
        Change::Unit(unit) => {
            draft.cadence.set_unit(unit);
            return true;
        }
    }
    false
}

pub(super) use choices::{control_for, model_label, project_label, unit_label};
