//! Real rendered dismissal paths plus the open-cycle ownership edge cases.
use super::*;
use crate::dialog::{DialogContent, DialogSize, dialog};
use crate::{Scheme, Tokens, outline_button};
use gpui::{
    Context, FocusHandle, InputEvent as _, InteractiveElement as _, IntoElement as _, Modifiers,
    MouseButton, MouseDownEvent, MouseUpEvent, ParentElement as _, Point, Render,
    StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, div, point,
    px,
};

struct Probe {
    state: DialogState,
    opener: Option<FocusHandle>,
    newer: FocusHandle,
    open: bool,
    pending: bool,
    decision: u8,
}
impl Probe {
    fn close(&mut self, expected: u8, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.pending || self.decision != expected {
            return;
        }
        self.open = false;
        self.state.dismissed(window, cx);
        cx.notify();
    }
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let mut root = div().size_full().child(div().track_focus(&self.newer));
        if let Some(opener) = &self.opener {
            root = root.child(div().track_focus(opener));
        }
        if self.open {
            let expected = self.decision;
            let tokens = Tokens::of(Scheme::Dark);
            root = root.child(dialog(
                &self.state,
                DialogContent {
                    title: "A decision".into(),
                    body: div().child("Choose an action.").into_any_element(),
                    footer: vec![
                        outline_button("return-focus-action", "Close", true, &tokens)
                            .debug_selector(|| "dialog-return-action".to_owned())
                            .on_click(cx.listener(move |probe, _, window, cx| {
                                probe.close(expected, window, cx);
                            }))
                            .into_any_element(),
                    ],
                    size: DialogSize::Normal,
                },
                &tokens,
                window,
                false,
                move |probe, window, cx| probe.close(expected, window, cx),
                cx,
            ));
        }
        root
    }
}
fn mount(cx: &mut TestAppContext) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    cx.add_window_view(|window, cx| {
        let opener = cx.focus_handle();
        opener.focus(window, cx);
        let state = DialogState::new(cx);
        state.focus(window, cx);
        Probe {
            state,
            opener: Some(opener),
            newer: cx.focus_handle(),
            open: true,
            pending: false,
            decision: 1,
        }
    })
}
fn click(cx: &mut VisualTestContext, at: Point<gpui::Pixels>) {
    cx.update(|window, cx| {
        let _down = window.dispatch_event(
            MouseDownEvent {
                position: at,
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        let _up = window.dispatch_event(
            MouseUpEvent {
                position: at,
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    });
}
#[derive(Debug, Clone, Copy)]
enum Path {
    Escape,
    Scrim,
    Glyph,
    Action,
}
fn activate(path: Path, cx: &mut VisualTestContext) {
    match path {
        Path::Escape => cx.simulate_keystrokes("escape"),
        Path::Scrim => click(cx, point(px(0.0), px(0.0))),
        Path::Glyph => {
            let at = cx.debug_bounds("dialog-close").unwrap().center();
            click(cx, at);
        }
        Path::Action => {
            let at = cx.debug_bounds("dialog-return-action").unwrap().center();
            click(cx, at);
        }
    }
}
#[gpui::test]
fn every_actual_close_returns_to_the_opener_and_refused_closes_keep_the_dialog(
    cx: &mut TestAppContext,
) {
    for path in [Path::Escape, Path::Scrim, Path::Glyph, Path::Action] {
        let (probe, cx) = mount(cx);
        cx.run_until_parked();
        probe.update_in(cx, |probe, _, _| probe.pending = true);
        activate(path, cx);
        probe.update_in(cx, |probe, window, _cx| {
            assert!(probe.open);
            assert!(probe.state.focus.is_focused(window));
        });
        // A stale callback from the already rendered decision must also refuse.
        probe.update_in(cx, |probe, _, _| {
            probe.pending = false;
            probe.decision = 2;
        });
        activate(path, cx);
        probe.read_with(cx, |probe, _| {
            assert!(probe.open, "{path:?} cannot close the newer decision");
        });
        probe.update_in(cx, |_probe, _window, cx| cx.notify());
        cx.run_until_parked();
        activate(path, cx);
        probe.update_in(cx, |probe, window, _| {
            assert!(!probe.open, "{path:?}");
            assert!(
                probe.opener.as_ref().unwrap().is_focused(window),
                "{path:?}"
            );
        });
    }
}
#[gpui::test]
fn actual_close_preserves_newer_focus_and_reopen_captures_a_fresh_cycle(cx: &mut TestAppContext) {
    let (probe, cx) = mount(cx);
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.newer.focus(window, cx);
        probe.state.dismissed(window, cx);
        assert!(probe.newer.is_focused(window));
        probe.state.focus(window, cx);
        assert!(probe.state.focus.is_focused(window));
        probe.state.dismissed(window, cx);
        assert!(
            probe.newer.is_focused(window),
            "the skipped restore still consumed its old cycle"
        );
    });
}
#[gpui::test]
fn a_deleted_opener_is_not_retained_and_no_opener_is_captured_only_once(cx: &mut TestAppContext) {
    let (probe, cx) = mount(cx);
    cx.run_until_parked();
    let weak = probe.update_in(cx, |probe, _, cx| {
        let weak = probe.opener.take().unwrap().downgrade();
        cx.notify();
        weak
    });
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    probe.update_in(cx, |probe, window, cx| {
        probe.state.dismissed(window, cx);
        assert!(
            window.focused(cx).is_none(),
            "dead opener clears the dialog's focus"
        );
        probe.state.focus(window, cx); // No opener on this new cycle.
        probe.newer.focus(window, cx);
        probe.state.focus(window, cx); // Must not overwrite captured absence.
        probe.state.dismissed(window, cx);
        assert!(window.focused(cx).is_none());
    });
}

#[gpui::test]
fn the_dialog_itself_is_never_restored_as_its_own_opener(cx: &mut TestAppContext) {
    let (probe, cx) = mount(cx);
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.state.dismissed(window, cx);
        // Model focus already taken by the rendered owner before a late open
        // hook. The hook must not turn that into a hidden-owner focus loop.
        probe.state.focus.focus(window, cx);
        probe.state.focus(window, cx);
        probe.state.dismissed(window, cx);
        assert!(window.focused(cx).is_none());
    });
}
