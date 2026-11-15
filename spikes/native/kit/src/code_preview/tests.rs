//! Actual source renderer bounds at both schemes and all four scales.
use super::*;
use crate::{Scheme, fonts::Typeface, syntax::SyntaxStyle};
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
const EPSILON: f32 = 0.02;

struct Probe {
    scheme: Scheme,
    width: f32,
    rows: Vec<PreviewRow>,
    typography: PreviewTypography,
    seen: Seen,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        code_preview(
            "Preview",
            &self.rows,
            &self.typography,
            &Tokens::of(self.scheme),
        )
        .w(px(self.width))
        .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
    }
}
fn typography() -> PreviewTypography {
    PreviewTypography {
        content: FontStack::from_names([], Typeface::Interface),
        code: FontStack::from_names([], Typeface::Code),
        content_size: FontSize::new(15).unwrap(),
        code_size: FontSize::new(12).unwrap(),
    }
}
fn rows() -> Vec<PreviewRow> {
    [Tone::Plain,Tone::Added,Tone::Removed].into_iter().enumerate().map(|(id,tone)| {
        PreviewRow::new(("preview-row",id), "const long_name = \"this long code row must remain a single line even when it exceeds its card\";", tone,
            vec![CodeRun{range:0..5,style:SyntaxStyle::Keyword}]).unwrap()
    }).collect()
}
fn near(actual: Pixels, expected: f32) {
    assert!(
        expected.mul_add(-1.0, f32::from(actual)).abs() < EPSILON,
        "{actual:?} != {expected}"
    );
}

#[gpui::test]
fn source_card_caption_and_full_rows_fit_both_schemes_and_four_scales(cx: &mut TestAppContext) {
    // Physical border snapping and measured text heights are calculated from
    // py8/px12/pb8/border1, content15/21 and code12/18, not from kit constants.
    // GPUI Taffy pre-snaps authored absolute widths with half toward zero:
    // 390 * 1.25 = 487.5 becomes487dp (389.6 logical). Each border is1dp,
    // so the inner width is485/1.25 =388; subtracting borders from390 is wrong.
    for (scale, edge, top, caption_height, row_height, small_outer) in [
        (1.0, 1.0, 9.0, 29.0, 18.0, 390.0),
        (1.25, 0.8, 8.8, 29.6, 18.4, 389.6),
        (1.5, 0.666_666_7, 8.666_667, 29.333_334, 18.0, 390.0),
        (2.0, 1.0, 9.0, 29.0, 18.0, 390.0),
    ] {
        for (width, expected_outer) in [(390.0, small_outer), (1280.0, 1280.0)] {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let seen = Seen::default();
                let (_, cx) = cx.add_window_view({
                    let seen = Rc::clone(&seen);
                    move |_, _| Probe {
                        scheme,
                        width,
                        rows: rows(),
                        typography: typography(),
                        seen,
                    }
                });
                cx.simulate_scale_factor_change(scale);
                cx.run_until_parked();
                let bounds = seen.borrow();
                assert_eq!(bounds.len(), 4);
                near(bounds[0].origin.x, edge);
                near(bounds[0].origin.y, top);
                near(bounds[0].size.width, edge.mul_add(-2.0, expected_outer));
                near(bounds[0].size.height, caption_height);
                for (index, row) in bounds.iter().skip(1).enumerate() {
                    near(row.origin.x, edge);
                    near(row.size.width, edge.mul_add(-2.0, expected_outer));
                    near(
                        row.origin.y,
                        f32::from(u16::try_from(index).unwrap())
                            .mul_add(row_height, top.mul_add(1.0, caption_height)),
                    );
                    near(row.size.height, row_height);
                }
            }
        }
    }
}

#[test]
fn constructor_refuses_bad_spans_and_prepares_the_prefix_once() {
    let run = |range| CodeRun {
        range,
        style: SyntaxStyle::String,
    };
    assert!(matches!(
        PreviewRow::new("bad", "한", Tone::Plain, vec![run(1..3)]),
        Err(PreviewError::Encoding)
    ));
    assert!(matches!(
        PreviewRow::new("bad", "abc", Tone::Plain, vec![run(0..9)]),
        Err(PreviewError::Range)
    ));
    assert!(matches!(
        PreviewRow::new("bad", "abc", Tone::Plain, vec![run(0..2), run(1..3)]),
        Err(PreviewError::Range)
    ));
    assert!(matches!(
        PreviewRow::new("bad", "a\nb", Tone::Plain, vec![]),
        Err(PreviewError::Newline)
    ));
    let row = PreviewRow::new("good", "한", Tone::Added, vec![run(0..3)]).unwrap();
    assert_eq!(row.text.as_ref(), "+ 한");
    assert_eq!(row.runs[0].range, 2..5);
}
