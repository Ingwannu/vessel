//! The reference schedule sheet on the existing form/input/select/dialog controls.
//! Domain state and validation remain in View; only an explicit press sends work.
mod body;
mod choices;
mod draw;
mod inputs;
mod lifecycle;
mod preferences;
use crate::shell::Shell;
pub(super) use choices::popup;
pub(super) use draw::render;
use inputs::Inputs;
pub(super) use lifecycle::{arrive, frame, hold, leaving, needs_frame, ready_choices};
use std::{
    rc::{Rc, Weak},
    time::Duration,
};
use vessel_app_kit::{
    self as kit, ComboboxState, Tokens,
    dialog::DialogState,
    select::Anchor,
    ui::{AnyElement, Context, Window},
};
use vessel_app_link::Command;
use vessel_app_view::{
    connection::ConnectionContext,
    schedules::{
        Action,
        editor::{cadence, io, session::Visit},
        page::Intent,
    },
};
use vessel_proto::automation::{ScheduleInfo, TimeZoneName};
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(300);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Picker {
    Host,
    Project,
    Model,
    Thinking,
    Mode,
    Cadence,
    Unit,
    Zone,
}
impl Picker {
    fn id(self) -> &'static str {
        match self {
            Self::Host => "schedule-form-host",
            Self::Project => "schedule-form-project",
            Self::Model => "schedule-form-model",
            Self::Thinking => "schedule-form-thinking",
            Self::Mode => "schedule-form-mode",
            Self::Cadence => "schedule-form-cadence",
            Self::Unit => "schedule-form-unit",
            Self::Zone => "schedule-form-zone",
        }
    }
}
#[derive(Debug, Default)]
struct Anchors {
    host: Anchor,
    project: Anchor,
    model: Anchor,
    thinking: Anchor,
    mode: Anchor,
    cadence: Anchor,
    unit: Anchor,
    zone: Anchor,
}
impl Anchors {
    fn get(&self, picker: Picker) -> &Anchor {
        match picker {
            Picker::Host => &self.host,
            Picker::Project => &self.project,
            Picker::Model => &self.model,
            Picker::Thinking => &self.thinking,
            Picker::Mode => &self.mode,
            Picker::Cadence => &self.cadence,
            Picker::Unit => &self.unit,
            Picker::Zone => &self.zone,
        }
    }
}
#[derive(Debug)]
enum Opening {
    Create,
    Edit {
        intent: Intent,
        info: Box<ScheduleInfo>,
        connection: ConnectionContext,
    },
}
#[derive(Debug)]
enum OpeningError {
    Cadence(vessel_app_view::schedules::editor::CadenceError),
    Definition(vessel_app_view::schedules::editor::EditError),
    Changed,
    PreviewCount(vessel_proto::automation::OutOfRange),
}
#[derive(Debug)]
struct Bootstrap {
    cycle: Rc<()>,
    opening: Opening,
}
#[derive(Debug)]
struct Projection {
    cycle: Rc<()>,
    task: kit::ui::Task<()>,
}
#[derive(Debug)]
pub(super) struct Ui {
    inputs: Option<Inputs>,
    bootstrap: Option<Bootstrap>,
    dialog: DialogState,
    picker: ComboboxState,
    picking: Option<Picker>,
    browsing: bool,
    anchors: Anchors,
    projection: Option<Projection>,
    mounted: bool,
    lang: Option<vessel_app_view::Lang>,
    error: Option<OpeningError>,
    error_words: Option<vessel_app_view::schedules::presentation::Notice>,
    zones: Result<Vec<TimeZoneName>, vessel_app_view::schedules::editor::zones::ZoneChoicesError>,
}
impl Ui {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            inputs: None,
            bootstrap: None,
            dialog: DialogState::new(cx),
            picker: ComboboxState::new(window, cx),
            picking: None,
            browsing: false,
            anchors: Anchors::default(),
            projection: None,
            mounted: false,
            lang: None,
            error: None,
            error_words: None,
            zones: vessel_app_view::schedules::editor::zones::bundled(),
        }
    }
    pub(super) fn opening_error(
        &self,
    ) -> Option<&vessel_app_view::schedules::presentation::Notice> {
        self.error_words.as_ref()
    }
    pub(super) fn is_mounted(&self) -> bool {
        self.mounted
    }
    fn visit(&self) -> Option<Visit> {
        self.inputs.as_ref().map(|inputs| inputs.visit)
    }
    fn cancel_preview(&mut self) {
        if let Some(projection) = self.projection.take() {
            drop(projection.task);
        }
    }
}
pub(super) fn open_new(
    shell: &mut Shell,
    expected: Option<Visit>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if shell.route != crate::routes::Route::Schedules
        || !shell.areas.schedules.data.can_create
        || shell.areas.schedules.data.editor.visit() != expected
        || shell.areas.schedules.ui.editor.mounted
        || shell.areas.schedules.data.editor.pending_save()
    {
        return;
    }
    begin(shell, Opening::Create, window, cx);
}
pub(super) fn open_edit(
    shell: &mut Shell,
    intent: &Intent,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let area = &shell.areas.schedules;
    if shell.route != crate::routes::Route::Schedules
        || !intent.allows(&area.data.page, Action::Edit)
        || area.data.editor.pending_save()
    {
        return;
    }
    let Some(row) = area.data.page.row(intent.key()) else {
        return;
    };
    let Some(connection) = row.connection.cloned() else {
        return;
    };
    let opening = Opening::Edit {
        intent: intent.clone(),
        info: Box::new(row.schedule.clone()),
        connection,
    };
    begin(shell, opening, window, cx);
}
fn begin(shell: &mut Shell, opening: Opening, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if shell.areas.schedules.ui.deleting.is_some() || shell.areas.schedules.ui.editor.mounted {
        return;
    }
    shell.areas.schedules.ui.editor.error = None;
    if let Some(visit) = shell.areas.schedules.ui.history.take() {
        let _closed = shell.areas.schedules.data.history.close(visit);
        shell
            .areas
            .schedules
            .ui
            .history_dialog
            .dismissed(window, cx);
    }
    let zone = shell
        .areas
        .schedules
        .data
        .clock
        .as_ref()
        .ok()
        .and_then(|clock| clock.name())
        .cloned();
    shell.areas.schedules.ui.menu.close_and_restore(window, cx);
    shell.areas.schedules.ui.selected = None;
    shell
        .areas
        .schedules
        .ui
        .picker
        .close_and_restore(window, cx);
    if let Some(zone) = zone {
        let _installed = install(shell, opening, zone, window, cx);
    } else {
        if let Some(visit) = shell.areas.schedules.data.editor.visit()
            && !shell.areas.schedules.data.editor.close(visit)
        {
            return;
        }
        let ui = &mut shell.areas.schedules.ui.editor;
        ui.inputs = None;
        ui.cancel_preview();
        if ui.mounted {
            ui.dialog.dismissed(window, cx);
        }
        ui.bootstrap = Some(Bootstrap {
            cycle: Rc::new(()),
            opening,
        });
        ui.dialog.focus(window, cx);
        ui.mounted = true;
    }
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
fn install(
    shell: &mut Shell,
    opening: Opening,
    zone: TimeZoneName,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> bool {
    let result = match opening {
        Opening::Create => {
            let cadence = match cadence::initial(zone.clone()) {
                Ok(cadence) => cadence,
                Err(error) => {
                    shell.areas.schedules.ui.editor.error = Some(OpeningError::Cadence(error));
                    return false;
                }
            };
            shell
                .areas
                .schedules
                .data
                .editor
                .create(cadence, zone)
                .map(|visit| (visit, None))
        }
        Opening::Edit {
            intent,
            info,
            connection,
        } => {
            if !intent.is_current(&shell.areas.schedules.data.page) {
                shell.areas.schedules.ui.editor.error = Some(OpeningError::Changed);
                return false;
            }
            shell
                .areas
                .schedules
                .data
                .editor
                .edit(*info, connection, zone)
                .map(|opened| match opened {
                    Some(opened) => (Some(opened.visit), Some(opened.request)),
                    None => (None, None),
                })
        }
    };
    let (visit, request) = match result {
        Ok((Some(visit), request)) => (visit, request),
        Ok((None, _)) => return false,
        Err(error) => {
            shell.areas.schedules.ui.editor.error = Some(OpeningError::Definition(error));
            return false;
        }
    };
    if let Some(request) = request {
        shell.command(Command::Schedules(request.into()));
    }
    mount(shell, visit, window, cx);
    if shell.areas.schedules.data.editor.connection().is_none() {
        let available: Vec<_> = shell
            .fleet
            .machines
            .iter()
            .filter_map(vessel_app_view::Machine::connection_context)
            .collect();
        if let [context] = available.as_slice() {
            choose_host(shell, visit, context.clone(), window, cx);
        }
    } else {
        report_authority(shell, visit);
        hold(shell, cx);
        queue_preview(shell, visit, cx);
    }
    true
}
fn mount(shell: &mut Shell, visit: Visit, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let inputs = Inputs::new(shell, visit, window, cx);
    let ui = &mut shell.areas.schedules.ui.editor;
    if ui.mounted {
        ui.dialog.dismissed(window, cx);
    }
    ui.picker.close_and_restore(window, cx);
    ui.inputs = inputs;
    ui.bootstrap = None;
    ui.picking = None;
    ui.browsing = false;
    ui.cancel_preview();
    ui.dialog.focus(window, cx);
    ui.mounted = true;
    ui.lang = Some(shell.t.lang);
}
fn report_authority(shell: &mut Shell, visit: Visit) {
    let model = &shell.areas.schedules.data.editor;
    let Some(connection) = model.connection() else {
        return;
    };
    let schedule = model
        .draft()
        .and_then(|draft| draft.opened())
        .map(|info| info.id);
    let authority = shell
        .areas
        .schedules
        .data
        .page
        .authority_for(connection, schedule);
    if let Some(authority) = authority {
        let _changed = shell
            .areas
            .schedules
            .data
            .editor
            .reported_authority(visit, &authority);
    }
}
fn choose_host(
    shell: &mut Shell,
    visit: Visit,
    context: ConnectionContext,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !shell
        .fleet
        .machines
        .iter()
        .any(|machine| machine.connection_context().as_ref() == Some(&context))
    {
        return;
    }
    if let Some(opened) = shell
        .areas
        .schedules
        .data
        .editor
        .select_host(visit, context)
    {
        shell.command(Command::Schedules(opened.request.into()));
        mount(shell, opened.visit, window, cx);
        report_authority(shell, opened.visit);
        hold(shell, cx);
        queue_preview(shell, opened.visit, cx);
    }
}
fn queue_preview(shell: &mut Shell, visit: Visit, cx: &mut Context<'_, Shell>) {
    let cycle = Rc::new(());
    let weak = Rc::downgrade(&cycle);
    let task = cx.spawn(async move |this, cx| {
        cx.background_executor().timer(PREVIEW_DEBOUNCE).await;
        let _closed = this.update(cx, |shell, cx| {
            if !shell
                .areas
                .schedules
                .ui
                .editor
                .projection
                .as_ref()
                .is_some_and(|pending| Weak::ptr_eq(&Rc::downgrade(&pending.cycle), &weak))
            {
                return;
            }
            match shell
                .areas
                .schedules
                .data
                .editor
                .preview(visit, chrono::Utc::now())
            {
                Ok(Some(request)) => shell.command(Command::Schedules(request.into())),
                Ok(None) => {}
                Err(error) => {
                    shell.areas.schedules.ui.editor.error = Some(OpeningError::PreviewCount(error));
                }
            }
            let _changed = shell.schedules_changed(cx);
            cx.notify();
        });
    });
    shell.areas.schedules.ui.editor.cancel_preview();
    shell.areas.schedules.ui.editor.projection = Some(Projection { cycle, task });
}
fn save(shell: &mut Shell, visit: Visit, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    if shell.route != crate::routes::Route::Schedules
        || !shell.areas.schedules.ui.editor.mounted
        || shell.areas.schedules.ui.editor.visit() != Some(visit)
    {
        return;
    }
    let Ok(Some(candidate)) = shell.areas.schedules.data.editor.prepare_save() else {
        return;
    };
    if let Some(request) = shell.areas.schedules.data.editor.save(visit, candidate) {
        hold(shell, cx);
        preferences::dispatch(shell, request, cx);
        let _changed = shell.schedules_changed(cx);
        cx.notify();
    }
}
fn close(
    shell: &mut Shell,
    visit: Option<Visit>,
    bootstrap: Option<Weak<()>>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let ui = &shell.areas.schedules.ui.editor;
    if shell.route != crate::routes::Route::Schedules || !ui.mounted || ui.visit() != visit {
        return;
    }
    if let Some(expected) = bootstrap {
        if !ui
            .bootstrap
            .as_ref()
            .is_some_and(|current| Weak::ptr_eq(&Rc::downgrade(&current.cycle), &expected))
        {
            return;
        }
    } else if let Some(visit) = visit {
        if !shell.areas.schedules.data.editor.close(visit) {
            return;
        }
    } else {
        return;
    }
    let ui = &mut shell.areas.schedules.ui.editor;
    ui.picker.close_and_restore(window, cx);
    ui.inputs = None;
    ui.bootstrap = None;
    ui.cancel_preview();
    ui.picking = None;
    ui.dialog.dismissed(window, cx);
    ui.mounted = false;
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
