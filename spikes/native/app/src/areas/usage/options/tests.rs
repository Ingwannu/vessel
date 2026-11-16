//! Actual cog clicks and retained intents over one isolated Shell/public snapshot.
//! No Link, authenticated transport, quota payload or external lookup is fabricated.
use super::*;
use gpui::{AppContext as _, Entity, Modifiers, Render, Subscription, TestAppContext};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};

struct Probe {
    shell: Entity<Shell>,
    _changed: Subscription,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
            div()
                .size_full()
                .flex()
                .items_start()
                .children(trigger(shell, "usage-options-cog", &tokens, false, cx))
                .children(dialogs(shell, &tokens, window, cx))
        })
    }
}

#[gpui::test]
fn usage_cog_toggles_and_retained_intents_cannot_close_or_replace_a_new_opening(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(
            MachineKey::from_book("fixture"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let selected = context.clone();
    let (probe, cx) = cx.add_window_view(move |window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let mut machine =
                vessel_app_view::Machine::unreachable("fixture".into(), "session read refused");
            machine.node = Some(selected.node());
            machine.connection_epoch = Some(selected.epoch());
            shell.fleet.machines = vec![machine];
            shell.areas.usage.connection = Some(selected.clone());
            shell
        });
        let changed = cx.observe(&shell, |_, _, cx| cx.notify());
        Probe {
            shell,
            _changed: changed,
        }
    });
    cx.run_until_parked();
    let shell = probe.read_with(cx, |probe, _| probe.shell.clone());
    let at = shell.read_with(cx, |shell, _| {
        shell.areas.usage.options_anchor.get().unwrap().center()
    });
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let first = shell.read_with(cx, |shell, _| {
        shell.areas.usage.options_open.clone().unwrap()
    });
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.usage.options_open.is_none());
    });
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    let second = shell.read_with(cx, |shell, _| {
        shell.areas.usage.options_open.clone().unwrap()
    });
    shell.update_in(cx, |shell, window, cx| {
        // Replay the exact arguments captured by earlier closed/open frames.
        shell.usage_toggle_options(context.clone(), None, None, window, cx);
        shell.usage_toggle_options(context, None, Some(first.clone()), window, cx);
        assert!(shell.areas.usage.options.is_current(&second.lease));
        assert!(!shell.areas.usage.options.is_current(&first.lease));
        assert!(
            shell
                .areas
                .usage
                .options_open
                .as_ref()
                .unwrap()
                .current(shell)
        );
    });
    cx.run_until_parked();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.usage.options_open.is_none());
        assert!(!shell.areas.usage.options.is_current(&second.lease));
    });
}
