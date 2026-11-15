// MODIFIED by Vessel (Team-WICKED), 2026-11-15: mapped document viewport tests (spikes/release/patches/README.md, "gpui-base", change 7)
// Vessel patch: use the existing editor and real parent list, never a fake range calculator.
use super::*;
use gpui::{ListAlignment, ListState, Render, StyleRefinement, list};

struct Document {
    editor: Entity<EditorState>,
    list: ListState,
    height: Pixels,
    cached: bool,
}
impl Render for Document {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        let cached = self.cached;
        let height = self.height;
        list(self.list.clone(), move |_, _, _| {
            let body = if cached {
                editor
                    .clone()
                    .cached(StyleRefinement::default().size_full())
                    .into_any_element()
            } else {
                editor.clone().into_any_element()
            };
            div().w_full().h(height).child(body).into_any_element()
        })
        .size_full()
    }
}

#[gpui::test]
fn outer_owner_scroll_reprojects_a_large_document_including_cached_children(
    cx: &mut TestAppContext,
) {
    for cached in [false, true] {
        cx.update(crate::init);
        let text = (0..2500)
            .map(|row| format!("line {row}"))
            .collect::<Vec<_>>()
            .join("\n");
        let calls = Arc::new(AtomicUsize::new(0));
        let list_state = ListState::new(1, ListAlignment::Top, px(0.));
        let mut input = None;
        let window = cx.open_window(size(px(240.), px(140.)), |window, cx| {
            let editor = cx.new(|cx| {
                let mut state = EditorState::new(window, cx)
                    .soft_wrap(false)
                    .folding(false)
                    .default_value(&text);
                state.set_readonly(true, cx);
                state.set_scroll_beyond_last_line(Some(0), window, cx);
                state.set_editor_style(state.editor_style.clone().with_line_presentation(Some(
                    Arc::new(Mapping {
                        sample: "9999".into(),
                        calls: calls.clone(),
                        colour: gpui::black(),
                    }),
                )));
                state
            });
            input = Some(editor.clone());
            Document {
                editor,
                list: list_state.clone(),
                height: window.line_height() * 2500,
                cached,
            }
        });
        let editor = input.unwrap();
        let owner = window.root(cx).unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let first = editor.read_with(&cx, |state, _| {
            let layout = state.last_layout.as_ref().unwrap();
            assert!(
                layout.lines.len() <= 12,
                "the 2500-line file shapes only its clipped viewport"
            );
            assert_eq!(layout.lines.len(), layout.visible_buffer_lines.len());
            layout.visible_range.clone()
        });
        calls.store(0, Ordering::Relaxed);
        // Notify only the real owner after its actual scroll; no child notify
        // or forced window.refresh masks a cache invalidation defect.
        cx.update(|_, cx| {
            list_state.scroll_by(px(4000.));
            owner.update(cx, |_, cx| cx.notify());
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        editor.read_with(&cx, |state, _| {
            let layout = state.last_layout.as_ref().unwrap();
            assert!(layout.visible_range.start > first.end);
            assert!(layout.lines.len() <= 12);
            assert_eq!(layout.lines.len(), layout.visible_buffer_lines.len());
            assert!(calls.load(Ordering::Relaxed) > 0);
            assert!(
                calls.load(Ordering::Relaxed) <= 24,
                "at most two visible projections, never all 2500 rows"
            );
            assert_eq!(state.text().to_string(), text);
            assert_eq!(state.display_map.wrap_row_count(), 2500);
        });
    }
}

#[gpui::test]
fn clipped_positions_are_absent_before_after_and_empty_but_correct_at_line_boundaries(
    cx: &mut TestAppContext,
) {
    let text = (0..2500)
        .map(|row| format!("row{row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (editor, window) = decoration_editor(cx, &text, false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        editor.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            state.set_editor_style(state.editor_style.clone().with_line_presentation(Some(
                Arc::new(Mapping {
                    sample: "9999".into(),
                    calls: Arc::new(AtomicUsize::new(0)),
                    colour: gpui::black(),
                }),
            )));
        })
    });
    let (_, paint) = cx.draw(
        point(px(0.), px(-4000.5)),
        size(px(240.), px(50000.)),
        |_, _| TextElement::new(editor.clone()),
    );
    assert!(paint.last_layout.lines.len() <= 12);
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            let layout = state.last_layout.clone().unwrap();
            assert_eq!(layout.lines.len(), layout.visible_buffer_lines.len());
            let first = layout.visible_buffer_lines[0];
            assert!(first > 0);
            assert!(state.line_and_position_for_offset(0).2.is_none());
            assert!(
                state
                    .line_and_position_for_offset(state.text.len())
                    .2
                    .is_none()
            );
            let end = state.text.line_end_offset(first);
            let next = state.text.line_start_offset(first + 1);
            let a = state.line_and_position_for_offset(end).2.unwrap();
            let b = state.line_and_position_for_offset(next).2.unwrap();
            assert_eq!(b.y - a.y, layout.line_height);
            let bounds = state.input_bounds();
            assert!(state.bounds_for_range(0..3, bounds, window, cx).is_none());
            let end_utf16 = state.offset_to_utf16(state.text.len());
            assert!(
                state
                    .bounds_for_range(end_utf16..end_utf16, bounds, window, cx)
                    .is_none()
            );
            let shown_utf16 = state.offset_to_utf16(next);
            assert!(
                state
                    .bounds_for_range(0..shown_utf16, bounds, window, cx)
                    .is_none()
            );
            let boundary_utf16 = state.offset_to_utf16(end);
            let caret = state
                .bounds_for_range(boundary_utf16..boundary_utf16, bounds, window, cx)
                .unwrap();
            assert_eq!(caret.top(), bounds.top() + a.y);
            let next_caret = state
                .bounds_for_range(shown_utf16..shown_utf16, bounds, window, cx)
                .unwrap();
            assert_eq!(next_caret.top() - caret.top(), layout.line_height);
            state.select_all(window, cx);
            state.copy(&Copy, window, cx);
            assert_eq!(state.selected_text().to_string(), text);
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), text);
        })
    });
    let (_, empty) = cx.draw(
        point(px(300.), px(0.)),
        size(px(240.), px(50000.)),
        |_, _| TextElement::new(editor.clone()),
    );
    assert!(empty.last_layout.lines.is_empty());
    assert!(empty.last_layout.visible_buffer_lines.is_empty());
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            assert!(state.line_and_position_for_offset(0).2.is_none());
            assert!(state.range_to_bounds(&(0..3)).is_none());
            assert!(
                state
                    .character_index_for_point(point(px(0.), px(0.)), window, cx)
                    .is_none()
            );
            let bounds = state.input_bounds();
            assert!(state.bounds_for_range(0..3, bounds, window, cx).is_none());
        })
    });
}

#[gpui::test]
fn ordinary_none_keeps_the_upstream_offscreen_lookup_result(cx: &mut TestAppContext) {
    let text = "source line\n".repeat(200);
    let (editor, window) = decoration_editor(cx, &text, false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        editor.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            state.set_scroll_offset(point(px(0.), px(-400.)), cx);
        })
    });
    let (_, before) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    let previous = editor.read_with(&cx, |state, _| state.line_and_position_for_offset(0));
    assert!(before.last_layout.visible_range.start > 0);
    // This wrong-looking coordinate is deliberately the old ordinary-input
    // behaviour; patch7 cannot silently fix all editor consumers.
    assert!(previous.2.is_some());
    cx.update(|_, cx| {
        editor.update(cx, |state, _| {
            state.set_editor_style(state.editor_style.clone().with_line_presentation(None));
        })
    });
    let (_, after) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    assert_eq!(
        before.last_layout.visible_range,
        after.last_layout.visible_range
    );
    assert_eq!(before.scroll_size, after.scroll_size);
    assert_eq!(
        editor.read_with(&cx, |state, _| state.line_and_position_for_offset(0)),
        previous
    );
}

#[test]
fn document_intersection_keeps_fractional_boundary_rows_and_both_axes() {
    let full = Bounds::new(point(px(10.), px(-4000.5)), size(px(240.), px(50000.)));
    let mask = Bounds::new(point(px(0.), px(0.)), size(px(300.), px(140.)));
    assert_eq!(mapped_document_viewport(full, mask), px(4000.5)..px(4140.5));
    let beside = Bounds::new(point(px(300.), px(0.)), size(px(50.), px(140.)));
    assert!(mapped_document_viewport(full, beside).is_empty());
}

#[gpui::test]
fn a_crossclip_selection_keeps_visible_paint_handle_and_complete_copy(cx: &mut TestAppContext) {
    let text = (0..2500)
        .map(|row| format!("source {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (editor, window) = decoration_editor(cx, &text, false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        window.activate_window();
        editor.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            state.focus(window, cx);
            state.set_editor_style(state.editor_style.clone().with_line_presentation(Some(
                Arc::new(Mapping {
                    sample: "9999".into(),
                    calls: Arc::new(AtomicUsize::new(0)),
                    colour: gpui::black(),
                }),
            )));
        });
    });
    let (_, first) = cx.draw(
        point(px(0.), px(-4000.5)),
        size(px(240.), px(50000.)),
        |_, _| TextElement::new(editor.clone()),
    );
    let end = editor.read_with(&cx, |state, _| {
        state
            .text
            .line_start_offset(first.last_layout.visible_range.start + 2)
    });
    cx.update(|_, cx| {
        editor.update(cx, |state, cx| {
            state.set_selection(0, end);
            state.keep_touch_selection(cx);
        })
    });
    let (_, selected) = cx.draw(
        point(px(0.), px(-4000.5)),
        size(px(240.), px(50000.)),
        |_, _| TextElement::new(editor.clone()),
    );
    assert!(
        !selected.selection_paths.is_empty(),
        "selection is clipped for paint, not discarded for an unseen endpoint"
    );
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            let touch = state.touch_selection().unwrap();
            assert!(!touch.is_edge_visible(crate::SelectionEdge::Start));
            assert!(touch.is_edge_visible(crate::SelectionEdge::End));
            assert!(touch.end().top() >= px(0.));
            state.copy(&Copy, window, cx);
            assert_eq!(state.selected_text().to_string(), &text[..end]);
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some(text[..end].to_owned())
            );
            let bounds = state.input_bounds();
            let end_utf16 = state.offset_to_utf16(end);
            assert!(
                state
                    .bounds_for_range(0..end_utf16, bounds, window, cx)
                    .is_none()
            );
        })
    });
}

#[gpui::test]
fn mapped_position_rows_keep_shape_order_when_the_existing_engine_folds(cx: &mut TestAppContext) {
    let text = "top\nhidden one\nhidden two\nend\nlast";
    let (editor, window) = decoration_editor(cx, text, false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        editor.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            state.set_editor_style(state.editor_style.clone().with_line_presentation(Some(
                Arc::new(Mapping {
                    sample: "9999".into(),
                    calls: Arc::new(AtomicUsize::new(0)),
                    colour: gpui::black(),
                }),
            )));
            state.apply_highlighter_fold_candidates(vec![FoldRange::new(0, 3)], cx);
            state.display_map.set_folded(0, true);
        })
    });
    let (_, paint) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    assert_eq!(paint.last_layout.visible_buffer_lines, vec![0, 3, 4]);
    assert_eq!(
        paint.last_layout.lines.len(),
        paint.last_layout.visible_buffer_lines.len()
    );
    editor.read_with(&cx, |state, _| {
        assert!(
            state
                .line_and_position_for_offset(state.text.line_start_offset(1))
                .2
                .is_none()
        );
        let top = state.line_and_position_for_offset(0).2.unwrap();
        let end = state
            .line_and_position_for_offset(state.text.line_start_offset(3))
            .2
            .unwrap();
        assert_eq!(end.y - top.y, paint.last_layout.line_height);
    });
}
