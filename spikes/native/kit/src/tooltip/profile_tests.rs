//! Deterministic source profiles and real hover/press cancellation in one controller.
use super::*;
use crate::{ControlSizing, Scheme, Tokens};
use gpui::{Context, Modifiers, Render, TestAppContext, point, size};
use std::time::Duration;

#[test]
fn default_and_toolbar_place_the_same_card_from_exact_trigger_edges() {
    let bounds = Bounds {
        origin: point(px(40.0), px(60.0)),
        size: size(px(20.0), px(20.0)),
    };
    assert_eq!(Configuration::default().delay(), Duration::ZERO);
    assert_eq!(Configuration::toolbar().delay(), Duration::from_millis(300));
    assert_eq!(
        Configuration::default().placement(bounds),
        (
            point(px(50.0), px(60.0)),
            gpui::Anchor::BottomCenter,
            point(px(0.0), px(-8.0)),
        )
    );
    assert_eq!(
        Configuration::toolbar().placement(bounds),
        (
            point(px(50.0), px(80.0)),
            gpui::Anchor::TopCenter,
            point(px(0.0), px(8.0)),
        )
    );
    assert_eq!(
        Configuration::toolbar().side(Side::Top).placement(bounds),
        Configuration::default().placement(bounds)
    );
    for points in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(Offset::try_from(px(points)), Err(InvalidOffset::NotFinite));
    }
    assert_eq!(Offset::try_from(px(-1.0)), Err(InvalidOffset::Negative));
    assert!(Offset::try_from(px(0.0)).is_ok());
}
#[derive(Clone, Copy)]
enum Profile {
    Legacy,
    Toolbar,
}
struct Probe {
    tips: Tooltips,
    a: Place,
    b: Place,
    profile: Profile,
    sizing: ControlSizing,
    visible: Visibility,
    words: Words,
}
#[derive(Clone, Copy)]
enum Words {
    Initial,
    Updated,
}
#[derive(Clone, Copy)]
enum Visibility {
    Shown,
    Hidden,
}
impl Probe {
    fn button(&self, id: &'static str, place: &Place) -> Stateful<gpui::Div> {
        let button = div()
            .id(id)
            .size(px(20.0))
            .debug_selector(move || id.to_owned());
        let text = match self.words {
            Words::Initial => id.to_owned(),
            Words::Updated => format!("updated {id}"),
        };
        let configuration = match self.words {
            Words::Initial => Configuration::toolbar(),
            Words::Updated => updated_configuration(),
        };
        match self.profile {
            Profile::Legacy => {
                self.tips
                    .on(button, place, text, self.sizing == ControlSizing::Touch)
            }
            Profile::Toolbar => {
                self.tips
                    .on_configured(button, place, text, configuration, self.sizing)
            }
        }
    }
    fn shown(&self) -> Option<String> {
        self.tips
            .state
            .borrow()
            .shown
            .as_ref()
            .map(|tip| tip.content.text().to_string())
    }
}
impl Render for Probe {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div()
            .size_full()
            .p(px(80.0))
            .flex()
            .gap(px(40.0))
            .when(matches!(self.visible, Visibility::Shown), |root| {
                root.child(self.button("A", &self.a))
                    .child(self.button("B", &self.b))
            })
            .children(self.tips.render(&tokens))
    }
}
fn setup(
    cx: &mut TestAppContext,
    profile: Profile,
    sizing: ControlSizing,
) -> (gpui::Entity<Probe>, &mut gpui::VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    cx.add_window_view(move |_, _| Probe {
        tips: Tooltips::new(),
        a: Place::default(),
        b: Place::default(),
        profile,
        sizing,
        visible: Visibility::Shown,
        words: Words::Initial,
    })
}
fn hover(cx: &mut gpui::VisualTestContext, id: &'static str) {
    let position = cx.debug_bounds(id).unwrap().center();
    cx.simulate_mouse_move(position, None, Modifiers::none());
    cx.run_until_parked();
}
fn advance(cx: &gpui::VisualTestContext, duration: Duration) {
    cx.executor().advance_clock(duration);
    cx.run_until_parked();
}
#[gpui::test]
fn existing_legacy_hover_is_immediate_and_toolbar_waits_exactly_its_delay(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, Profile::Legacy, ControlSizing::Pointer);
    hover(cx, "A");
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("A"));
    });
    view.update(cx, |probe, cx| {
        probe.profile = Profile::Toolbar;
        probe.tips = Tooltips::new();
        cx.notify();
    });
    cx.run_until_parked();
    hover(cx, "B");
    advance(cx, Duration::from_millis(299));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
    advance(cx, Duration::from_millis(1));
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("B"));
    });
}
#[gpui::test]
fn pending_hover_aba_and_press_cannot_publish_old_or_cancelled_tips(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, Profile::Toolbar, ControlSizing::Pointer);
    hover(cx, "A");
    advance(cx, Duration::from_millis(100));
    hover(cx, "B");
    advance(cx, Duration::from_millis(100));
    hover(cx, "A");
    advance(cx, Duration::from_millis(299));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
    advance(cx, Duration::from_millis(1));
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("A"));
    });
    let position = cx.debug_bounds("A").unwrap().center();
    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    advance(cx, Duration::from_millis(300));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
}
#[gpui::test]
fn hidden_trigger_and_replaced_controller_do_not_deliver_pending_hover(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, Profile::Toolbar, ControlSizing::Pointer);
    hover(cx, "A");
    view.update(cx, |probe, cx| {
        probe.visible = Visibility::Hidden;
        cx.notify();
    });
    cx.run_until_parked();
    advance(cx, Duration::from_millis(300));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
    let old = view.read_with(cx, |probe, _| Rc::downgrade(&probe.tips.state));
    view.update(cx, |probe, cx| {
        probe.tips = Tooltips::new();
        cx.notify();
    });
    cx.run_until_parked();
    assert!(
        old.upgrade().is_none(),
        "no timer/controller retention cycle"
    );
}
#[gpui::test]
fn compact_touch_profile_installs_no_tooltip_or_timer(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, Profile::Toolbar, ControlSizing::Touch);
    hover(cx, "A");
    advance(cx, Duration::from_millis(300));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
}

fn updated_configuration() -> Configuration {
    Configuration::new(
        Side::Top,
        Offset::EIGHT,
        Delay::new(Duration::from_millis(1000)),
    )
}
#[gpui::test]
fn stationary_pending_hover_updates_words_and_side_without_restarting_original_timer(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, Profile::Toolbar, ControlSizing::Pointer);
    hover(cx, "A");
    advance(cx, Duration::from_millis(100));
    view.update(cx, |probe, cx| {
        probe.words = Words::Updated;
        cx.notify();
    });
    cx.run_until_parked();
    advance(cx, Duration::from_millis(199));
    view.read_with(cx, |probe, _| assert!(probe.shown().is_none()));
    advance(cx, Duration::from_millis(1));
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("updated A"));
        assert_eq!(
            probe
                .tips
                .state
                .borrow()
                .shown
                .as_ref()
                .unwrap()
                .configuration,
            updated_configuration()
        );
    });
}
#[gpui::test]
fn stationary_shown_hover_updates_current_words_and_side_in_the_same_render(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, Profile::Toolbar, ControlSizing::Pointer);
    hover(cx, "A");
    advance(cx, Duration::from_millis(300));
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("A"));
    });
    view.update(cx, |probe, cx| {
        probe.words = Words::Updated;
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        assert_eq!(probe.shown().as_deref(), Some("updated A"));
        assert_eq!(
            probe
                .tips
                .state
                .borrow()
                .shown
                .as_ref()
                .unwrap()
                .configuration,
            updated_configuration()
        );
    });
}
