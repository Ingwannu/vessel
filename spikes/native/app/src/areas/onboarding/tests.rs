//! UI-controller regression: Finish must persist the model's completion effect.

use super::*;
use gpui::TestAppContext;
use vessel_app_link::onboarding::Answer;
use vessel_app_view::Reach;
use vessel_app_view::onboarding::scope::Target;
use vessel_proto::NodeId;

#[gpui::test]
fn finish_records_completion_after_the_model_has_already_hidden(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let node = NodeId::random();
        let mut machine = vessel_app_view::Machine::unreachable("laptop".to_owned(), "fixture");
        machine.reach = Reach::Online;
        machine.node = Some(node);
        let context = vessel_app_view::connection::ConnectionEpochs::default()
            .admit(MachineKey::from_book("laptop"), node)
            .unwrap();
        machine.connection_epoch = Some(context.epoch());
        shell.fleet.machines = vec![machine];
        let target = Target::authenticated(context);
        shell
            .areas
            .onboarding
            .model
            .reopen(target.clone(), Layout::Wide)
            .unwrap();
        shell.areas.onboarding.model.go_to(Step::Phone).unwrap();
        let visit = match shell.areas.onboarding.model.state() {
            State::Open(open) => open.visit(),
            State::Deciding | State::Hidden => panic!("setup fixture did not open"),
        };
        shell.onboarding_answer(
            Answer::Bound {
                target,
                visit,
                result: Ok(()),
            },
            cx,
        );
        shell.setup_continue(visit, window, cx);
        assert!(matches!(
            shell.areas.onboarding.model.state(),
            State::Hidden
        ));
        assert!(
            shell.prefs.onboarding.is_some(),
            "Finish swallowed the completed model's effect"
        );
        assert!(shell.areas.onboarding.shown.is_none());
        // Repeated completion cannot erase or rewrite the accepted timestamp.
        let recorded = shell.prefs.onboarding.clone();
        shell.setup_complete(visit, window, cx);
        assert_eq!(shell.prefs.onboarding, recorded);
    });
}

#[gpui::test]
fn stale_close_and_rail_actions_cannot_change_a_reopened_visit(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = Target::authenticated(
            vessel_app_view::connection::ConnectionEpochs::default()
                .admit(MachineKey::from_book("fixture"), NodeId::random())
                .unwrap(),
        );
        let model = &mut shell.areas.onboarding.model;
        model.reopen(target.clone(), Layout::Wide).unwrap();
        let old = match model.state() {
            State::Open(open) => open.visit(),
            State::Deciding | State::Hidden => panic!("fixture did not open"),
        };
        model.reopen(target, Layout::Wide).unwrap();
        model.go_to(Step::Projects).unwrap();
        let before = model.state().clone();
        shell.setup_complete(old, window, cx);
        shell.setup_move(old, Some(Step::Phone), window, cx);
        assert_eq!(shell.areas.onboarding.model.state(), &before);
        assert!(shell.prefs.onboarding.is_none());
        let current = match before {
            State::Open(open) => open.visit(),
            State::Deciding | State::Hidden => panic!("fixture closed"),
        };
        shell.setup_complete(current, window, cx);
        assert!(matches!(
            shell.areas.onboarding.model.state(),
            State::Hidden
        ));
        let completed = shell.prefs.onboarding.clone();
        shell.setup_complete(current, window, cx);
        assert_eq!(shell.prefs.onboarding, completed);
    });
}
