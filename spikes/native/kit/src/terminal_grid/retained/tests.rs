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

#[derive(Clone, Copy)]
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
                Rc::make_mut(&mut demo.frame).background = gpui::rgb(0xffffff).into()
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
        assert!(changed.prepared.get() > before.prepared.get());
        assert!(changed.painted.get() > before.painted.get());
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

struct RestoreFonts {
    cx: TestAppContext,
    interface: FontStack,
    code: FontStack,
}
impl Drop for RestoreFonts {
    fn drop(&mut self) {
        self.cx
            .update(|cx| crate::set_font_stacks(cx, self.interface.clone(), self.code.clone()));
    }
}

#[gpui::test]
fn font_stack_changes_replace_retained_paint_even_with_unchanged_metrics(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let _restore = RestoreFonts {
        cx: cx.clone(),
        interface: active(Typeface::Interface),
        code: active(Typeface::Code),
    };
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
    cx.update(|cx| crate::set_font_stacks(cx, active(Typeface::Interface), next.clone()));
    window.update(cx, |_, _, cx| cx.notify()).unwrap();
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
