//! Real windows share one writer and exact receipts through their normal prefer path.

use super::*;
use gpui::{AppContext as _, Entity, VisualTestContext, WindowHandle};
use vessel_app_link::desktop::preferences::{Outcome, Receipt};
use vessel_app_view::prefs::{Grouping, Kept, PreferenceChange, Theme};

struct Pair {
    first: Entity<Shell>,
    second: WindowHandle<Shell>,
    path: std::path::PathBuf,
    _root: Temp,
}

fn mount(cx: &mut TestAppContext) -> (Pair, &mut VisualTestContext) {
    crate::shell::language_tests::init(cx);
    cx.executor().allow_parking();
    let root = Temp(std::env::temp_dir().join(format!(
        "vessel-real-window-prefs-{}",
        vessel_proto::SessionId::random()
    )));
    let path = root.0.join("prefs.json");
    let mut initial = Prefs {
        kept: Kept::File {
            path: path.display().to_string(),
        },
        ..Prefs::default()
    };
    initial.settings.appearance.theme = Theme::Dark;
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    cx.update(|cx| cx.set_global(super::super::super::testing::desktop(coordinator)));
    let (first, cx) = cx.add_window_view(Shell::without_io);
    first.update_in(cx, |shell, window, cx| {
        shell.prefs = initial.clone();
        start(shell, window, cx);
    });
    let second = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            cx.new(|cx| {
                let mut shell = Shell::without_io(window, cx);
                shell.prefs = initial;
                start(&mut shell, window, cx);
                shell
            })
        })
        .unwrap()
    });
    cx.run_until_parked();
    (
        Pair {
            first,
            second,
            path,
            _root: root,
        },
        cx,
    )
}

impl Pair {
    fn first(&self, change: &PreferenceChange, cx: &mut VisualTestContext) -> Receipt {
        self.first.update_in(cx, |shell, window, cx| {
            prefer_with_receipt(shell, change, window, cx)
                .unwrap()
                .unwrap()
        })
    }

    fn second(&self, change: &PreferenceChange, cx: &mut VisualTestContext) -> Receipt {
        cx.update(|_, cx| {
            self.second
                .update(cx, |shell, window, cx| {
                    prefer_with_receipt(shell, change, window, cx)
                        .unwrap()
                        .unwrap()
                })
                .unwrap()
        })
    }

    fn observe(&self, cx: &mut VisualTestContext) {
        let flushing = cx.update(|_, cx| cx.global::<Desktop>().preferences.flush());
        super::super::super::testing::wait(flushing).unwrap();
        cx.run_until_parked();
    }

    fn assert_converged(&self, theme: Theme, group: Grouping, cx: &mut VisualTestContext) {
        cx.update(|_, cx| {
            assert_eq!(cx.windows().len(), 2, "two real GPUI windows are mounted");
            let first = &self.first.read(cx).prefs;
            let second = &self.second.read(cx).unwrap().prefs;
            let stored = cx.global::<Desktop>().preferences.current();
            let disk: Prefs = serde_json::from_slice(&std::fs::read(&self.path).unwrap()).unwrap();
            for prefs in [first, second, stored.as_ref(), &disk] {
                assert_eq!(prefs.settings.appearance.theme, theme);
                assert_eq!(prefs.settings.sidebar.group, group);
            }
        });
    }
}

#[gpui::test]
fn two_windows_rebase_stale_edits_and_exact_failed_receipts_without_a_second_writer(
    cx: &mut TestAppContext,
) {
    let (pair, cx) = mount(cx);
    // Both normal edits publish before either foreground watch is observed.
    let first = pair.first(&PreferenceChange::Theme(Theme::Light), cx);
    let second = pair.second(&PreferenceChange::SidebarGroup(Grouping::Status), cx);
    pair.observe(cx);
    assert!(matches!(
        super::super::super::testing::wait(first.completed())
            .unwrap()
            .outcome(),
        Outcome::Saved
    ));
    assert!(matches!(
        super::super::super::testing::wait(second.completed())
            .unwrap()
            .outcome(),
        Outcome::Saved
    ));
    pair.assert_converged(Theme::Light, Grouping::Status, cx);

    // Conflicting writes choose the last accepted delta, in both actual windows.
    let first = pair.first(&PreferenceChange::Theme(Theme::Dark), cx);
    let second = pair.second(&PreferenceChange::Theme(Theme::System), cx);
    pair.observe(cx);
    let first = super::super::super::testing::wait(first.completed()).unwrap();
    let second = super::super::super::testing::wait(second.completed()).unwrap();
    assert!(first.revision() < second.revision());
    pair.assert_converged(Theme::System, Grouping::Status, cx);

    // Force a real atomic-file failure, then succeed before pumping observers.
    std::fs::remove_file(&pair.path).unwrap();
    std::fs::create_dir(&pair.path).unwrap();
    let failed = pair.first(&PreferenceChange::Theme(Theme::Light), cx);
    let failed = super::super::super::testing::wait(failed.completed()).unwrap();
    assert!(matches!(failed.outcome(), Outcome::Failed(_)));
    std::fs::remove_dir(&pair.path).unwrap();
    let succeeded = pair.second(&PreferenceChange::SidebarGroup(Grouping::Machine), cx);
    pair.observe(cx);
    let succeeded = super::super::super::testing::wait(succeeded.completed()).unwrap();
    assert!(failed.revision() < succeeded.revision());
    pair.assert_converged(Theme::System, Grouping::Machine, cx);
    pair.first.update_in(cx, |shell, window, cx| {
        assert!(
            shell.notice.is_none(),
            "coalesced success did not report this exact failure"
        );
        acknowledge(shell, &failed, window, cx);
        assert_eq!(
            shell.notice.as_deref(),
            Some(shell.words.desktop_shell().preferences_failed())
        );
        shell.notice = None;
        acknowledge(shell, &failed, window, cx);
        assert!(
            shell.notice.is_none(),
            "one exact failure is not reported twice"
        );
    });
    pair.assert_converged(Theme::System, Grouping::Machine, cx);
}
