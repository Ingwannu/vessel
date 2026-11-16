//! Actual Clear and Reset choices retain their different stored meaning.
use gpui::TestAppContext;
use vessel_app_view::prefs::Bindings;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{Action, Platform, binding_id, default_context};

#[gpui::test]
fn settings_menu_clear_and_reset_remain_distinct(cx: &mut TestAppContext) {
    let (shell, cx) = super::setup(cx);
    let id = binding_id(Action::Palette, default_context(Platform::native())).unwrap();
    shell.update_in(cx, |shell, window, cx| {
        // Seed an existing preference; the menu operations themselves use the
        // production editor and preference transaction. This is not disk I/O.
        drop(shell.prefs.settings.shortcuts.insert(
            id.clone(),
            Bindings::try_from(vec!["Mod+J".parse().unwrap()]).unwrap(),
        ));
        shell.open_settings(Some(Section::Shortcuts), window, cx);
    });
    cx.run_until_parked();
    let menu = match Platform::native() {
        Platform::Mac => "shortcut-menu-command-center-toggle-cmd-k-mac",
        Platform::Other => "shortcut-menu-command-center-toggle-ctrl-k-non-mac",
    };
    for cleared in [true, false] {
        let bounds = cx.debug_bounds(menu).expect("canonical binding menu");
        cx.simulate_click(bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        // Bound override: Rebind, Clear, Reset. After Clear: Bind, Reset.
        cx.simulate_keystrokes("down down enter");
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| {
            let keys = shell.prefs.settings.shortcuts.get(&id);
            if cleared {
                assert!(keys.is_some_and(|keys| keys.keys().is_empty()));
            } else {
                assert!(
                    keys.is_none(),
                    "Reset restores the default, not an unassignment"
                );
            }
        });
    }
}
