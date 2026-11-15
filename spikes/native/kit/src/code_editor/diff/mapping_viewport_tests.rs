//! Real parent scrolling and IME range queries, without private Base layout data.
use super::*;
use gpui::{ListAlignment, ListState, StyleRefinement, list};

struct Document {
    state: Entity<EditorState>,
    list: ListState,
    height: gpui::Pixels,
    cached: bool,
}
impl Render for Document {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let height = self.height;
        let cached = self.cached;
        list(self.list.clone(), move |_, _, _| {
            let body = if cached {
                state
                    .clone()
                    .cached(StyleRefinement::default().size_full())
                    .into_any_element()
            } else {
                Editor::new(&state).into_any_element()
            };
            div().w_full().h(height).child(body).into_any_element()
        })
        .size_full()
    }
}
#[gpui::test]
fn parent_scroll_reprojects_bounded_rows_including_a_cached_editor(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let text = (0..2500)
        .map(|row| format!("row{row}"))
        .collect::<Vec<_>>()
        .join("\n");
    for cached in [false, true] {
        let mapping = Mapping::new("9999");
        let list_state = ListState::new(1, ListAlignment::Top, px(0.));
        let (owner, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| {
                let mut state = EditorState::new(window, cx)
                    .soft_wrap(false)
                    .folding(false)
                    .default_value(&text);
                state.set_readonly(true, cx);
                state.set_scroll_beyond_last_line(Some(0), window, cx);
                state.set_editor_style(mapping.style());
                state
            });
            Document {
                state,
                list: list_state.clone(),
                height: window.line_height() * 2500,
                cached,
            }
        });
        cx.run_until_parked();
        mapping.rows.lock().unwrap().clear();
        // Invalidate only the real owner, never the child or whole window.
        cx.update(|_, cx| {
            list_state.scroll_by(px(4000.5));
            owner.update(cx, |_, cx| cx.notify());
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let projected = mapping.rows.lock().unwrap().clone();
        assert!(
            !projected.is_empty(),
            "new parent viewport must repaint even a cached child"
        );
        assert!(projected.iter().all(|row| *row > 0));
        assert!(
            projected.len() < 200,
            "viewport plus one-row overscan, never all2500 rows"
        );
        let first = *projected.iter().min().unwrap();
        let start = text.find(&format!("row{first}\n")).unwrap();
        let end = start + format!("row{first}").len();
        let next = end + 1;
        cx.update(|window, cx| {
            owner.update(cx, |owner, cx| {
                owner.state.update(cx, |state, cx| {
                    let bounds = state.input_bounds();
                    assert_eq!(state.display_map().wrap_row_count(), 2500);
                    assert_eq!(state.value(), text);
                    assert!(state.range_to_bounds(&(0..0)).is_none());
                    assert!(state.bounds_for_range(0..0, bounds, window, cx).is_none());
                    assert!(
                        state
                            .bounds_for_range(text.len()..text.len(), bounds, window, cx)
                            .is_none()
                    );
                    assert!(
                        state
                            .bounds_for_range(0..next, bounds, window, cx)
                            .is_none()
                    );
                    let a = state
                        .bounds_for_range(end..end, bounds, window, cx)
                        .unwrap();
                    let b = state
                        .bounds_for_range(next..next, bounds, window, cx)
                        .unwrap();
                    assert_eq!(b.top() - a.top(), state.line_height().unwrap());
                    state.select_all(window, cx);
                    state.copy(&gpui_base::input::Copy, window, cx);
                    assert_eq!(
                        cx.read_from_clipboard().and_then(|v| v.text()),
                        Some(text.clone())
                    );
                })
            })
        });
    }
}
#[gpui::test]
fn empty_parent_mask_shapes_nothing_and_refuses_direct_ime_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let mapping = Mapping::new("9999");
    let (owner, cx) = cx.add_window_view(|w, cx| input("one\ntwo", mapping.style(), w, cx));
    let state = owner.read_with(cx, |owner, _| owner.state.clone());
    mapping.rows.lock().unwrap().clear();
    let _ = cx.draw(
        point(px(2000.), px(0.)),
        size(px(240.), px(5000.)),
        |_, _| Editor::new(&state),
    );
    assert!(mapping.rows.lock().unwrap().is_empty());
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            let bounds = state.input_bounds();
            assert!(state.range_to_bounds(&(0..0)).is_none());
            assert!(state.bounds_for_range(0..0, bounds, window, cx).is_none());
            assert!(
                state
                    .character_index_for_point(point(px(0.), px(0.)), window, cx)
                    .is_none()
            );
            assert_eq!(state.value(), "one\ntwo");
        })
    });
}
#[gpui::test]
fn none_preserves_the_existing_offscreen_coordinate_result(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let text = "source line\n".repeat(200);
    let (owner, cx) = cx.add_window_view(|w, cx| input(&text, complete_literal(), w, cx));
    let state = owner.read_with(cx, |owner, _| owner.state.clone());
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            state.set_scroll_offset(point(px(0.), px(-400.)), cx)
        });
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let before = state.read_with(cx, |state, _| {
        (state.range_to_bounds(&(0..0)), state.scroll_offset())
    });
    assert!(
        before.0.is_some(),
        "ordinary upstream offclip behavior deliberately remains"
    );
    cx.update(|window, cx| {
        state.update(cx, |state, _| {
            state.set_editor_style(complete_literal().with_line_presentation(None))
        });
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let after = state.read_with(cx, |state, _| {
        (state.range_to_bounds(&(0..0)), state.scroll_offset())
    });
    assert_eq!(before, after);
}
