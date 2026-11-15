//! Actual dialog, field callbacks and matching book answers over a detached link.
use gpui::{AppContext as _, Entity, Render, TestAppContext};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_link::{Command, Link, machines};
use vessel_app_view::machine_book::{Edit, Failure, Name};

use super::*;

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            div().children(dialogs(shell, &Tokens::of(Scheme::Dark), window, cx))
        })
    }
}

#[gpui::test]
fn editing_after_a_failed_answer_clears_the_displayed_error_and_retries_the_exact_draft(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(4).unwrap();
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
        shell: cx.new(|cx| Shell::with_test_link(window, cx, link)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.overview_open_rename(Name::parse("laptop").unwrap(), window, cx);
        });
    });
    cx.run_until_parked();
    cx.simulate_input("held-draft");
    cx.run_until_parked();
    let first = probe.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let typed = shell.areas.settings.overview.field.text(cx);
            assert_eq!(typed, "held-draft");
            shell.overview_rename(&typed, cx);
            shell
                .areas
                .settings
                .overview
                .rename
                .as_ref()
                .unwrap()
                .pending
                .clone()
                .unwrap()
        })
    });
    assert!(
        matches!(commands.try_recv().unwrap(), Command::Machines(machines::Ask::Edit(edit)) if edit == first)
    );
    cx.simulate_input("blocked");
    cx.run_until_parked();
    probe.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, cx| {
            assert_eq!(shell.areas.settings.overview.field.text(cx), "held-draft");
            shell.machines_answer(
                machines::Answer::Edited {
                    edit: Edit::Forget(Name::parse("other").unwrap()),
                    result: Err(Failure::Write),
                },
                cx,
            );
            assert_eq!(
                shell
                    .areas
                    .settings
                    .overview
                    .rename
                    .as_ref()
                    .unwrap()
                    .pending
                    .as_ref(),
                Some(&first)
            );
            shell.machines_answer(
                machines::Answer::Edited {
                    edit: first,
                    result: Err(Failure::Write),
                },
                cx,
            );
            assert!(rename_error(shell).is_some());
            assert_eq!(shell.areas.settings.overview.field.text(cx), "held-draft");
        });
    });
    cx.run_until_parked();
    cx.simulate_input("-retry");
    cx.run_until_parked();
    probe.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let typed = shell.areas.settings.overview.field.text(cx);
            assert_eq!(typed, "held-draft-retry");
            assert!(rename_error(shell).is_none());
            shell.overview_rename(&typed, cx);
        });
    });
    assert!(
        matches!(commands.try_recv().unwrap(), Command::Machines(machines::Ask::Edit(Edit::Rename { from, to }))
        if from.as_str() == "laptop" && to.as_str() == "held-draft-retry")
    );
    assert!(commands.try_recv().is_err());
}
