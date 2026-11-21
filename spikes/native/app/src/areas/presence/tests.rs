//! The window's presence on GPUI's test platform: opening reaches the link
//! at once, before the first heartbeat (app review R1).

use gpui::TestAppContext;
use vessel_app_view::presence::Told;
use vessel_proto::push::PushActivity;

use crate::shell::{Shell, language_tests};

#[gpui::test]
fn opening_reaches_the_link_at_once_and_a_new_connection_hears_it_as_input(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    // No input and no time passed: only what the shell does on its own.
    cx.run_until_parked();
    shell.update(cx, |shell, _cx| {
        // At least the one at once (the window's activation, if the test
        // platform reports one, is input too); what the link holds is the
        // last.
        let held = shell
            .areas
            .presence
            .handed
            .last()
            .expect("a snapshot at once, before the 15 s heartbeat");
        // A machine that connects now hears opening as input.
        let mut told = Told::default();
        let report = told.report("connection", "lab", held, 0);
        assert_eq!(report.activity, PushActivity::Active);
    });
}
