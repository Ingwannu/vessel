//! A discarded form cannot control its successor; accepted Add never probes.
use super::super::testing::{self, COMMAND_CAPACITY};
use super::{Context, Intent, Phase, Rc, Shell, Window};
use gpui::{AppContext as _, Entity, Render, TestAppContext};
use vessel_app_kit::{
    Scheme, Tokens,
    ui::{IntoElement, ParentElement as _, div},
};
use vessel_app_link::{
    Command, Link,
    agents::{Answer, Ask},
};
use vessel_app_view::agent_status::pages::{MachineKey, Operation};

struct Probe {
    shell: Entity<Shell>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            div().children(
                shell
                    .areas
                    .settings
                    .agents
                    .addition
                    .form
                    .as_ref()
                    .map(|form| form.program.element(&Tokens::of(Scheme::Dark), false, cx)),
            )
        })
    }
}
fn form(
    shell: &mut Shell,
    name: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Rc<Intent> {
    shell.agent_open_add(None, window, cx);
    let form = shell.areas.settings.agents.addition.form.as_ref().unwrap();
    form.name.set_text(name, window, cx);
    form.program.set_text("fake-acp", window, cx);
    Rc::clone(&form.intent)
}

#[gpui::test]
fn a_stale_form_cannot_close_or_send_its_successor_and_sending_holds_real_input(
    cx: &mut TestAppContext,
) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (view, cx) = cx.add_window_view(move |window, cx| Probe {
        shell: cx.new(|cx| Shell::with_test_link(window, cx, link)),
    });
    cx.update(|window, _| window.activate_window());
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            testing::select(shell, "one", Vec::new(), cx);
            let old = form(shell, "discarded", window, cx);
            let current = form(shell, "current", window, cx);
            shell.agent_close_add(&old, window, cx);
            shell.agent_submit_add(&old, cx);
            assert!(commands.try_recv().is_err());
            assert!(Rc::ptr_eq(
                &shell
                    .areas
                    .settings
                    .agents
                    .addition
                    .form
                    .as_ref()
                    .unwrap()
                    .intent,
                &current
            ));
            shell.agent_submit_add(&current, cx);
            let Command::Agents {
                machine,
                request,
                ask: Ask::Add(addition),
            } = commands.try_recv().unwrap()
            else {
                panic!("one record-only Add");
            };
            assert_eq!(machine.as_str(), "one");
            let Operation::Add { call, .. } = Ask::Add(addition).operation() else {
                panic!("Add operation");
            };
            assert_eq!(call.name.as_str(), "current");
            let active = shell.areas.settings.agents.addition.form.as_ref().unwrap();
            assert!(matches!(active.phase, Phase::Sending(sent) if sent == request));
            active.program.focus(window, cx);
            shell.agent_submit_add(&current, cx);
            assert!(commands.try_recv().is_err());
        })
    });
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let active = shell.areas.settings.agents.addition.form.as_ref().unwrap();
            assert!(active.program.focus_handle(cx).is_focused(window));
        });
    });
    cx.simulate_input("must not change the submitted program");
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let active = shell.areas.settings.agents.addition.form.as_ref().unwrap();
            assert_eq!(active.program.text(cx), "fake-acp");
            let current = Rc::clone(&active.intent);
            let ticket = shell
                .areas
                .settings
                .agents
                .pages
                .get("one")
                .unwrap()
                .pending()
                .unwrap()
                .id;
            shell.agent_close_add(&current, window, cx);
            assert!(shell.areas.settings.agents.addition.form.is_none());
            assert_eq!(
                shell
                    .areas
                    .settings
                    .agents
                    .pages
                    .get("one")
                    .unwrap()
                    .pending()
                    .unwrap()
                    .id,
                ticket
            );
            assert!(commands.try_recv().is_err());
        })
    });
}

#[gpui::test]
fn an_old_machines_add_ack_keeps_the_new_form_and_sends_no_followup(cx: &mut TestAppContext) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        testing::select(shell, "one", Vec::new(), cx);
        let old = form(shell, "recorded", window, cx);
        shell.agent_submit_add(&old, cx);
        let Command::Agents {
            request,
            ask: Ask::Add(_),
            ..
        } = commands.try_recv().unwrap()
        else {
            panic!("record-only Add");
        };
        testing::select(shell, "two", Vec::new(), cx);
        let current = form(shell, "still-editing", window, cx);
        shell.agent_close_add(&old, window, cx);
        shell.agent_submit_add(&old, cx);
        shell.agents_answer(
            &MachineKey::from_book("one"),
            request,
            Ok(Answer::Added {
                node: testing::node(),
                info: testing::added("recorded"),
            }),
            cx,
        );
        let active = shell.areas.settings.agents.addition.form.as_ref().unwrap();
        assert!(Rc::ptr_eq(&active.intent, &current));
        assert_eq!(active.name.text(cx), "still-editing");
        assert!(matches!(active.phase, Phase::Editing));
        assert!(
            shell
                .areas
                .settings
                .agents
                .pages
                .get("one")
                .unwrap()
                .pending()
                .is_none()
        );
        assert!(
            commands.try_recv().is_err(),
            "ACK must not probe options, discover, install or reload"
        );
    });
}
