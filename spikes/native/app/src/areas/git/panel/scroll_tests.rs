//! Actual deferred callbacks preserve replacement/retired owner incarnations.
//! Observations run before paint; no synthetic source range or callback receipt.
use super::*;
use crate::areas::git::panel::{bodies, target_tests};
use crate::shell::language_tests;
use gpui::TestAppContext;
use std::{cell::Cell, rc::Rc};
use vessel_app_link::{Command, Link};
use vessel_proto::workspace::RelPath;

#[derive(Clone, Copy)]
enum Transition {
    Current,
    Refreshed,
    Retired,
}

#[gpui::test]
fn deferred_scroll_clears_only_its_original_current_owner(cx: &mut TestAppContext) {
    language_tests::init(cx);
    for transition in [
        Transition::Current,
        Transition::Refreshed,
        Transition::Retired,
    ] {
        let (link, mut inbox) = Link::detached(32).unwrap();
        let (shell, cx) =
            cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
        let observed = Rc::new(Cell::new(None));
        let seen = Rc::clone(&observed);
        shell.update_in(cx, |shell, window, cx| {
            let (address, _) = bodies::prepare(shell, window, cx, 1, &mut inbox);
            target_tests::background_agent(shell, &address, window, cx, &mut inbox);
            let captured =
                Scroll::capture(&shell.areas.git.panels.entries[&address], &address).unwrap();
            captured.defer(&cx.entity().downgrade(), window, cx);
            transition_before_dispatch(shell, &captured, transition, window, cx, &mut inbox);
            let sentinel: RelPath = "successor-sentinel.rs".parse().unwrap();
            shell.areas.git.panels.entries[&address]
                .visible_bodies
                .mark(&sentinel);
            let owner = cx.entity().downgrade();
            // This second actual deferred callback observes the first before
            // paint can replace the current frame's visibility observations.
            window.defer(cx, move |_, cx| {
                let retained = owner.upgrade().unwrap().read_with(cx, |shell, _| {
                    shell.areas.git.panels.entries[&address]
                        .visible_bodies
                        .contains(&sentinel)
                });
                seen.set(Some(retained));
            });
        });
        cx.run_until_parked();
        let expected = match transition {
            Transition::Current => false,
            Transition::Refreshed | Transition::Retired => true,
        };
        assert_eq!(observed.get(), Some(expected));
    }
}

fn transition_before_dispatch(
    shell: &mut Shell,
    captured: &Scroll,
    transition: Transition,
    window: &Window,
    cx: &mut Context<'_, Shell>,
    inbox: &mut tokio::sync::mpsc::Receiver<Command>,
) {
    match transition {
        Transition::Current => {}
        Transition::Refreshed => {
            shell.git_panel_refresh(
                captured.address.clone(),
                captured.target.clone(),
                window,
                cx,
            );
            let Command::GitPanel(vessel_app_link::git_panel::Ask::Locate(_)) =
                inbox.try_recv().unwrap()
            else {
                panic!("one fresh root acquisition");
            };
            assert_ne!(
                shell.areas.git.panels.entries[&captured.address].instance,
                Some(captured.instance)
            );
        }
        Transition::Retired => {
            assert!(shell.git_panels_retired(&captured.context.tag()));
            assert_eq!(
                shell.areas.git.panels.entries[&captured.address].context,
                None
            );
        }
    }
    assert!(matches!(
        inbox.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
}
