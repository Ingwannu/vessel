//! Real pointer input through the shared info widget, including its hit slop.
use super::info::Alignment;
use super::*;
use crate::picker::anchor_probe;
use crate::{Scheme, screen_header::Density};
use gpui::{Context, Modifiers, Render, TestAppContext, Window, point};

#[derive(Clone, Copy)]
enum Mode {
    LegacyTouch,
    Desktop,
    Compact,
}
struct Probe {
    mode: Mode,
    tips: Tooltips,
    place: Place,
    toolbar: Place,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let control = match self.mode {
            Mode::LegacyTouch => self
                .tips
                .on(
                    div()
                        .id("legacy-touch")
                        .size(px(22.0))
                        .child(anchor_probe(&self.place)),
                    &self.place,
                    "Header explanation",
                    true,
                )
                .into_any_element(),
            Mode::Desktop | Mode::Compact => crate::settings::info_tip(
                "sidebar-info",
                "Header explanation".into(),
                &self.tips,
                &self.place,
                match self.mode {
                    Mode::Compact => Density::Compact,
                    Mode::LegacyTouch | Mode::Desktop => Density::Desktop,
                },
                &tokens,
            )
            .into_any_element(),
        };
        let toolbar = self.tips.on_configured(
            div()
                .id("delayed-toolbar")
                .absolute()
                .left_0()
                .top(px(320.0))
                .size(px(20.0)),
            &self.toolbar,
            "Toolbar delayed",
            Configuration::toolbar(),
            crate::ControlSizing::Pointer,
        );
        div()
            .size_full()
            .child(
                div()
                    .w(px(360.0))
                    .mt(px(80.0))
                    .child(crate::settings::section_with_info(
                        "Header",
                        div().h(px(80.0)),
                        Some(control),
                        &tokens,
                    )),
            )
            .child(toolbar)
            .children(self.tips.render(&tokens))
    }
}
fn probe(
    cx: &mut TestAppContext,
    mode: Mode,
) -> (gpui::Entity<Probe>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        mode,
        tips: Tooltips::new(),
        place: Rc::default(),
        toolbar: Rc::default(),
    });
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn settings_press_and_clear_cancel_the_same_configured_hover_timer(cx: &mut TestAppContext) {
    use std::time::Duration;
    let (view, cx) = probe(cx, Mode::Compact);
    let (toolbar, info) = view.read_with(cx, |probe, _| {
        (
            probe.toolbar.get().unwrap().center(),
            probe.place.get().unwrap().center(),
        )
    });
    cx.simulate_mouse_move(toolbar, None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.simulate_click(info, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        let state = probe.tips.state.borrow();
        assert_eq!(
            state.shown.as_ref().unwrap().content.text().as_ref(),
            "Header explanation"
        );
    });
    cx.simulate_mouse_move(toolbar, None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(100));
    view.update(cx, |probe, cx| {
        probe.tips.clear();
        cx.notify();
    });
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        assert!(probe.tips.state.borrow().shown.is_none());
    });
}
#[gpui::test]
fn sidebar_info_compact_press_uses_expanded_hit_area_and_dismisses_outside(
    cx: &mut TestAppContext,
) {
    let (view, cx) = probe(cx, Mode::Compact);
    let bounds = view.read_with(cx, |probe, _| probe.place.get().unwrap());
    assert_eq!(bounds.size.width, px(22.0));
    assert_eq!(bounds.size.height, px(22.0));
    let hit = point(
        px(f32::from(bounds.left()).mul_add(1.0, -4.0)),
        bounds.center().y,
    );
    cx.simulate_click(hit, Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        let state = probe.tips.state.borrow();
        let tip = state.shown.as_ref().unwrap();
        assert!(matches!(tip.options.alignment, Alignment::Start));
        assert_eq!(tip.options.interaction, InfoInteraction::Press);
    });
    cx.simulate_click(point(px(340.0), px(300.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        assert!(probe.tips.state.borrow().shown.is_none());
    });
}
#[gpui::test]
fn sidebar_info_desktop_hover_and_legacy_touch_keep_distinct_policies(cx: &mut TestAppContext) {
    for mode in [Mode::Desktop, Mode::LegacyTouch] {
        let (view, cx) = probe(cx, mode);
        let at = view.read_with(cx, |probe, _| probe.place.get().unwrap().center());
        cx.simulate_mouse_move(at, None, Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert_eq!(
                probe.tips.state.borrow().shown.is_some(),
                matches!(mode, Mode::Desktop)
            );
        });
        cx.simulate_click(at, Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert!(probe.tips.state.borrow().shown.is_none());
        });
    }
}
