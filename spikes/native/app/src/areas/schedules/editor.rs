//! Form visit, input identities and exact RPC handlers for one Schedule sheet.
mod draw;
mod inputs;
mod pick;
mod preferences;

use crate::{routes::Route, shell::Shell};
pub(super) use draw::{new_button, render};
use inputs::Inputs;
use pick::Field;
use std::time::Duration;
use vessel_app_kit::{
    ComboboxState, DialogState,
    select::Anchor,
    ui::{Context, SharedString, Window},
};
use vessel_app_link::{Command, Event, Rejected, schedules::form::Answer};
use vessel_app_view::{
    connection::ConnectionContext,
    schedules::{
        Action,
        editor::{
            Draft,
            io::{Ask, Form, Request},
        },
        load::{Failure, Tickets},
        page::Intent,
    },
};
use vessel_proto::{
    GetHarnessOptions,
    automation::{PreviewCount, PreviewSchedule},
};

const PREVIEW_DELAY: Duration = Duration::from_millis(240);

pub(super) struct Editor {
    model: Option<Form>,
    inputs: Option<Inputs>,
    dialog: DialogState,
    picker: ComboboxState,
    anchors: [Anchor; Field::COUNT],
    current_picker: Option<Field>,
    browsing: Option<GetHarnessOptions>,
    preview_task: Option<gpui::Task<()>>,
    preview_labels: Vec<SharedString>,
    tickets: Tickets,
    pub(super) sync_inputs: bool,
    pub(super) dismiss: bool,
    retired: bool,
    archive_turn: u64,
    restore_prefs: bool,
}

impl Editor {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            model: None,
            inputs: None,
            dialog: DialogState::new(cx),
            picker: ComboboxState::new(window, cx),
            anchors: std::array::from_fn(|_| Anchor::default()),
            current_picker: None,
            browsing: None,
            preview_task: None,
            preview_labels: Vec::new(),
            tickets: Tickets::default(),
            sync_inputs: false,
            dismiss: false,
            retired: false,
            archive_turn: 0,
            restore_prefs: false,
        }
    }

    fn available(&self) -> bool {
        !self.retired && self.model.as_ref().is_some_and(|form| !form.saving())
    }

    pub(super) fn close(&mut self, window: &mut Window, cx: &mut Context<'_, Shell>) {
        self.picker.close_and_restore(window, cx);
        self.current_picker = None;
        self.browsing = None;
        self.preview_task = None;
        self.preview_labels.clear();
        self.model = None;
        self.inputs = None;
        self.dialog.dismissed(window, cx);
        self.dismiss = false;
        self.sync_inputs = false;
        self.retired = false;
        self.restore_prefs = false;
    }

    pub(super) fn retired(&mut self, tag: &vessel_app_view::connection::ConnectionTag) -> bool {
        let found = self
            .model
            .as_ref()
            .is_some_and(|form| &form.connection().tag() == tag);
        if found {
            self.retired = true;
            self.preview_task = None;
        }
        found
    }
}

pub(super) fn editable(shell: &Shell, intent: &Intent) -> bool {
    let page = &shell.areas.schedules.data.page;
    intent.allows(page, Action::Delete)
        && page
            .row(intent.key())
            .is_some_and(|row| Draft::can_open(row.schedule))
}

pub(crate) fn refuse(request: Request, why: Rejected) -> Event {
    let failure = match why {
        Rejected::Full => Failure::Busy,
        Rejected::Closed => Failure::Offline,
    };
    Event::ScheduleForm(Answer::refused(request, failure))
}

impl Shell {
    fn schedule_form_open(
        &mut self,
        connection: ConnectionContext,
        draft: Draft,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let editor = &mut self.areas.schedules.editor;
        editor.close(window, cx);
        let inputs = Inputs::new(self.words, &draft, window, cx);
        editor.model = Some(Form::new(connection, draft));
        editor.inputs = Some(inputs);
        editor.dialog.focus(window, cx);
        self.schedule_form_issue(Ask::Targets);
        let options = self
            .areas
            .schedules
            .editor
            .model
            .as_ref()
            .and_then(|form| form.draft.spec())
            .map(|spec| GetHarnessOptions {
                harness: spec.harness.clone(),
                profile: spec.profile.clone(),
            });
        if let Some(options) = options {
            self.schedule_form_issue(Ask::Options(options));
        }
        cx.notify();
    }

    pub(super) fn schedule_create_open(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.route != Route::Schedules || self.areas.schedules.editor.model.is_some() {
            return;
        }
        let online = self
            .fleet
            .machines
            .iter()
            .filter_map(vessel_app_view::Machine::connection_context);
        let selected = self
            .areas
            .schedules
            .data
            .page
            .selected_machine()
            .map(|key| key.as_str())
            .or(self.prefs.machine.as_deref());
        let connection = online
            .clone()
            .find(|context| selected == Some(context.machine().as_str()))
            .or_else(|| online.into_iter().next());
        if let Some(connection) = connection {
            self.schedule_form_open(connection, Draft::create(None), window, cx);
        }
    }

    pub(super) fn schedule_edit_open(
        &mut self,
        intent: &Intent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.route != Route::Schedules
            || !editable(self, intent)
            || self.areas.schedules.editor.model.is_some()
        {
            return;
        }
        let Some(row) = self.areas.schedules.data.page.row(intent.key()) else {
            return;
        };
        let Some(draft) = Draft::open(row.schedule, None) else {
            return;
        };
        let Some(connection) = row.connection.cloned() else {
            return;
        };
        self.areas.schedules.ui.selected = None;
        self.areas.schedules.ui.menu.close_and_restore(window, cx);
        self.schedule_form_open(connection, draft, window, cx);
    }

    fn schedule_form_issue(&mut self, ask: Ask) {
        if self.route != Route::Schedules {
            return;
        }
        let editor = &mut self.areas.schedules.editor;
        if !editor.available() {
            return;
        }
        if let Some(request) = editor
            .model
            .as_mut()
            .and_then(|form| form.issue(&mut editor.tickets, ask))
        {
            self.command(Command::ScheduleForm(request));
        }
    }

    fn schedule_preview_after_edit(&mut self, cx: &mut Context<'_, Self>) {
        let editor = &mut self.areas.schedules.editor;
        let Some(form) = editor.model.as_mut() else {
            return;
        };
        form.invalidate_preview();
        editor.preview_task = Some(cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(PREVIEW_DELAY).await;
            let _closed = shell.update(cx, |shell, cx| shell.schedule_preview(cx));
        }));
    }

    fn schedule_preview(&mut self, cx: &mut Context<'_, Self>) {
        let editor = &mut self.areas.schedules.editor;
        let _done = editor.preview_task.take();
        let Some(cadence) = editor
            .model
            .as_ref()
            .and_then(|form| form.draft.cadence.checked().ok())
            .cloned()
        else {
            return;
        };
        self.schedule_form_issue(Ask::Preview(PreviewSchedule {
            cadence,
            after: Some(chrono::Utc::now()),
            count: PreviewCount::default(),
        }));
        cx.notify();
    }

    fn schedule_form_save(&mut self, cx: &mut Context<'_, Self>) {
        let editor = &self.areas.schedules.editor;
        if !editor.available() {
            return;
        }
        if let Some(write) = editor.model.as_ref().and_then(Form::save_request) {
            self.schedule_form_issue(Ask::Save(Box::new(write)));
            if let Some(inputs) = &self.areas.schedules.editor.inputs {
                inputs.hold(true, cx);
            }
            cx.notify();
        }
    }

    pub(crate) fn schedule_form_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let (request, result) = answer.settle();
        let write = matches!(request.ask, Ask::Save(_));
        let refresh = write
            && (result.is_ok()
                || matches!(
                    result,
                    Err(Failure::Unconfirmed)
                        | Err(Failure::Node(vessel_proto::ErrorCode::Conflict))
                ));
        let editor = &mut self.areas.schedules.editor;
        if let Some(form) = &mut editor.model
            && form.accept(&request, result)
        {
            if matches!(request.ask, Ask::Targets) {
                editor.sync_inputs = true;
                self.schedule_preview_after_edit(cx);
            } else if let Ask::Options(options) = &request.ask {
                if std::mem::take(&mut editor.restore_prefs) {
                    preferences::restore(form, options, &self.prefs);
                }
            } else if matches!(request.ask, Ask::Preview(_)) {
                editor.preview_labels = preview_labels(form, self.words);
            } else {
                if let Some(inputs) = &editor.inputs {
                    inputs.hold(
                        editor.retired || matches!(form.saved(), Some(Err(Failure::Unconfirmed))),
                        cx,
                    );
                }
                if matches!(form.saved(), Some(Ok(_))) {
                    editor.dismiss = true;
                    preferences::keep(self);
                }
            }
            cx.notify();
        }
        if refresh {
            self.areas.schedules.data.page.refresh();
            self.schedules_due();
        }
    }

    pub(super) fn schedule_form_frame(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let editor = &mut self.areas.schedules.editor;
        if editor.dismiss {
            editor.close(window, cx);
            cx.notify();
            return;
        }
        if std::mem::take(&mut editor.sync_inputs)
            && let Some(form) = &editor.model
            && let Some(inputs) = &editor.inputs
        {
            inputs.sync(&form.draft, window, cx);
        }
    }
}

fn preview_labels(
    form: &Form,
    catalog: &vessel_app_view::localization::Translator,
) -> Vec<SharedString> {
    use vessel_app_view::localization::Date;
    if form.draft.opened().is_some_and(|opened| {
        matches!(
            opened.state,
            vessel_proto::automation::ScheduleState::Ended { .. }
        )
    }) {
        return Vec::new();
    }
    let Some(Ok(preview)) = form.preview() else {
        return Vec::new();
    };
    let device = form
        .targets()
        .and_then(|facts| facts.as_ref().ok())
        .and_then(|facts| facts.device_zone.as_ref());
    preview
        .runs
        .iter()
        .filter_map(|at| {
            let (local, zone) = device.map_or_else(
                || (at.naive_utc(), "UTC".to_owned()),
                |zone| {
                    (
                        at.with_timezone(&chrono::Local).naive_local(),
                        zone.to_string(),
                    )
                },
            );
            catalog
                .schedules()
                .form_next_at(&Date::DateTime {
                    date: local.date(),
                    time: local.time(),
                    zone,
                })
                .ok()
                .map(Into::into)
        })
        .collect()
}
