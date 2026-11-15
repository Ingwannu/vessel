// MODIFIED by Vessel (Team-WICKED), 2026-11-15: optional line presentation tests (spikes/release/patches/README.md, "gpui-base", change 7)
// Vessel patch: these use the existing real editor harness and rendered scene.
use super::*;
use crate::input::{BufferRow, Copy, InputEditorStyle, InputLinePaint, InputLinePresentation};
use std::cell::Cell;
use std::rc::Rc;

struct Mapping {
    sample: SharedString,
    calls: Rc<Cell<usize>>,
    colour: Hsla,
}
impl InputLinePresentation for Mapping {
    fn gutter_sample(&self) -> SharedString {
        self.sample.clone()
    }
    fn line(&self, row: BufferRow) -> InputLinePaint {
        self.calls.set(self.calls.get() + 1);
        InputLinePaint {
            number: (row.index() == 0).then(|| "42".into()),
            number_color: None,
            background: (row.index() == 0).then_some(self.colour),
        }
    }
}

#[gpui::test]
fn no_mapping_keeps_the_original_actual_window_gutter(cx: &mut TestAppContext) {
    let (editor, window) = decoration_editor(cx, "one\ntwo\n", false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let (_, before) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    cx.update(|_, cx| {
        editor.update(cx, |state, _| {
            let style = InputEditorStyle {
                line_presentation: None,
                ..state.editor_style.clone()
            };
            state.set_editor_style(style);
        })
    });
    let (_, after) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    assert_eq!(
        before.last_layout.line_number_width,
        after.last_layout.line_number_width
    );
    assert_eq!(before.scroll_size, after.scroll_size);
    assert!(before.line_paints.is_none());
    assert!(after.line_paints.is_none());
    let widths = |paint: &PrepaintState| {
        paint
            .line_numbers
            .as_ref()
            .unwrap()
            .iter()
            .map(|lines| lines.iter().map(|line| line.width).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    assert_eq!(widths(&before), widths(&after));
}

#[gpui::test]
fn mapped_labels_use_actual_font_width_and_one_projection_per_visible_row(cx: &mut TestAppContext) {
    let (editor, window) = decoration_editor(cx, "one\ntwo\n", false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let calls = Rc::new(Cell::new(0));
    let sample: SharedString = "9999999".into();
    let mapping = Rc::new(Mapping {
        sample: sample.clone(),
        calls: calls.clone(),
        colour: gpui::hsla(0.23, 0.91, 0.42, 0.31),
    });
    cx.update(|_, cx| {
        editor.update(cx, |state, _| {
            state.editor_style.line_presentation = Some(mapping.clone());
            state.set_editor_style(state.editor_style.clone());
        })
    });
    calls.set(0);
    let (_, paint) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    assert_eq!(calls.get(), paint.last_layout.visible_buffer_lines.len());
    cx.update(|window, _| {
        let style = window.text_style();
        let shaped = window.text_system().shape_line(
            sample.clone(),
            style.font_size.to_pixels(window.rem_size()),
            &[TextRun {
                len: sample.len(),
                font: style.font(),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        assert_eq!(
            paint.last_layout.line_number_width,
            shaped.width + LINE_NUMBER_RIGHT_MARGIN + FOLD_ICON_HITBOX_WIDTH
        );
    });
    let label = &paint.line_numbers.as_ref().unwrap()[0][0];
    let origin = mapped_number_origin(
        px(5.),
        paint.last_layout.line_number_width,
        label.width,
        px(0.),
    );
    assert_eq!(
        origin.x + label.width + LINE_NUMBER_RIGHT_MARGIN,
        px(5.) + paint.last_layout.line_number_width
    );
    assert!(paint.line_numbers.as_ref().unwrap()[1][0].width == px(0.));
}

#[gpui::test]
fn row_tints_cover_wraps_and_fixed_scrolled_gutters_inside_horizontal_bounds(
    cx: &mut TestAppContext,
) {
    let text = format!("{}\nnext", "wide ".repeat(80));
    let (editor, window) = decoration_editor(cx, &text, true);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let colour = gpui::hsla(0.23, 0.91, 0.42, 0.31);
    cx.update(|_, cx| {
        editor.update(cx, |state, cx| {
            state.set_editor_paddings(Edges {
                left: px(5.),
                ..Edges::default()
            });
            let style = InputEditorStyle {
                line_presentation: Some(Rc::new(Mapping {
                    sample: "9999".into(),
                    calls: Rc::new(Cell::new(0)),
                    colour,
                })),
                ..state.editor_style.clone()
            };
            state.set_editor_style(style);
            cx.notify();
        })
    });
    let gutter = cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        let state = editor.read(cx);
        let layout = state.last_layout.as_ref().unwrap();
        assert!(layout.lines[0].wrapped_lines.len() > 1);
        let quads = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.background == gpui::solid_background(colour))
            .collect::<Vec<_>>();
        assert!(
            quads.len() >= 2,
            "body and fixed gutter must both be tinted"
        );
        let edge = state
            .text_bounds()
            .unwrap()
            .right()
            .scale(window.scale_factor());
        assert!(
            quads
                .iter()
                .all(|quad| quad.bounds.right() <= edge + gpui::ScaledPixels(1.))
        );
        let body = quads
            .iter()
            .max_by(|a, b| {
                a.bounds
                    .size
                    .width
                    .partial_cmp(&b.bounds.size.width)
                    .unwrap()
            })
            .unwrap();
        assert!(body.bounds.size.height > layout.line_height.scale(window.scale_factor()));
        quads
            .iter()
            .min_by(|a, b| {
                a.bounds
                    .size
                    .width
                    .partial_cmp(&b.bounds.size.width)
                    .unwrap()
            })
            .unwrap()
            .bounds
    });
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            state.set_soft_wrap(false, window, cx);
        })
    });
    // Horizontal movement must not translate the fixed gutter; reflow itself
    // may change vertical geometry, so compare its horizontal bounds only.
    cx.update(|window, cx| {
        editor.update(cx, |state, cx| {
            state.set_scroll_offset(point(px(-80.), px(0.)), cx);
        });
        window.refresh();
        window.draw(cx).clear(cx);
        let quads = window
            .painted_quads()
            .into_iter()
            .filter(|quad| quad.background == gpui::solid_background(colour))
            .collect::<Vec<_>>();
        let fixed = quads
            .iter()
            .min_by(|a, b| {
                a.bounds
                    .size
                    .width
                    .partial_cmp(&b.bounds.size.width)
                    .unwrap()
            })
            .unwrap()
            .bounds;
        assert_eq!(fixed.origin.x, gutter.origin.x);
        assert_eq!(fixed.size.width, gutter.size.width);
    });
}

#[gpui::test]
fn mapped_readonly_rows_keep_cross_row_selection_and_copy_without_numbers(cx: &mut TestAppContext) {
    let text = "old source\n새 줄\nlast";
    let (editor, window) = decoration_editor(cx, text, false);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        window.activate_window();
        editor.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            state.focus(window, cx);
            state.editor_style.line_presentation = Some(Rc::new(Mapping {
                sample: "99999".into(),
                calls: Rc::new(Cell::new(0)),
                colour: gpui::hsla(0.23, 0.91, 0.42, 0.31),
            }));
            state.set_editor_style(state.editor_style.clone());
            state.select_all(window, cx);
            state.copy(&Copy, window, cx);
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()),
                Some(text.to_owned())
            );
            state.replace_text_in_range(None, "blocked", window, cx);
            assert_eq!(state.value(), text);
            assert_eq!(state.selected_text().to_string(), text);
        });
    });
    let (_, paint) = cx.draw(point(px(0.), px(0.)), size(px(240.), px(140.)), |_, _| {
        TextElement::new(editor.clone())
    });
    assert!(
        !paint.selection_paths.is_empty(),
        "selection remains laid out above the row tint"
    );
    assert!(paint.line_paints.is_some());
}
