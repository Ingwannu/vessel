//! Test the production composition and actual kit buttons, not a second
//! row implementation. No app shell, credentials or node is started.
use super::*;
use gpui::{
    Bounds, Context, InteractiveElement as _, IntoElement, Pixels, Render, TestAppContext, Window,
};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::{badge, outline_button, theme::Scheme};

const ACTIONS: [&str; 4] = [
    "device-action-short",
    "device-action-short-badge",
    "device-action-long",
    "device-action-long-badge",
];
const BADGES: [&str; 4] = [
    "device-badge-unused-short",
    "device-badge-short",
    "device-badge-unused-long",
    "device-badge-long",
];

struct Probe {
    seen: Rc<RefCell<[Vec<Bounds<Pixels>>; 4]>>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let mut rows = div()
            .w(px(340.0))
            .flex()
            .flex_col()
            .debug_selector(|| "device-row-frame".to_owned());
        let long = "A personal tablet with a long name that must wrap while its revoke action keeps the same trailing edge";
        for (index, (title, unseen)) in [
            ("Phone", false),
            ("Phone", true),
            (long, false),
            (long, true),
        ]
        .into_iter()
        .enumerate()
        {
            let selector = ACTIONS[index];
            let badge_selector = BADGES[index];
            let action = outline_button(selector, "Revoke", true, &tokens)
                .debug_selector(move || selector.to_owned());
            let badge = unseen.then(|| {
                badge::badge(
                    "Not seen",
                    badge::Variant::Muted,
                    badge::Size::Small,
                    None,
                    &tokens,
                )
                .debug_selector(move || badge_selector.to_owned())
            });
            let seen = Rc::clone(&self.seen);
            rows = rows.child(
                compose(title, "Read only", badge, action, false, true, &tokens)
                    .on_children_prepainted(move |bounds, _window, _cx| {
                        if let Some(place) = seen.borrow_mut().get_mut(index) {
                            *place = bounds;
                        }
                    }),
            );
        }
        rows
    }
}

#[gpui::test]
fn revoke_keeps_its_trailing_edge_with_and_without_a_badge_and_wrapping_name(
    cx: &mut TestAppContext,
) {
    let seen = Rc::new(RefCell::new(std::array::from_fn(|_| Vec::new())));
    let (_, cx) = cx.add_window_view({
        let seen = Rc::clone(&seen);
        move |_window, _cx| Probe { seen }
    });
    cx.run_until_parked();
    let frame = cx
        .debug_bounds("device-row-frame")
        .expect("production row frame");
    let edge = spacing::S4.mul_add(-1.0, f32::from(frame.right()));
    let bounds = seen.borrow();
    for index in 0..4 {
        let action = cx
            .debug_bounds(ACTIONS[index])
            .expect("actual kit revoke button");
        assert!(f32::from(action.right()).mul_add(-1.0, edge).abs() <= 0.01);
        let row = bounds.get(index).expect("measured row");
        assert_eq!(row.len(), 2, "one text column and one trailing group");
        let text = row.first().expect("words");
        if index == 1 || index == 3 {
            let badge = cx
                .debug_bounds(BADGES[index])
                .expect("actual distinguishing badge");
            assert_eq!(badge.center().y, action.center().y);
            assert!(action.top() >= text.bottom(), "group wraps as one unit");
        } else {
            assert_eq!(text.center().y, action.center().y);
        }
    }
    assert!(bounds[2][0].size.height > bounds[0][0].size.height);
    assert!(bounds[3][0].size.height > bounds[1][0].size.height);
}
