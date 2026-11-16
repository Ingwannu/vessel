//! A real App fence rejects resize/edit events between capture, acknowledgment and exit.

use super::*;
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, TestAppContext};
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
    // This case deliberately connects GPUI's actual foreground observer to a
    // real std-thread file owner. GPUI's mixed-I/O test mode admits that wake;
    // writer receipts still order each step, and wait() never pumps the UI.
    cx.executor().allow_parking();
    let (_root, store) = owner(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, crate::desktop::preferences::start);
    cx.update(|window, cx| {
        crate::host_windows::register(&shell, window, cx);
    });
    cx.run_until_parked();
    let flushing = cx.cx.update(|cx| flush_state(cx));
    let moved = WindowState::new(Some((40.0, 50.0)), 900.0, 700.0, Presentation::Windowed).unwrap();
    cx.cx.update(|cx| {
        cx.global_mut::<Desktop>().remember(moved);
    });
    let old = super::super::testing::wait(flushing).unwrap();
    cx.cx
        .update(|cx| assert!(matches!(flush_current(&old, cx), StateReadiness::Changed)));
    let flushing = cx.cx.update(|cx| flush_state(cx));
    let current = super::super::testing::wait(flushing).unwrap();
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&current, cx),
            StateReadiness::LaunchOnly
        ))
    });
    assert_eq!(store.load_window().unwrap(), Some(moved));
    shell.update_in(cx, |shell, window, cx| {
        shell.prefer(&PreferenceChange::Theme(Theme::Light), window, cx)
    });
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&current, cx),
            StateReadiness::Changed
        ))
    });
    let flushing = cx.cx.update(|cx| flush_state(cx));
    let written = super::super::testing::wait(flushing).unwrap();
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&written, cx),
            StateReadiness::Changed
        ))
    });
    cx.run_until_parked();
    cx.cx.update(|cx| {
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

#[gpui::test]
fn state_fence_refuses_a_retained_old_root_and_a_successor_without_its_client(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (_root, _store) = owner(cx);
    let (old, cx) = cx.add_window_view(Shell::without_io);
    old.update_in(cx, |shell, window, cx| {
        crate::desktop::preferences::start(shell, window, cx);
        crate::host_windows::register(&cx.entity(), window, cx);
    });
    cx.run_until_parked();
    let flushing = cx.cx.update(|cx| flush_state(cx));
    let receipt = super::super::testing::wait(flushing).unwrap();
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::LaunchOnly
        ));
    });
    let replacement = cx.update(|window, cx| window.replace_root(cx, Shell::without_io));
    cx.cx.update(|cx| {
        assert!(old.downgrade().upgrade().is_some());
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::Changed
        ));
    });
    replacement.update_in(cx, |_shell, window, cx| {
        crate::host_windows::register(&cx.entity(), window, cx);
    });
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::Changed
        ));
    });
    replacement.update_in(cx, crate::desktop::preferences::start);
    cx.run_until_parked();
    cx.cx.update(|cx| {
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::LaunchOnly
        ));
    });
}

#[gpui::test]
fn state_fence_waits_for_a_passive_window_to_apply_the_saved_canonical_revision(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let (_root, _store) = owner(cx);
    let (first, cx) = cx.add_window_view(Shell::without_io);
    first.update_in(cx, |shell, window, cx| {
        crate::desktop::preferences::start(shell, window, cx);
        crate::host_windows::register(&cx.entity(), window, cx);
    });
    let second = cx.cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            let shell = cx.new(|cx| Shell::without_io(window, cx));
            shell.update(cx, |shell, cx| {
                crate::desktop::preferences::start(shell, window, cx);
            });
            crate::host_windows::register(&shell, window, cx);
            shell
        })
        .unwrap()
    });
    cx.run_until_parked();
    first.update_in(cx, |shell, window, cx| {
        shell.prefer(&PreferenceChange::Theme(Theme::Light), window, cx);
    });
    let flushing = cx.cx.update(|cx| flush_state(cx));
    let receipt = super::super::testing::wait(flushing).unwrap();
    // Reconcile only the initiating window, without pumping the other watch.
    let snapshot = receipt.preferences.as_ref().unwrap().snapshot();
    first.update_in(cx, |shell, window, cx| {
        crate::desktop::preferences::acknowledge(shell, snapshot, window, cx);
        assert_eq!(shell.prefs.settings.appearance.theme, Theme::Light);
    });
    cx.cx.update(|cx| {
        assert_eq!(cx.windows().len(), 2);
        assert_ne!(
            second.read(cx).unwrap().prefs.settings.appearance.theme,
            Theme::Light
        );
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::Changed
        ));
    });
    cx.run_until_parked();
    cx.cx.update(|cx| {
        assert_eq!(
            second.read(cx).unwrap().prefs.settings.appearance.theme,
            Theme::Light
        );
        assert!(matches!(
            flush_current(&receipt, cx),
            StateReadiness::LaunchOnly
        ));
    });
}
