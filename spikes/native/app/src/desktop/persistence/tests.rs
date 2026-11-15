//! A real App fence rejects resize/edit events between capture, acknowledgment and exit.

use super::*;
use crate::desktop::{WindowClose, WindowRecord};
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_link::desktop::{
    preferences::Coordinator, store::Store, window_writer::WindowWriter,
};
use vessel_app_view::{
    desktop::window::Presentation,
    prefs::{PreferenceChange, Theme},
};

struct Temp(std::path::PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _cleanup = std::fs::remove_dir_all(&self.0);
    }
}

fn owner(cx: &mut TestAppContext) -> (Temp, Store) {
    language_tests::init(cx);
    let root = Temp(std::env::temp_dir().join(format!(
        "vessel-state-fence-{}",
        vessel_proto::SessionId::random()
    )));
    let store = Store::new(root.0.clone()).unwrap();
    let writer = WindowWriter::start(store.clone()).unwrap();
    let coordinator = Coordinator::start(vessel_app_link::Prefs::default()).unwrap();
    cx.update(|cx| {
        let mut desktop = crate::desktop::testing::desktop(coordinator);
        desktop.writer = Some(writer);
        cx.set_global(desktop);
    });
    (root, store)
}

#[gpui::test]
fn app_fence_rejects_late_geometry_and_unreconciled_preference_edits(cx: &mut TestAppContext) {
    // The real preference writer wakes the production UI watch from its thread.
    cx.executor().allow_parking();
    let (_root, store) = owner(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, crate::desktop::preferences::start);
    cx.update(|window, cx| {
        cx.global_mut::<Desktop>().records.push(WindowRecord {
            window: window.window_handle(),
            shell: shell.downgrade(),
            close: WindowClose::Idle,
        });
    });
    cx.run_until_parked();
    let flushing = cx.update(|_, cx| flush_state(cx));
    let moved = WindowState::new(Some((40.0, 50.0)), 900.0, 700.0, Presentation::Windowed).unwrap();
    cx.update(|_, cx| {
        cx.global_mut::<Desktop>().remember(moved);
    });
    let old = super::super::testing::wait(flushing).unwrap();
    cx.update(|_, cx| assert!(matches!(flush_current(&old, cx), StateReadiness::Changed)));
    let flushing = cx.update(|_, cx| flush_state(cx));
    let current = super::super::testing::wait(flushing).unwrap();
    cx.update(|_, cx| {
        assert!(matches!(
            flush_current(&current, cx),
            StateReadiness::LaunchOnly
        ))
    });
    assert_eq!(store.load_window().unwrap(), Some(moved));
    shell.update_in(cx, |shell, window, cx| {
        shell.prefer(&PreferenceChange::Theme(Theme::Light), window, cx)
    });
    cx.update(|_, cx| {
        assert!(matches!(
            flush_current(&current, cx),
            StateReadiness::Changed
        ))
    });
    let flushing = cx.update(|_, cx| flush_state(cx));
    let written = super::super::testing::wait(flushing).unwrap();
    cx.update(|_, cx| {
        assert!(matches!(
            flush_current(&written, cx),
            StateReadiness::Changed
        ))
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(matches!(
            flush_current(&written, cx),
            StateReadiness::LaunchOnly
        ))
    });
}

#[gpui::test]
fn equal_bounds_after_intervening_publications_still_require_a_fresh_receipt(
    cx: &mut TestAppContext,
) {
    let (_root, store) = owner(cx);
    let flushing = cx.update(|cx| flush_state(cx));
    let old = super::super::testing::wait(flushing).unwrap();
    let moved = WindowState::new(Some((40.0, 50.0)), 900.0, 700.0, Presentation::Windowed).unwrap();
    cx.update(|cx| {
        let desktop = cx.global_mut::<Desktop>();
        desktop.remember(moved);
        desktop.remember(WindowState::default());
        // Value equality alone would admit exit while a different state can
        // still be in the writer; the same A requires a new acknowledgment.
        assert!(matches!(flush_current(&old, cx), StateReadiness::Changed));
    });
    let flushing = cx.update(|cx| flush_state(cx));
    let current = super::super::testing::wait(flushing).unwrap();
    cx.update(|cx| {
        assert!(matches!(
            flush_current(&current, cx),
            StateReadiness::LaunchOnly
        ));
    });
    assert_eq!(store.load_window().unwrap(), Some(WindowState::default()));
}
