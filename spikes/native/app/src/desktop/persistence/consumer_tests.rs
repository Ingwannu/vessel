//! The lifecycle consumer must wait for accepted edits to reach their UI owner.

use crate::desktop::{Desktop, WindowClose, WindowRecord};
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_link::desktop::preferences::Coordinator;
use vessel_app_view::prefs::{PreferenceChange, Theme};

#[gpui::test]
async fn exit_effect_observes_settled_accepted_preferences_on_the_same_app_turn(
    cx: &mut TestAppContext,
) {
    // This exercises the actual blocking writer and foreground watch together.
    cx.executor().allow_parking();
    language_tests::init(cx);
    let coordinator = Coordinator::start(vessel_app_link::Prefs::default()).unwrap();
    cx.update(|cx| cx.set_global(crate::desktop::testing::desktop(coordinator)));
    let (shell, window) = cx.add_window_view(Shell::without_io);
    shell.update_in(window, crate::desktop::preferences::start);
    window.update(|window, cx| {
        cx.global_mut::<Desktop>().records.push(WindowRecord {
            window: window.window_handle(),
            shell: shell.downgrade(),
            close: WindowClose::Idle,
        });
    });
    shell.update_in(window, |shell, window, cx| {
        shell.prefer(&PreferenceChange::Theme(Theme::Light), window, cx);
        assert!(crate::quit::begin(cx));
    });
    let accepted = shell.clone();
    let admitted = window
        .spawn(async move |cx| {
            crate::local_node::flush_then(&cx, |cx| {
                assert!(crate::quit::is_finalizing(cx));
                assert_eq!(
                    accepted.read(cx).prefs.settings.appearance.theme,
                    Theme::Light
                );
                assert_eq!(
                    cx.global::<Desktop>()
                        .preferences
                        .current()
                        .settings
                        .appearance
                        .theme,
                    Theme::Light
                );
                crate::quit::finish(cx);
                true
            })
            .await
        })
        .await
        .unwrap();
    assert!(admitted);
}
