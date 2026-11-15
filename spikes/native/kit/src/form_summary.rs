//! GOAT's commit summary and its file-list inset (`GitActionsControl.tsx`).
//!
//! [Decision Log]
//! - Purpose: keep a form's branch/files group on the reference's shared frame,
//!   rather than repainting it in an area or changing Settings' radius8 card.
//! - Reference: `GitActionsControl.tsx:1914–2030` and `index.css:273`: radius12,
//!   spacing3 padding/gap, zinc25/light and white3.5%/dark; radius8 inset at
//!   176 points, card/light and white2.5%/dark; static black5%/white5% edges.
//! - Alternatives: a layout border steals content space. GPUI drop shadows
//!   paint beneath translucent fills too, incorrectly tinting the interior;
//!   their spread also keeps the original rather than expanded corner radius.
//! - Choice: a passive one-point border on an absolute child lies entirely
//!   outside the frame, like the static CSS ring, and never depends on focus.
//!   Its outer radius includes that point; no border enters content layout.
//! - Why: existing settings cards and their callers keep their exact geometry.
//! - Cost: neutral OKLCH zinc25 (.992,0,0) is encoded as its nearest 8-bit sRGB
//!   value #fcfcfc; the dark overlays keep the reference's exact alpha values.

use gpui::prelude::*;
use gpui::{Div, Hsla, div, px, rgb};

use crate::theme::{SUMMARY_LIGHT, Scheme, Tokens, spacing};

const SUMMARY_RADIUS: f32 = 12.0;
const INSET_RADIUS: f32 = 8.0;
const FILES_HEIGHT: f32 = 176.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Paint {
    fill: Hsla,
    edge: Hsla,
}

fn summary_paint(scheme: Scheme) -> Paint {
    match scheme {
        Scheme::Light => Paint {
            fill: rgb(SUMMARY_LIGHT).into(),
            edge: gpui::black().opacity(0.05),
        },
        Scheme::Dark => Paint {
            fill: gpui::white().opacity(0.035),
            edge: gpui::white().opacity(0.05),
        },
    }
}

fn inset_paint(scheme: Scheme) -> Paint {
    match scheme {
        Scheme::Light => Paint {
            fill: gpui::white(),
            edge: gpui::black().opacity(0.05),
        },
        Scheme::Dark => Paint {
            fill: gpui::white().opacity(0.025),
            edge: gpui::white().opacity(0.05),
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum Frame {
    Summary,
    Files,
}

impl Frame {
    const fn radii(self) -> (f32, f32) {
        match self {
            Self::Summary => (SUMMARY_RADIUS, 13.0),
            Self::Files => (INSET_RADIUS, 9.0),
        }
    }
}

fn frame(paint: Paint, kind: Frame) -> Div {
    let (inner, outer) = kind.radii();
    div().relative().bg(paint.fill).rounded(px(inner)).child(
        div()
            .absolute()
            .top(px(-1.0))
            .right(px(-1.0))
            .bottom(px(-1.0))
            .left(px(-1.0))
            .rounded(px(outer))
            .border_1()
            .border_color(paint.edge),
    )
}

/// The branch/files summary frame; the caller supplies its content in order.
#[must_use]
pub fn summary(tokens: &Tokens) -> Div {
    frame(summary_paint(tokens.scheme), Frame::Summary)
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .p(px(spacing::S3))
}

/// The summary's fixed-height file-list viewport. Its caller supplies the
/// existing virtual list, with spacing1 insets/row gaps as in the reference.
#[must_use]
pub fn files(tokens: &Tokens) -> Div {
    frame(inset_paint(tokens.scheme), Frame::Files)
        .w_full()
        .h(px(FILES_HEIGHT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_and_file_list_keep_the_references_exact_geometry_and_paint() {
        assert_eq!(SUMMARY_RADIUS.to_bits(), 12.0_f32.to_bits());
        assert_eq!(INSET_RADIUS.to_bits(), 8.0_f32.to_bits());
        assert_eq!(FILES_HEIGHT.to_bits(), 176.0_f32.to_bits());
        assert_eq!(spacing::S3.to_bits(), 12.0_f32.to_bits());
        assert_eq!(
            summary_paint(Scheme::Light),
            Paint {
                fill: rgb(0x00fc_fcfc).into(),
                edge: gpui::black().opacity(0.05),
            }
        );
        assert_eq!(
            summary_paint(Scheme::Dark),
            Paint {
                fill: gpui::white().opacity(0.035),
                edge: gpui::white().opacity(0.05),
            }
        );
        assert_eq!(
            inset_paint(Scheme::Light),
            Paint {
                fill: gpui::white(),
                edge: gpui::black().opacity(0.05),
            }
        );
        assert_eq!(
            inset_paint(Scheme::Dark),
            Paint {
                fill: gpui::white().opacity(0.025),
                edge: gpui::white().opacity(0.05),
            }
        );
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use gpui::{Bounds, Pixels, Render, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    type Seen = Rc<RefCell<Option<Bounds<Pixels>>>>;

    struct Probe {
        scheme: Scheme,
        summary: Seen,
        files: Seen,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut gpui::Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let summary_seen = Rc::clone(&self.summary);
            let files_seen = Rc::clone(&self.files);
            let tokens = Tokens::of(self.scheme);
            div()
                .w(px(200.0))
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(
                    summary(&tokens)
                        .on_children_prepainted(move |bounds, _, _| {
                            *summary_seen.borrow_mut() = bounds.last().copied();
                        })
                        .child(div().h(px(20.0))),
                )
                .child(
                    files(&tokens)
                        .on_children_prepainted(move |bounds, _, _| {
                            *files_seen.borrow_mut() = bounds.last().copied();
                        })
                        .child(div().w_full().h(px(20.0))),
                )
        }
    }

    #[gpui::test]
    fn the_static_outer_edge_does_not_consume_a_pixel_of_layout(cx: &mut TestAppContext) {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let summary = Rc::new(RefCell::new(None));
            let files = Rc::new(RefCell::new(None));
            let (_, cx) = cx.add_window_view({
                let summary = Rc::clone(&summary);
                let files = Rc::clone(&files);
                move |_, _| Probe {
                    scheme,
                    summary,
                    files,
                }
            });
            cx.run_until_parked();
            let summary = summary.borrow().expect("the summary child was laid out");
            let files = files.borrow().expect("the file-list child was laid out");
            assert_eq!(summary.origin.x, px(12.0));
            assert_eq!(summary.size.width, px(176.0));
            assert_eq!(files.origin.x, px(0.0));
            assert_eq!(files.size.width, px(200.0));
        }
    }
}
