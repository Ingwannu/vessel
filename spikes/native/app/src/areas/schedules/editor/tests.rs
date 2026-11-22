//! The preset control speaks the same field and selection the form displays.
use gpui::{AppContext as _, Entity, IntoElement, Render, TestAppContext, size};
use vessel_app_kit::{
    Scheme, Tokens,
    ui::{Context, Window, px},
};
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    schedules::editor::cadence::Choice,
};
use vessel_proto::NodeId;

use super::{Draft, Route, Shell, draw};
use crate::shell::language_tests::{init, named_button};

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            draw::render(shell, &Tokens::of(Scheme::Dark), window, false, cx)
                .expect("open schedule form")
        })
    }
}

#[gpui::test]
fn cadence_button_announces_its_field_and_preset(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.route = Route::Schedules;
            let connection = ConnectionEpochs::default()
                .admit(MachineKey::from_book("laptop"), NodeId::random())
                .unwrap();
            let mut draft = Draft::create(Some(&"UTC".parse().unwrap()));
            draft.cadence.select(Choice::Daily);
            shell.schedule_form_open(connection, draft, false, window, cx);
            shell
        }),
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.update(|window, _| window.refresh());
    assert!(named_button(cx, "Cadence (Daily 9:00)"));
}
