//! Actual GPUI canvas callbacks distinguish painted-row reuse from shared input data.
use super::*;
use crate::terminal_grid::{GridCursor, GridRow, GridRun, Ink, Strike, Underline};
use gpui::{Context, FontStyle, FontWeight, Render, TestAppContext, Window, px, size};

struct Demo {
    frame: Rc<GridFrame>,
    cell: CellSize,
    grid: RetainedGrid,
}
impl Demo {
    fn new(cx: &mut Context<'_, Self>) -> Self {
        let cell = CellSize {
            width: px(8.0),
            height: px(16.0),
        };
        let frame = Rc::new(GridFrame {
            rows: Rc::from([GridRow {
                fills: Vec::new(),
                runs: vec![GridRun {
                    col: 0,
                    text: "retained cells".into(),
                    ink: Ink {
                        color: gpui::rgb(0xffffff).into(),
                        weight: FontWeight::NORMAL,
                        style: FontStyle::Normal,
                        underline: Underline::None,
                        strike: Strike::None,
                    },
                }],
            }]),
            cursor: None,
            background: gpui::rgb(0x000000).into(),
            font_size: px(12.0),
        });
        let grid = RetainedGrid::new(Rc::clone(&frame), cell, cx);
        Self { frame, cell, grid }
    }
}
impl Render for Demo {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        self.grid.draw(Rc::clone(&self.frame), self.cell, cx)
    }
}

#[gpui::test]
fn cursor_only_update_reuses_actual_row_preparation_and_paint(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let window = cx.open_window(size(px(240.0), px(96.0)), |_, cx| Demo::new(cx));
    cx.run_until_parked();
    let before = window
        .update(cx, |demo, _, _| demo.grid.probe.counts())
        .unwrap();
    assert!(before.prepared.get() > 0 && before.painted.get() > 0);
    window
        .update(cx, |demo, _, cx| {
            Rc::make_mut(&mut demo.frame).cursor = Some(GridCursor {
                row: 0,
                col: 1,
                color: gpui::rgb(0xffffff).into(),
            });
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let after = window
        .update(cx, |demo, _, _| {
            assert!(demo.frame.cursor.is_some());
            demo.grid.probe.counts()
        })
        .unwrap();
    assert_eq!(before, after);
}
