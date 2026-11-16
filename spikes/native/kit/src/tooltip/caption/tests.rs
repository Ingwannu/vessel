//! Native body layout and same-Place current props, using the existing timer.
use super::*;
use crate::{Scheme, fonts::Type, widgets::TypeExt as _};
use gpui::{Bounds, Context, Modifiers, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[test]
fn empty_display_chords_are_absent_and_caption_metrics_use_ui_not_code_scale() {
    assert!(DisplayChords::new(["".into()]).is_empty());
    let keys = DisplayChords::new(["Ctrl+K".into(), "".into(), "Ctrl+U".into()]);
    assert_eq!(
        keys.as_slice(),
        &[SharedString::from("Ctrl+K"), SharedString::from("Ctrl+U")]
    );
    let caption = Content::Caption(KeyCaption::new("Settings", keys));
    let text = Content::Text("Settings".into());
    let interface = |value: f32| value.mul_add(2.0, 0.0);
    let code = |value: f32| value.mul_add(3.0, 0.0);
    assert_eq!(
        caption.metrics_with(interface, code),
        text.metrics_with(interface, code)
    );
    assert_eq!(caption.text().as_ref(), "Settings");
}

#[derive(Clone, Copy)]
enum Layout {
    Caption,
    Chords,
}
struct Measure {
    caption: KeyCaption,
    scheme: Scheme,
    layout: Layout,
    bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Measure {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let contents = match self.layout {
            Layout::Caption => body(&self.caption, &tokens).max_w(px(180.0)),
            Layout::Chords => chord_badges(&self.caption.keys, &tokens).w(px(74.0)),
        };
        let bounds = Rc::clone(&self.bounds);
        div()
            .size_full()
            .flex()
            .items_start()
            .type_(Type::Body)
            .child(contents.on_children_prepainted(move |seen, _, _| {
                *bounds.borrow_mut() = seen.to_vec();
            }))
    }
}
#[gpui::test]
fn empty_keys_reserve_no_native_child_and_bounded_sequences_wrap(cx: &mut TestAppContext) {
    for scheme in [Scheme::Light, Scheme::Dark] {
        for label in ["Settings", "설정"] {
            for keys in [
                DisplayChords::default(),
                DisplayChords::new(["Ctrl+,".into()]),
            ] {
                let expected = if keys.is_empty() { 1 } else { 2 };
                let (view, cx) = cx.add_window_view(move |_, _| Measure {
                    caption: KeyCaption::new(label, keys),
                    scheme,
                    layout: Layout::Caption,
                    bounds: Rc::default(),
                });
                cx.run_until_parked();
                view.read_with(cx, |view, _| {
                    let bounds = view.bounds.borrow();
                    assert_eq!(bounds.len(), expected);
                    if expected == 2 {
                        let gap = bounds[0]
                            .right()
                            .as_f32()
                            .mul_add(-1.0, bounds[1].left().as_f32());
                        assert!(gap.mul_add(-1.0, 8.0).abs() < 0.001);
                    }
                });
            }
        }
        let (view, cx) = cx.add_window_view(move |_, _| Measure {
            caption: KeyCaption::new(
                "",
                DisplayChords::new(["Ctrl+K".into(), "Ctrl+U".into(), "Ctrl+C".into()]),
            ),
            scheme,
            layout: Layout::Chords,
            bounds: Rc::default(),
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            let bounds = view.bounds.borrow();
            assert_eq!(bounds.len(), 3);
            assert!(bounds[2].top() > bounds[0].top());
            assert!(bounds.iter().all(|bound| bound.right() <= px(74.001)));
        });
    }
}

#[derive(Clone, Copy)]
enum Words {
    Initial,
    Updated,
}
struct Probe {
    tips: Tooltips,
    place: Place,
    words: Words,
}
impl Probe {
    fn caption(&self) -> KeyCaption {
        match self.words {
            Words::Initial => KeyCaption::new("Settings", DisplayChords::new(["Ctrl+,".into()])),
            Words::Updated => KeyCaption::new(
                "설정",
                DisplayChords::new(["Ctrl+K".into(), "Ctrl+U".into()]),
            ),
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.tips.on_caption(
                div().id("caption-trigger").mt(px(160.0)).size(px(20.0)),
                &self.place,
                self.caption(),
                Configuration::toolbar().side(super::super::Side::Top),
                ControlSizing::Pointer,
            ))
            .children(self.tips.render(&Tokens::of(Scheme::Dark)))
    }
}
fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Probe>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|_, _| Probe {
        tips: Tooltips::new(),
        place: Rc::default(),
        words: Words::Initial,
    });
    cx.run_until_parked();
    let at = view.read_with(cx, |probe, _| probe.place.get().unwrap().center());
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
    (view, cx)
}
fn current(view: &Probe) -> Option<KeyCaption> {
    let state = view.tips.state.borrow();
    match state.shown.as_ref() {
        Some(tip) => {
            assert!(tip.trigger.upgrade().is_some());
            match &tip.content {
                Content::Caption(caption) => Some(caption.clone()),
                Content::Text(_) | Content::Code(_) => panic!("typed caption"),
            }
        }
        None => None,
    }
}
fn updated() -> KeyCaption {
    KeyCaption::new(
        "설정",
        DisplayChords::new(["Ctrl+K".into(), "Ctrl+U".into()]),
    )
}
#[gpui::test]
fn stationary_pending_caption_and_keys_refresh_without_restarting_the_deadline(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    cx.executor().advance_clock(Duration::from_millis(100));
    view.update(cx, |view, cx| {
        view.words = Words::Updated;
        cx.notify();
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(199));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(current(view).is_none());
    });
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(current(view), Some(updated()));
    });
}
#[gpui::test]
fn stationary_shown_caption_and_keys_refresh_in_the_same_controller(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(current(view).is_some());
    });
    view.update(cx, |view, cx| {
        view.words = Words::Updated;
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(current(view), Some(updated()));
    });
}
