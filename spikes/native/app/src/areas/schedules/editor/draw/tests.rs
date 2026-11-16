//! Real ready form: Enter has no write shortcut; its actual footer still saves.
use super::*;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use tokio::sync::mpsc::error::TryRecvError;
use vessel_app_link::{Command, schedules::Request};
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    schedules::{
        editor::{definition::Save, session::Received},
        load::Failure,
    },
};
use vessel_proto::{
    NodeId,
    automation::{ScheduleInfo, TimeZoneName},
};

struct Probe {
    shell: kit::ui::Entity<Shell>,
    scheme: kit::Scheme,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        self.shell.update(cx, |shell, cx| {
            let ui = &shell.areas.schedules.ui.editor;
            let cache = shell
                .areas
                .schedules
                .data
                .editor_cache
                .as_ref()
                .ok()
                .and_then(Option::as_ref);
            let input = ui
                .inputs
                .as_ref()
                .map(|inputs| inputs.rule.element(&tokens, false, cx));
            div()
                .size_full()
                .flex()
                .flex_col()
                .children(input)
                .child(footer(shell, cache, ui.visit(), None, false, &tokens, cx))
        })
    }
}
fn prepare(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Option<Save> {
    let info: ScheduleInfo = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../view/tests/fixtures/schedules/info.json"
    )))
    .ok()?;
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .ok()?;
    let model = &mut shell.areas.schedules.data.editor;
    let opened = model.edit(info, context, TimeZoneName::utc()).ok()??;
    let at = "2026-11-15T00:00:00Z".parse().ok()?;
    let preview = model.preview(opened.visit, at).ok()??;
    assert!(matches!(
        model.received(&preview, Err(Failure::Older)),
        Received::Changed
    ));
    let candidate = model.prepare_save().ok()??;
    shell.route = crate::routes::Route::Schedules;
    shell.areas.schedules.ui.editor.mounted = true;
    shell.areas.schedules.ui.editor.inputs =
        super::super::Inputs::new(shell, opened.visit, window, cx);
    let _changed = shell.schedules_changed(cx);
    Some(candidate)
}
#[gpui::test]
fn ready_form_enter_never_writes_but_the_actual_footer_sends_one_original_update(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    for scheme in [kit::Scheme::Light, kit::Scheme::Dark] {
        let mut prepared = None;
        let mut commands = None;
        let (view, cx) = cx.add_window_view(|window, cx| {
            let shell = cx.new(|cx| {
                let (mut shell, receiver) = language_tests::with_commands(window, cx);
                prepared = prepare(&mut shell, window, cx);
                commands = Some(receiver);
                shell
            });
            Probe { shell, scheme }
        });
        assert!(prepared.is_some());
        let Some(Save::Update(expected)) = prepared else {
            return;
        };
        let Some(mut commands) = commands else {
            return;
        };
        cx.run_until_parked();
        view.update_in(cx, |probe, window, cx| {
            probe.shell.update(cx, |shell, cx| {
                assert!(matches!(
                    shell.areas.schedules.data.editor.prepare_save(),
                    Ok(Some(_))
                ));
                let inputs = shell.areas.schedules.ui.editor.inputs.as_ref();
                assert!(inputs.is_some());
                if let Some(inputs) = inputs {
                    inputs.rule.focus(window, cx);
                }
            });
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
        view.read_with(cx, |probe, cx| {
            let model = &probe.shell.read(cx).areas.schedules.data.editor;
            assert!(!model.pending_save());
            assert!(matches!(model.prepare_save(), Ok(Some(_))));
        });
        let bounds = cx.debug_bounds("schedule-form-save");
        assert!(bounds.is_some());
        let Some(bounds) = bounds else {
            return;
        };
        cx.simulate_click(bounds.center(), Modifiers::default());
        cx.run_until_parked();
        let command = commands.try_recv();
        assert!(matches!(command, Ok(Command::Schedules(Request::Form(_)))));
        let Ok(Command::Schedules(Request::Form(request))) = command else {
            return;
        };
        assert!(
            matches!(request.operation(), io::Operation::Save(candidate) if **candidate == Save::Update(expected))
        );
        view.read_with(cx, |probe, cx| {
            assert_eq!(
                probe
                    .shell
                    .read(cx)
                    .areas
                    .schedules
                    .data
                    .editor
                    .connection(),
                Some(request.connection())
            );
        });
        // The same rendered footer cannot submit a second request while pending.
        cx.simulate_click(bounds.center(), Modifiers::default());
        cx.run_until_parked();
        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
        view.read_with(cx, |probe, cx| {
            assert!(
                probe
                    .shell
                    .read(cx)
                    .areas
                    .schedules
                    .data
                    .editor
                    .pending_save()
            );
        });
    }
}
