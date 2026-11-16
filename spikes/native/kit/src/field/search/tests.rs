//! Actual one-line engine edits and literal source chrome, without OS claims.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Bounds, Modifiers, Pixels, Render, TestAppContext};
use std::{cell::RefCell, rc::Rc};
#[test]
fn source_geometry_and_focus_palette_are_literal_in_both_themes() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        for focused in [false, true] {
            let mut field = frame(focused, &tokens);
            let style = field.style();
            assert_eq!(style.max_size.width, Some(px(420.0).into()));
            assert_eq!(style.padding.left, Some(px(12.0).into()));
            assert_eq!(style.padding.right, Some(px(12.0).into()));
            assert_eq!(style.padding.top, Some(px(6.0).into()));
            assert_eq!(style.padding.bottom, Some(px(6.0).into()));
            assert_eq!(style.gap.width, Some(px(8.0).into()));
            assert_eq!(style.border_widths.left, Some(px(1.0).into()));
            assert_eq!(
                style.border_color,
                Some(if focused {
                    tokens.neutral.border_accent
                } else {
                    tokens.neutral.border
                })
            );
            assert_eq!(
                style.background,
                Some(
                    if focused {
                        tokens.neutral.surface2
                    } else {
                        tokens.neutral.surface1
                    }
                    .into()
                )
            );
        }
        let mut clear = clear_target("clear".into(), &tokens);
        let style = clear.style();
        assert_eq!(style.margin.left, Some(px(-8.0).into()));
        assert_eq!(style.padding.left, Some(px(8.0).into()));
        assert_eq!(style.background, None);
        assert_eq!(px(ICON), px(14.0));
        assert_eq!(px(INPUT_HEIGHT), px(20.0));
        assert_eq!(Glyph::Search.path(), "kit/search.svg");
        assert_eq!(Glyph::Remove.path(), "kit/remove.svg");
    }
}
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    field: SearchField,
    scheme: Scheme,
    seen: Seen,
    changes: Vec<String>,
    rail: Pixels,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        div().size_full().p(px(24.0)).child(
            div()
                .w(self.rail)
                .flex()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .child(self.field.element("filter", &Tokens::of(self.scheme), cx)),
        )
    }
}
#[gpui::test]
fn the_shared_clear_target_clears_the_real_engine_and_keeps_input_focus(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for scheme in [Scheme::Light, Scheme::Dark] {
        let seen = Seen::default();
        let (view, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| Probe {
                field: SearchField::new(
                    "Filter",
                    |this: &mut Probe, text, cx| {
                        this.changes.push(text);
                        cx.notify();
                    },
                    window,
                    cx,
                ),
                scheme,
                seen,
                changes: Vec::new(),
                rail: px(800.0),
            }
        });
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        let bounds = *seen.borrow().first().unwrap();
        assert_eq!(bounds.size.width, px(420.0));
        assert_eq!(bounds.size.height, px(34.0));
        assert!(cx.debug_bounds("search-field-clear").is_none());
        view.update_in(cx, |this, window, cx| this.field.editor.focus(window, cx));
        cx.run_until_parked();
        cx.simulate_input("한글 query");
        cx.run_until_parked();
        let clear = cx.debug_bounds("search-field-clear").unwrap();
        assert_eq!(clear.size.width, px(30.0));
        assert_eq!(clear.size.height, px(30.0));
        cx.simulate_click(clear.center(), Modifiers::none());
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| {
            assert_eq!(this.field.text(cx), "");
            assert!(this.field.editor.focus_handle(cx).is_focused(window));
            // simulate_input types each grapheme through the real engine;
            // X must append one empty Change after those ordinary edits.
            assert_eq!(
                this.changes,
                [
                    "한",
                    "한글",
                    "한글 ",
                    "한글 q",
                    "한글 qu",
                    "한글 que",
                    "한글 quer",
                    "한글 query",
                    "",
                ]
            );
        });
        assert!(cx.debug_bounds("search-field-clear").is_none());
        view.update_in(cx, |this, window, cx| {
            let editor = this.field.editor.id();
            this.field.set_placeholder("검색", window, cx);
            assert_eq!(this.field.editor.id(), editor);
            this.rail = px(260.0);
            cx.notify();
        });
        cx.run_until_parked();
        let narrow = *seen.borrow().first().unwrap();
        assert_eq!(narrow.size.width, px(260.0));
        assert_eq!(narrow.size.height, px(34.0));
        cx.simulate_input("after clear");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        let clear = cx.debug_bounds("search-field-clear").unwrap();
        assert!(clear.left() >= narrow.left());
        assert!(clear.right() <= narrow.right());
        assert!(clear.top() >= narrow.top());
        assert!(clear.bottom() <= narrow.bottom());
        view.read_with(cx, |this, cx| {
            assert_eq!(this.field.text(cx), "after clear");
        });
    }
}
