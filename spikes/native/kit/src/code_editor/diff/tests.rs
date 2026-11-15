//! Buffer admission and the actual readonly profile, on the shared engine.

use super::*;
#[test]
fn metadata_must_cover_trailing_empty_buffer_rows() {
    let rows: Arc<[DiffLine]> = Arc::from([DiffLine {
        number: NonZeroU32::new(9),
        tone: DiffTone::Added,
    }]);
    assert_eq!(
        DiffText::new(Arc::from("x\n"), rows).unwrap_err(),
        DiffTextError::RowCount { lines: 2, rows: 1 }
    );
    let admitted = DiffText::new(
        Arc::from(""),
        Arc::from([DiffLine {
            number: None,
            tone: DiffTone::Empty,
        }]),
    )
    .unwrap();
    assert_eq!(admitted.text.as_ref(), "");
    assert_eq!(admitted.sample.as_ref(), "00");
}
#[test]
fn sample_uses_actual_mapped_numbers_with_a_two_digit_minimum() {
    let rows = Arc::from([
        DiffLine {
            number: NonZeroU32::new(3),
            tone: DiffTone::Context,
        },
        DiffLine {
            number: NonZeroU32::new(999_999),
            tone: DiffTone::Removed,
        },
    ]);
    let admitted = DiffText::new(Arc::from("a\nb"), rows).unwrap();
    assert_eq!(admitted.sample.as_ref(), "999999");
}

pub(super) struct Plain;
impl LineSyntax for Plain {
    type State = ();
    fn language(&self) -> gpui::SharedString {
        "text".into()
    }
    fn line(&self, _text: &str, _state: ()) -> (super::super::Runs, ()) {
        (Vec::new(), ())
    }
}
struct Probe {
    editor: DiffEditor,
}
impl gpui::Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.editor
            .element(&Tokens::of(crate::theme::Scheme::Dark), cx)
    }
}
#[gpui::test]
fn the_profile_rejects_input_and_keeps_selection_on_the_shared_engine(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::EntityInputHandler as _;
    cx.update(gpui_base::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let text = DiffText::new(
            Arc::from("old\n새 줄"),
            Arc::from([
                DiffLine {
                    number: NonZeroU32::new(42),
                    tone: DiffTone::Removed,
                },
                DiffLine {
                    number: NonZeroU32::new(99),
                    tone: DiffTone::Added,
                },
            ]),
        )
        .unwrap();
        Probe {
            editor: DiffEditor::new(text, Plain, DiffWrapping::Scroll, window, cx),
        }
    });
    cx.update(|window, cx| {
        view.update(cx, |probe, cx| {
            probe.editor.focus(window, cx);
            let state = probe.editor.editor.state.clone();
            state.update(cx, |state, cx| {
                state.select_all(window, cx);
                assert_eq!(state.selected_text().to_string(), "old\n새 줄");
                state.replace_text_in_range(None, "changed", window, cx);
                assert_eq!(state.value(), "old\n새 줄");
            });
            probe.editor.set_wrapping(DiffWrapping::Wrap, window, cx);
            assert_eq!(probe.editor.editor.text(cx), "old\n새 줄");
        });
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

#[test]
fn non_source_rows_cannot_acquire_source_numbers_and_split_side_is_explicit() {
    for tone in [DiffTone::Empty, DiffTone::Header, DiffTone::Notice] {
        assert_eq!(
            DiffText::new(
                Arc::from("metadata"),
                Arc::from([DiffLine {
                    number: NonZeroU32::new(42),
                    tone
                }])
            )
            .unwrap_err(),
            DiffTextError::NonSourceNumber {
                row: DiffRowIndex(0)
            }
        );
    }
    let text = DiffText::new(
        Arc::from("source"),
        Arc::from([DiffLine {
            number: None,
            tone: DiffTone::Context,
        }]),
    )
    .unwrap();
    assert_eq!(text.side, DiffSide::Modified);
    assert_eq!(text.with_side(DiffSide::Original).side, DiffSide::Original);
}
