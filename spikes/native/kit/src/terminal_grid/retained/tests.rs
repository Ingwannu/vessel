//! Actual GPUI canvas callbacks distinguish painted-row reuse from shared input data.
use super::*;
use crate::terminal_grid::{GridCursor, GridRow, GridRun, Ink, Strike, Underline};
use gpui::{Context, FontStyle, FontWeight, Render, TestAppContext, Window, px, size};

struct Demo {
    frame: Rc<GridFrame>,
    cell: CellSize,
    grid: RetainedGrid,
    font: FontStack,
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
                        color: gpui::rgb(0x00ff_ffff).into(),
                        weight: FontWeight::NORMAL,
                        style: FontStyle::Normal,
                        underline: Underline::None,
                        strike: Strike::None,
                    },
                }],
            }]),
            cursor: None,
            background: gpui::rgb(0x0000_0000).into(),
            font_size: px(12.0),
        });
        let grid = RetainedGrid::new(&frame, cell, cx);
        Self {
            frame,
            cell,
            grid,
            font: active(Typeface::Code),
        }
    }
}
impl Render for Demo {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        self.grid
            .draw_with_font(Rc::clone(&self.frame), self.cell, self.font.clone(), cx)
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
    let cursor_before = window
        .update(cx, |demo, _, _| demo.grid.probe.cursor())
        .unwrap();
    window
        .update(cx, |demo, _, cx| {
            Rc::make_mut(&mut demo.frame).cursor = Some(GridCursor {
                row: 0,
                col: 1,
                color: gpui::rgb(0x00ff_ffff).into(),
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
    let cursor_after = window
        .update(cx, |demo, _, _| demo.grid.probe.cursor())
        .unwrap();
    assert!(cursor_after.get() > cursor_before.get());
    window
        .update(cx, |demo, _, cx| {
            Rc::make_mut(&mut demo.frame).cursor = None;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        before,
        window
            .update(cx, |demo, _, _| demo.grid.probe.counts())
            .unwrap()
    );
    assert!(
        window
            .update(cx, |demo, _, _| demo.grid.probe.cursor())
            .unwrap()
            .get()
            > cursor_after.get()
    );
}

#[derive(Debug, Clone, Copy)]
enum Change {
    Output,
    Background,
    FontSize,
    Metrics,
}
impl Change {
    fn apply(self, demo: &mut Demo) {
        match self {
            Self::Output => {
                let row = Rc::make_mut(&mut Rc::make_mut(&mut demo.frame).rows);
                row.first_mut().unwrap().runs.first_mut().unwrap().text = "new output".into();
            }
            Self::Background => {
                Rc::make_mut(&mut demo.frame).background = gpui::rgb(0x00ff_ffff).into();
            }
            Self::FontSize => Rc::make_mut(&mut demo.frame).font_size = px(14.0),
            Self::Metrics => demo.cell.width = px(9.0),
        }
    }
}

#[gpui::test]
fn row_affecting_updates_invalidate_then_quiet_updates_reuse_paint(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let window = cx.open_window(size(px(240.0), px(96.0)), |_, cx| Demo::new(cx));
    cx.run_until_parked();
    for change in [
        Change::Output,
        Change::Background,
        Change::FontSize,
        Change::Metrics,
    ] {
        let before = window
            .update(cx, |demo, _, _| demo.grid.probe.counts())
            .unwrap();
        window
            .update(cx, |demo, _, cx| {
                change.apply(demo);
                cx.notify();
            })
            .unwrap();
        cx.run_until_parked();
        let changed = window
            .update(cx, |demo, _, _| demo.grid.probe.counts())
            .unwrap();
        assert!(changed.prepared.get() > before.prepared.get(), "{change:?}");
        assert!(changed.painted.get() > before.painted.get(), "{change:?}");
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.run_until_parked();
        assert_eq!(
            changed,
            window
                .update(cx, |demo, _, _| demo.grid.probe.counts())
                .unwrap()
        );
    }
}

#[gpui::test]
fn font_stack_changes_replace_retained_paint_even_with_unchanged_metrics(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let window = cx.open_window(size(px(240.0), px(96.0)), |_, cx| Demo::new(cx));
    cx.run_until_parked();
    let (before, cell, rows) = window
        .update(cx, |demo, _, _| {
            (
                demo.grid.probe.counts(),
                demo.cell,
                Rc::clone(&demo.frame.rows),
            )
        })
        .unwrap();
    let next = FontStack::from_names(
        ["DejaVu Sans Mono".into(), "Pretendard".into()],
        Typeface::Code,
    );
    assert_ne!(next, active(Typeface::Code));
    window
        .update(cx, |demo, _, cx| {
            demo.font = next.clone();
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |demo, _, cx| {
            assert_eq!(demo.cell, cell);
            assert!(Rc::ptr_eq(&demo.frame.rows, &rows));
            assert_eq!(demo.grid.rows.read(cx).font, next);
            let after = demo.grid.probe.counts();
            assert!(after.prepared.get() > before.prepared.get());
            assert!(after.painted.get() > before.painted.get());
        })
        .unwrap();
}

#[gpui::test]
fn containing_window_resize_invalidates_the_retained_paint_bounds(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let window = cx.open_window(size(px(240.0), px(96.0)), |_, cx| Demo::new(cx));
    cx.run_until_parked();
    let before = window
        .update(cx, |demo, _, _| demo.grid.probe.counts())
        .unwrap();
    cx.simulate_window_resize(window.into(), size(px(320.0), px(128.0)));
    cx.run_until_parked();
    let after = window
        .update(cx, |demo, _, _| demo.grid.probe.counts())
        .unwrap();
    assert!(after.prepared.get() > before.prepared.get());
    assert!(after.painted.get() > before.painted.get());
}
