//! Real controller, dialog dismissal and language dispatch, with no link or opener.
use super::*;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement, ParentElement as _, div};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::orchestration::{Output, Picker};
use vessel_proto::NodeId;
use vessel_proto::skills::{SkillNames, SkillSelection, SkillsSaved, SkillsSnapshot, SkillsState};

fn status() -> SkillsSnapshot {
    let names = SkillNames::new(vec![
        "vessel".parse().unwrap(),
        "vessel-advisor".parse().unwrap(),
    ])
    .unwrap();
    SkillsSnapshot {
        state: SkillsState::UpToDate,
        ops: vec![],
        available: names.clone(),
        installed: names,
        selection: SkillSelection::All,
    }
}
fn seed(shell: &mut Shell) -> Scope {
    let state = &mut shell.areas.settings.orchestration;
    let scope = Scope {
        consumer: Consumer::Settings,
        target: vessel_app_view::connection::ConnectionEpochs::default()
            .admit(
                vessel_app_view::connection::MachineKey::from_book("isolated"),
                NodeId::random(),
            )
            .unwrap(),
        visit: Visit::new(state.ticket().unwrap()),
    };
    let mut page = Page::new(scope.clone());
    let task = page.read_skills(state.ticket().unwrap()).unwrap();
    assert!(page.answered(Answer {
        task,
        result: Ok(Output::Skills(status())),
    }));
    state.page = Some(page);
    scope
}
struct Probe {
    shell: Entity<Shell>,
    _changed: gpui::Subscription,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            shell.areas.settings.orchestration.settle_focus(window, cx);
            let tokens = Tokens::of(Scheme::Dark);
            div()
                .child(shell.areas.session.composer.element(&tokens, false, cx))
                .children(dialog(shell, &tokens, window, cx))
        })
    }
}

#[gpui::test]
fn escape_waits_for_save_ack_then_returns_focus_to_the_original_opener(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let _scope = seed(&mut shell);
            shell
        });
        let changed = cx.observe(&shell, |_probe, _shell, cx| cx.notify());
        Probe {
            shell,
            _changed: changed,
        }
    });
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.areas.session.composer.focus(window, cx);
            let scope = shell
                .areas
                .settings
                .orchestration
                .page
                .as_ref()
                .unwrap()
                .scope()
                .clone();
            shell.orchestration_intent(&scope, Intent::Choose, window, cx);
            assert!(
                !shell
                    .areas
                    .session
                    .composer
                    .focus_handle(cx)
                    .is_focused(window)
            );
        });
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    let pending = probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            assert!(
                shell
                    .areas
                    .settings
                    .orchestration
                    .page
                    .as_ref()
                    .unwrap()
                    .picker
                    .is_none()
            );
            assert!(
                shell
                    .areas
                    .session
                    .composer
                    .focus_handle(cx)
                    .is_focused(window)
            );
            let scope = shell
                .areas
                .settings
                .orchestration
                .page
                .as_ref()
                .unwrap()
                .scope()
                .clone();
            shell.orchestration_intent(&scope, Intent::Choose, window, cx);
            let state = &mut shell.areas.settings.orchestration;
            let ticket = state.ticket().unwrap();
            let task = state.page.as_mut().unwrap().save_picker(ticket).unwrap();
            // Retain the exact task in the test harness, not an untyped RPC or
            // a real node that could touch orchestration directories.
            task
        })
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let state = &mut shell.areas.settings.orchestration;
            let page = state.page.as_ref().unwrap();
            assert!(page.picker.is_some());
            assert!(page.skills.pending());
            assert!(
                shell
                    .orchestration_answer(Answer {
                        task: pending,
                        result: Ok(Output::Saved(SkillsSaved {
                            snapshot: status(),
                            confirmation_required: None,
                        })),
                    })
                    .is_none()
            );
            shell.areas.settings.orchestration.settle_focus(window, cx);
            assert!(
                shell
                    .areas
                    .settings
                    .orchestration
                    .page
                    .as_ref()
                    .unwrap()
                    .picker
                    .is_none()
            );
            assert!(
                shell
                    .areas
                    .session
                    .composer
                    .focus_handle(cx)
                    .is_focused(window)
            );
        });
    });
}

#[gpui::test]
fn old_visit_dismissal_and_ack_cannot_close_a_new_picker(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        let changed = cx.observe(&shell, |_probe, _shell, cx| cx.notify());
        Probe {
            shell,
            _changed: changed,
        }
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let old = seed(shell);
            shell.orchestration_intent(&old, Intent::Choose, window, cx);
            let state = &mut shell.areas.settings.orchestration;
            let ticket = state.ticket().unwrap();
            let task = state.page.as_mut().unwrap().save_picker(ticket).unwrap();
            let new = seed(shell);
            shell.orchestration_intent(&new, Intent::Choose, window, cx);
            shell.orchestration_intent(&old, Intent::Cancel, window, cx);
            assert!(
                shell
                    .orchestration_answer(Answer {
                        task,
                        result: Ok(Output::Saved(SkillsSaved {
                            snapshot: status(),
                            confirmation_required: None,
                        })),
                    })
                    .is_none()
            );
            let page = shell.areas.settings.orchestration.page.as_ref().unwrap();
            assert_eq!(page.scope(), &new);
            assert!(page.picker.is_some());
            assert!(!page.skills.pending());
        });
    });
}

#[gpui::test]
fn language_change_keeps_the_open_selection_draft_and_real_input(cx: &mut TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            let scope = seed(shell);
            shell.orchestration_intent(&scope, Intent::Choose, window, cx);
            shell.orchestration_intent(&scope, Intent::All, window, cx);
            shell.orchestration_intent(
                &scope,
                Intent::Toggle("vessel-advisor".parse().unwrap()),
                window,
                cx,
            );
            assert_eq!(shell.words.orchestration().choose(), "Choose skills");
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            let page = shell.areas.settings.orchestration.page.as_ref().unwrap();
            let Picker { draft, .. } = page.picker.as_ref().unwrap();
            assert_eq!(
                draft,
                &SkillSelection::Custom {
                    skills: SkillNames::new(vec!["vessel".parse().unwrap()]).unwrap(),
                }
            );
            assert_ne!(shell.words.orchestration().choose(), "Choose skills");
        },
    );
}

#[gpui::test]
fn a_retired_snapshot_cannot_reopen_the_page_but_a_new_admission_can(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        let changed = cx.observe(&shell, |_probe, _shell, cx| cx.notify());
        Probe {
            shell,
            _changed: changed,
        }
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let scope = seed(shell);
            let original = scope.target;
            let mut machine =
                vessel_app_view::Machine::unreachable("isolated".into(), "sessions forbidden");
            machine.node = Some(original.node());
            machine.connection_epoch = Some(original.epoch());
            machine.protocol = Some(vessel_proto::PROTOCOL_VERSION);
            shell.fleet.machines = vec![machine];
            shell.route = Route::Settings;
            shell.areas.settings.section = Section::Overview;
            shell.areas.settings.pushed = true;
            assert!(shell.orchestration_retired(&original.tag()));
            shell.settle_orchestration(window, cx);
            assert!(shell.areas.settings.orchestration.page.is_none());
            let mut admissions = vessel_app_view::connection::ConnectionEpochs::default();
            let _old = admissions
                .admit(original.machine().clone(), original.node())
                .unwrap();
            let next = admissions
                .admit(original.machine().clone(), original.node())
                .unwrap();
            shell.fleet.machines.first_mut().unwrap().connection_epoch = Some(next.epoch());
            shell.settle_orchestration(window, cx);
            let page = shell.areas.settings.orchestration.page.as_ref().unwrap();
            assert_eq!(page.scope().target, next);
            assert!(!shell.orchestration_retired(&original.tag()));
            assert!(shell.areas.settings.orchestration.page.is_some());
        });
    });
}

#[gpui::test]
fn reopening_in_one_visit_rejects_the_previous_dialogs_close_and_save(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        let changed = cx.observe(&shell, |_probe, _shell, cx| cx.notify());
        Probe {
            shell,
            _changed: changed,
        }
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let scope = seed(shell);
            shell.orchestration_intent(&scope, Intent::Choose, window, cx);
            let old = DialogScope {
                scope: scope.clone(),
                ticket: shell.areas.settings.orchestration.dialog_ticket.unwrap(),
            };
            shell.orchestration_dialog_intent(&old, Intent::Cancel, window, cx);
            shell.orchestration_intent(&scope, Intent::Choose, window, cx);
            let current = shell.areas.settings.orchestration.dialog_ticket.unwrap();
            assert_ne!(old.ticket, current);
            shell.orchestration_dialog_intent(&old, Intent::Cancel, window, cx);
            shell.orchestration_dialog_intent(&old, Intent::Save, window, cx);
            let page = shell.areas.settings.orchestration.page.as_ref().unwrap();
            assert!(page.picker.is_some());
            assert!(!page.skills.pending());
            assert!(page.skills.problem().is_none());
            shell.orchestration_dialog_intent(
                &DialogScope {
                    scope,
                    ticket: current,
                },
                Intent::Cancel,
                window,
                cx,
            );
            assert!(
                shell
                    .areas
                    .settings
                    .orchestration
                    .page
                    .as_ref()
                    .unwrap()
                    .picker
                    .is_none()
            );
        });
    });
}
