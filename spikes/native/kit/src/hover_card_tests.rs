//! Paseo's hover card in a real window: open at once on the trigger, kept
//! while the pointer crosses onto the card, closed 100 ms after it leaves
//! both, closed by a press on the trigger and by Escape, never on a
//! trigger that is not enabled; never outliving its trigger (the app
//! reviewer's R1 on TRACKER 810).
use super::*;
use crate::{Scheme, Tokens};
use gpui::{Context, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext};

struct Probe {
    /// Whatever has the keys (the app always has a focused element).
    focus: gpui::FocusHandle,
    cards: HoverCards,
    a: Place,
    b: Place,
    enabled: bool,
    /// A is drawn (a row that vanished when not).
    draw_a: bool,
    /// Nothing is drawn, triggers or cards (a sidebar hidden).
    hidden: bool,
}

impl Probe {
    fn trigger(&self, id: &'static str, place: &Place) -> Stateful<gpui::Div> {
        let trigger = div()
            .id(id)
            .w(px(200.0))
            .h(px(36.0))
            .debug_selector(move || id.to_owned());
        self.cards.on(trigger, place, id, self.enabled)
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let content = div().h(px(80.0)).child("details");
        let root = div()
            .track_focus(&self.focus)
            .size_full()
            .p(px(40.0))
            .flex()
            .flex_col()
            .gap(px(40.0));
        if self.hidden {
            return root;
        }
        // A's room stays when it is not drawn, so B does not slide under
        // the pointer.
        let a = if self.draw_a {
            self.trigger("A", &self.a).into_any_element()
        } else {
            div().w(px(200.0)).h(px(36.0)).into_any_element()
        };
        root.child(a)
            .child(self.trigger("B", &self.b))
            .children(
                self.cards
                    .render(content, "Session details", 260.0, &tokens, window),
            )
    }
}

fn setup(cx: &mut TestAppContext, enabled: bool) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(move |_, cx| Probe {
        focus: cx.focus_handle(),
        cards: HoverCards::new(),
        a: Place::default(),
        b: Place::default(),
        enabled,
        draw_a: true,
        hidden: false,
    });
    view.update_in(cx, |probe, window, cx| probe.focus.focus(window, cx));
    (view, cx)
}

fn move_to(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
}

fn advance(cx: &VisualTestContext, duration: Duration) {
    cx.executor().advance_clock(duration);
    cx.run_until_parked();
}

fn shown(view: &gpui::Entity<Probe>, cx: &mut VisualTestContext) -> Option<String> {
    view.read_with(cx, |probe, _| {
        probe.cards.shown().map(|key| key.to_string())
    })
}

#[gpui::test]
fn it_opens_at_once_and_stays_while_the_pointer_crosses_onto_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let a = cx.debug_bounds("A").unwrap();
    move_to(cx, a.center());
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    // Right of the trigger (40 to 240), 4 away, centred on its height.
    let card = cx.debug_bounds("hover-card").unwrap();
    assert_eq!(a.right(), px(240.0));
    assert_eq!(card.left(), px(244.0));
    assert_eq!(card.center().y, a.center().y);
    assert_eq!(card.size.width, px(260.0));
    // Across the gap onto the card: kept past the grace.
    move_to(cx, point(px(242.0), a.center().y));
    move_to(cx, card.center());
    advance(cx, Duration::from_millis(300));
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    // Off both: gone after the grace, not before.
    move_to(cx, point(px(5.0), px(5.0)));
    advance(cx, Duration::from_millis(99));
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    advance(cx, Duration::from_millis(1));
    assert_eq!(shown(&view, cx), None);
}

#[gpui::test]
fn another_trigger_takes_it_and_a_press_or_escape_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let a = cx.debug_bounds("A").unwrap();
    let b = cx.debug_bounds("B").unwrap();
    move_to(cx, a.center());
    move_to(cx, b.center());
    advance(cx, Duration::from_millis(200));
    assert_eq!(shown(&view, cx).as_deref(), Some("B"));
    cx.simulate_mouse_down(b.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(shown(&view, cx), None);
    cx.simulate_mouse_up(b.center(), MouseButton::Left, Modifiers::none());
    move_to(cx, a.center());
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(shown(&view, cx), None);
}

#[gpui::test]
fn a_trigger_not_enabled_shows_nothing(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let a = cx.debug_bounds("A").unwrap();
    move_to(cx, a.center());
    assert_eq!(shown(&view, cx), None);
    assert!(cx.debug_bounds("hover-card").is_none());
}

#[gpui::test]
fn a_card_whose_trigger_is_no_longer_drawn_closes(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let a = cx.debug_bounds("A").unwrap();
    move_to(cx, a.center());
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    // A vanishes under the pointer (its machine went offline): no
    // hover-leave reaches it, and the card goes with it.
    view.update(cx, |probe, cx| {
        probe.draw_a = false;
        cx.notify();
    });
    cx.run_until_parked();
    move_to(cx, point(px(600.0), px(500.0)));
    assert_eq!(shown(&view, cx), None);
    assert!(cx.debug_bounds("hover-card").is_none());
}

#[gpui::test]
fn a_card_does_not_come_back_with_its_trigger_under_a_pointer_elsewhere(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let a = cx.debug_bounds("A").unwrap();
    move_to(cx, a.center());
    assert_eq!(shown(&view, cx).as_deref(), Some("A"));
    // The sidebar is hidden by a key, the pointer moves into the
    // conversation, the sidebar comes back: no card beside A.
    view.update(cx, |probe, cx| {
        probe.hidden = true;
        cx.notify();
    });
    cx.run_until_parked();
    move_to(cx, point(px(600.0), px(500.0)));
    view.update(cx, |probe, cx| {
        probe.hidden = false;
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(shown(&view, cx), None);
    assert!(cx.debug_bounds("hover-card").is_none());
}
