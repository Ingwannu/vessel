//! Actual Shell -> layout registry -> Git list pressure and native wheel input.
use super::*;
use crate::shell::language_tests;
use gpui::{Entity, TestAppContext, VisualTestContext, point, px, size};
use tokio::sync::mpsc;
use vessel_app_link::git_panel::{Answer, Outcome};
use vessel_app_link::{Command, Link};
use vessel_app_view::layout::{LayoutKey, OpenIntent, Placement, Target};
use vessel_proto::workspace::{ChangeStatus, DiffContent, FileDiff};

fn background_agent(
    shell: &mut Shell,
    address: &PanelAddress,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
    inbox: &mut mpsc::Receiver<Command>,
) {
    let LayoutKey::Session { session, .. } = address.workspace else {
        panic!("legacy session fixture");
    };
    let _tab = shell
        .layout_open_target(
            &address.workspace,
            Target::Agent(session),
            OpenIntent::Background,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    let Command::SavePrefs(prefs) = inbox.try_recv().unwrap() else {
        panic!("one background-agent layout persistence");
    };
    assert_eq!(*prefs, shell.prefs);
    assert!(matches!(
        inbox.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

fn finish_visible_calls(
    shell: &Entity<Shell>,
    inbox: &mut mpsc::Receiver<Command>,
    cx: &mut VisualTestContext,
) -> usize {
    let mut total = 0_usize;
    for _frame in 0..64 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        let mut calls = 0_usize;
        shell.update_in(cx, |shell, _window, cx| {
            while let Ok(command) = inbox.try_recv() {
                let Command::GitPanel(Ask::File {
                    address,
                    context,
                    ticket,
                }) = command
                else {
                    panic!("only newly admitted file reads");
                };
                let path = ticket.path().clone();
                shell.git_panel_answer(
                    Answer {
                        address,
                        context,
                        outcome: Outcome::File(
                            ticket,
                            Ok(FileDiff {
                                path,
                                old_path: None,
                                status: ChangeStatus::Modified,
                                mode: None,
                                content: DiffContent::Unchanged,
                                more_from: None,
                            }),
                        ),
                    },
                    cx,
                );
                calls = calls.saturating_add(1);
            }
            assert!(matches!(
                inbox.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            assert!(shell.areas.git.panels.body_order.len() <= MAX_BODY_FILES);
        });
        total = total.saturating_add(calls);
        if calls == 0 {
            return total;
        }
    }
    panic!("visible file requests did not quiesce within the bounded frame budget");
}

#[gpui::test]
fn actual_registry_visible_slots_quiesce_and_outer_wheel_releases_old_vetoes(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(128).unwrap();
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let mut shell = Shell::with_test_link(window, cx, link);
        let (address, _paths) = super::tests::prepare(&mut shell, window, cx, 80, &mut inbox);
        background_agent(&mut shell, &address, window, cx, &mut inbox);
        shell
    });
    cx.simulate_resize(size(px(1280.), px(2400.)));
    assert_eq!(finish_visible_calls(&shell, &mut inbox, cx), MAX_BODY_FILES);
    assert_eq!(finish_visible_calls(&shell, &mut inbox, cx), 0);
    let extra = cx
        .debug_bounds("git-file-section-32")
        .expect("33rd actual file section");
    assert!(extra.size.height > px(0.));
    assert!(extra.top() >= px(0.) && extra.bottom() < px(2400.));
    let before = shell.read_with(cx, |shell, _| {
        let entry = shell.areas.git.panels.entries.values().next().unwrap();
        (entry.files.top(), shell.areas.git.panels.body_order.clone())
    });
    cx.simulate_event(gpui::ScrollEvent {
        position: point(px(1000.), px(1600.)),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-1400.))),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    cx.run_until_parked();
    let after = shell.read_with(cx, |shell, _| {
        shell
            .areas
            .git
            .panels
            .entries
            .values()
            .next()
            .unwrap()
            .files
            .top()
    });
    assert_ne!(
        before.0, after,
        "native outer-owner wheel must actually move"
    );
    assert!(finish_visible_calls(&shell, &mut inbox, cx) > 0);
    shell.read_with(cx, |shell, _| {
        let panels = &shell.areas.git.panels;
        assert_eq!(panels.body_order.len(), MAX_BODY_FILES);
        assert!(panels.body_order.iter().any(|key| !before.1.contains(key)));
    });
    assert_eq!(finish_visible_calls(&shell, &mut inbox, cx), 0);
}
