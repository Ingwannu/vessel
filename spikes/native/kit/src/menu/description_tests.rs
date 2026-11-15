//! The existing row frame owns hints; feedback never leaves a stale hint below it.
use super::{ItemStatus, MenuItem, item, row};
use crate::{Scheme, Tokens, popup::Surfaces};
use gpui::prelude::*;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, div, px};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
enum Feedback {
    Idle,
    Pending,
    Success,
}
struct Probe {
    scheme: Scheme,
    narrow: bool,
    description: Option<&'static str>,
    feedback: Feedback,
    seen: Rc<Cell<Option<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let surfaces = Surfaces::of(&tokens);
        let mut item = MenuItem::new("Option");
        if let Some(description) = self.description {
            item = item.with_description(description);
        }
        item = match self.feedback {
            Feedback::Idle => item,
            Feedback::Pending => item.pending("Working"),
            Feedback::Success => item.success("Done"),
        };
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| seen.set(bounds.first().copied()))
            .child(
                row::frame(
                    0,
                    &item,
                    item::paint(&item, false, &surfaces),
                    self.narrow,
                    &surfaces,
                    row::Content::default(),
                )
                .id("description-row"),
            )
    }
}
#[test]
fn menu_description_preserves_the_whole_caption_but_feedback_hides_it() {
    let item = MenuItem::new("Choice").with_description("main...feature");
    assert_eq!(
        item.visible_description().unwrap().as_ref(),
        "main...feature"
    );
    assert!(
        item.clone()
            .pending("Working")
            .visible_description()
            .is_none()
    );
    assert!(item.clone().success("Done").visible_description().is_none());
    assert!(
        MenuItem::new("Choice")
            .with_description("")
            .visible_description()
            .is_none()
    );
    assert_eq!(item.status, ItemStatus::Idle);
}
#[gpui::test]
fn menu_description_is_capped_to_two_lines_and_feedback_retains_legacy_row_height(
    cx: &mut TestAppContext,
) {
    for scheme in [Scheme::Dark, Scheme::Light] {
        for narrow in [false, true] {
            for (description, pointer_height, touch_height) in [
                (None, 28.0, 44.0),
                (Some(""), 28.0, 44.0),
                (Some("Details"), 46.0, 46.0),
                (Some("One\nTwo\nThree"), 62.0, 62.0),
            ] {
                for feedback in [Feedback::Idle, Feedback::Pending, Feedback::Success] {
                    let seen = Rc::new(Cell::new(None));
                    let (_, cx) = cx.add_window_view({
                        let seen = Rc::clone(&seen);
                        move |_, _| Probe {
                            scheme,
                            narrow,
                            description,
                            feedback,
                            seen,
                        }
                    });
                    cx.run_until_parked();
                    let expected = match (feedback, narrow) {
                        (Feedback::Idle, false) => pointer_height,
                        (Feedback::Idle, true) => touch_height,
                        (Feedback::Pending | Feedback::Success, false) => 28.0,
                        (Feedback::Pending | Feedback::Success, true) => 44.0,
                    };
                    let bounds = seen.get().expect("actual same row frame");
                    assert_eq!(bounds.size.height, px(expected));
                    assert_eq!(bounds.origin.x, px(4.0));
                    assert_eq!(bounds.size.width, px(312.0));
                }
            }
        }
    }
}
