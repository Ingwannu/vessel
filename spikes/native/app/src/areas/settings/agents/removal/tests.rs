//! A queued old decision must neither dismiss nor send a newer confirmation.
use super::super::testing::{self, COMMAND_CAPACITY};
use super::{Rc, Shell};
use gpui::{Bounds, TestAppContext, point, px, size};
use vessel_app_kit::menu::TriggerAnchor;
use vessel_app_link::{Command, Link, agents::Ask};
use vessel_app_view::agent_status::pages::{MachineKey, Operation};

#[gpui::test]
fn only_the_current_confirmation_can_send_one_remove(cx: &mut TestAppContext) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let info = testing::added("remove-fixture");
        let agent = info.agent.clone();
        testing::select(shell, "one", vec![info], cx);
        let machine = MachineKey::from_book("one");
        let anchor = TriggerAnchor::default();
        anchor.set(Some(Bounds::new(
            point(px(100.0), px(100.0)),
            size(px(32.0), px(32.0)),
        )));
        shell.agent_open_actions(&machine, testing::node(), &agent, &anchor, window, cx);
        let old = Rc::clone(shell.areas.settings.agents.removal.target.as_ref().unwrap());
        shell.agent_choose_removal(&old, window, cx);
        shell.agent_open_actions(&machine, testing::node(), &agent, &anchor, window, cx);
        let current = Rc::clone(shell.areas.settings.agents.removal.target.as_ref().unwrap());
        shell.agent_choose_removal(&current, window, cx);
        shell.agent_cancel_removal(&old, window, cx);
        shell.agent_confirm_removal(&old, window, cx);
        assert!(commands.try_recv().is_err());
        assert!(Rc::ptr_eq(
            shell
                .areas
                .settings
                .agents
                .removal
                .confirmation
                .as_ref()
                .unwrap(),
            &current
        ));
        shell.agent_confirm_removal(&current, window, cx);
        let Command::Agents {
            machine: sent,
            ask: Ask::Remove(removal),
            ..
        } = commands.try_recv().unwrap()
        else {
            panic!("one confirmed removal");
        };
        assert_eq!(sent, machine);
        let Operation::Remove { node, name } = Ask::Remove(removal).operation() else {
            panic!("Remove operation");
        };
        assert_eq!(node, testing::node());
        assert_eq!(name.as_str(), "remove-fixture");
        shell.agent_confirm_removal(&current, window, cx);
        assert!(commands.try_recv().is_err());
        assert!(shell.areas.settings.agents.removal.confirmation.is_none());
    });
}
