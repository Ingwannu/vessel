//! Real click dispatch across idle/disabled/pending and pending-to-idle frames.
//! A consumer deliberately supplies every callback; the shared guard refuses
//! disabled/pending handlers rather than trusting only its grey appearance.
use super::*;
use gpui::{Context, Modifiers, Render, TestAppContext, Window, point};
struct Probe {
    scheme: crate::Scheme,
    pending: bool,
    ready: usize,
    disabled: usize,
    loading: usize,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let ready = ButtonConfig::new(
            ButtonKind::Ghost,
            ButtonState::Enabled,
            ControlSizing::Pointer,
        );
        let disabled = ButtonConfig::new(
            ButtonKind::Ghost,
            ButtonState::Disabled,
            ControlSizing::Pointer,
        );
        let pending = ButtonConfig::new(
            ButtonKind::Outline,
            ButtonState::Enabled,
            ControlSizing::Pointer,
        );
        let pending = if self.pending {
            pending.loading()
        } else {
            pending
        };
        div()
            .flex()
            .flex_col()
            .items_start()
            .size_full()
            .child(
                button_control(
                    "ready",
                    ButtonContent::Label("Action".into()),
                    ready,
                    &tokens,
                )
                .on_click(cx.listener(|this, _, _, _| this.ready = this.ready.saturating_add(1))),
            )
            .child(
                button_control(
                    "disabled",
                    ButtonContent::Label("Disabled".into()),
                    disabled,
                    &tokens,
                )
                .on_click(
                    cx.listener(|this, _, _, _| this.disabled = this.disabled.saturating_add(1)),
                ),
            )
            .child(
                button_control(
                    "loading",
                    ButtonContent::Label("Adding...".into()),
                    pending,
                    &tokens,
                )
                .on_click(
                    cx.listener(|this, _, _, _| this.loading = this.loading.saturating_add(1)),
                ),
            )
    }
}
#[gpui::test]
fn disabled_and_loading_callbacks_cannot_fire_and_completion_reenables(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
        let (view, cx) = cx.add_window_view(move |_, _| Probe {
            scheme,
            pending: true,
            ready: 0,
            disabled: 0,
            loading: 0,
        });
        cx.run_until_parked();
        for y in [16.0, 48.0, 80.0] {
            cx.simulate_click(point(px(20.0), px(y)), Modifiers::none());
            cx.run_until_parked();
        }
        view.read_with(cx, |this, _| {
            assert_eq!((this.ready, this.disabled, this.loading), (1, 0, 0));
        });
        view.update(cx, |this, cx| {
            this.pending = false;
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_click(point(px(20.0), px(80.0)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert_eq!((this.ready, this.disabled, this.loading), (1, 0, 1));
        });
    }
}
