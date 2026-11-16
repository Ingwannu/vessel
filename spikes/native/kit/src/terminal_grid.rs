//! A terminal's grid of cells (ADR 0073, slice T1): the control draws a
//! [`GridFrame`] it is given and nothing else; it does not parse, keep a
//! model or read the app's state.
//!
//! [Decision Log]
//! - Purpose: a terminal session's screen, drawn at a terminal's speed (a
//!   200 × 50 grid repainted whole when a program redraws it).
//! - Constraint: every cell is one width; GPUI shapes text by the font's
//!   advances, and a wide scalar (Hangul, CJK, emoji) falls back to
//!   Pretendard, whose advance is not two code cells.
//! - Choice: a row is runs of cells that look the same, each shaped once
//!   with GPUI's `force_width` (every base glyph at a multiple of the cell
//!   width, so fallback glyphs cannot drift) and placed at its first
//!   cell's column; a wide scalar is a run of its own, so it starts on its
//!   cell whatever its advance. Shaped lines come from GPUI's line cache,
//!   so an unchanged row is not shaped again on the next frame.
//!   Backgrounds are cell rectangles painted under the text; the cursor is
//!   Paseo's bar (`cursorStyle: "bar"`), two points wide.
//! - Face: the configured code stack (default Geist Mono), then Pretendard (TYPE-07: a real code face before
//!   any fallback; GPUI resolves families by name, so no generic
//!   `monospace` is listed). Paseo asks for the system's developer fonts
//!   (`terminal-font.ts`); the app bundles its code face, a stated
//!   difference.
//! - Row height: the face's ascent plus descent (Paseo's `lineHeight:
//!   1.0` multiplies the cell height xterm.js measures the same way).
//! - Alternatives: one shaped line per row (wide scalars push every later
//!   cell off the grid); a text element per cell (10,000 elements a frame).

use std::rc::Rc;

use gpui::{
    App, Bounds, Font, FontFeatures, FontStyle, FontWeight, Hsla, IntoElement, Pixels, Point,
    ShapedLine, SharedString, Size, StrikethroughStyle, Styled as _, TextAlign, TextRun,
    UnderlineStyle, Window, canvas, fill, point, px, size,
};

use crate::fonts::{FontStack, Typeface, active};

mod retained;
pub use retained::RetainedGrid;
#[cfg(test)]
mod probe;

/// A text run's line under or through its glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Underline {
    /// None.
    None,
    /// A straight line (single, double, dotted and dashed are drawn so).
    Straight,
    /// A wave (curly).
    Wavy,
}

/// Whether a run is struck through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strike {
    /// No.
    None,
    /// Yes.
    Through,
}

/// How a run's glyphs are drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ink {
    /// The glyphs' colour (half its alpha for faint text).
    pub color: Hsla,
    /// The weight.
    pub weight: FontWeight,
    /// Upright or italic.
    pub style: FontStyle,
    /// The underline.
    pub underline: Underline,
    /// The strike.
    pub strike: Strike,
}

/// Cells of one row that look the same, from `col`.
#[derive(Debug, Clone, PartialEq)]
pub struct GridRun {
    /// The first cell.
    pub col: u16,
    /// The text: one scalar a cell (a wide scalar is a run of its own).
    pub text: SharedString,
    /// How it is drawn.
    pub ink: Ink,
}

/// A background over `cells` cells from `col`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFill {
    /// The first cell.
    pub col: u16,
    /// How many cells.
    pub cells: u16,
    /// The colour.
    pub color: Hsla,
}

/// One row: its backgrounds, then its text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridRow {
    /// Backgrounds other than the grid's own.
    pub fills: Vec<GridFill>,
    /// Text.
    pub runs: Vec<GridRun>,
}

/// The cursor's bar, at the left edge of its cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCursor {
    /// Row.
    pub row: u16,
    /// Column.
    pub col: u16,
    /// Colour.
    pub color: Hsla,
}

/// Everything the control draws.
#[derive(Debug, Clone, PartialEq)]
pub struct GridFrame {
    /// Immutable painted rows, shared across frames that only change the cursor.
    pub rows: Rc<[GridRow]>,
    /// The cursor, when it is drawn this frame (shown, in view, and in
    /// the visible half of its blink).
    pub cursor: Option<GridCursor>,
    /// The grid's own background.
    pub background: Hsla,
    /// The text size.
    pub font_size: Pixels,
}

/// One cell's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSize {
    /// Width: the code face's advance.
    pub width: Pixels,
    /// Height: its ascent plus descent, rounded up to a whole pixel.
    pub height: Pixels,
}

impl CellSize {
    /// How many whole cells fit in `room`: columns and rows, at least one
    /// of each.
    #[must_use]
    pub fn fit(self, room: Size<Pixels>) -> (u16, u16) {
        (
            whole_cells(f32::from(room.width), f32::from(self.width)),
            whole_cells(f32::from(room.height), f32::from(self.height)),
        )
    }
}

/// Whole cells of `cell` in `room`, 1 through `u16::MAX`.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the count is floored and clamped to 1..=65535 before the cast"
)]
fn whole_cells(room: f32, cell: f32) -> u16 {
    if cell <= 0.0 {
        return 1;
    }
    room.div_euclid(cell).clamp(1.0, f32::from(u16::MAX)) as u16
}

/// `cells` steps of `step` from `origin`.
fn at(origin: Pixels, step: Pixels, cells: u16) -> Pixels {
    px(f32::from(step).mul_add(f32::from(cells), f32::from(origin)))
}

/// The cursor bar's width.
const CURSOR_WIDTH: f32 = 2.0;

/// The code face at `weight` and `style`.
fn face(stack: &FontStack, weight: FontWeight, style: FontStyle) -> Font {
    Font {
        family: stack.family(),
        features: FontFeatures::default(),
        fallbacks: Some(stack.fallbacks()),
        weight,
        style,
    }
}

/// A cell's size at `font_size`, measured from the code face.
#[must_use]
pub fn cell_size(window: &Window, font_size: Pixels) -> CellSize {
    let text = window.text_system();
    let id = text.resolve_font(&face(
        &active(Typeface::Code),
        FontWeight::NORMAL,
        FontStyle::Normal,
    ));
    let width = text
        .advance(id, font_size, 'M')
        .map_or(font_size, |advance| advance.width);
    // GPUI gives the descent below the baseline as a negative length (its
    // Linux text system negates swash's, as Core Text's is); its size is
    // what the row needs, whatever the platform's sign.
    let ascent = f32::from(text.ascent(id, font_size));
    let height = ascent.mul_add(1.0, f32::from(text.descent(id, font_size)).abs());
    CellSize {
        width,
        height: px(height.ceil()),
    }
}

/// The control: `frame` drawn on the cell grid of `cell`, filling its
/// parent.
#[must_use]
pub fn terminal_grid(frame: Rc<GridFrame>, cell: CellSize) -> impl IntoElement {
    control(
        frame,
        cell,
        active(Typeface::Code),
        #[cfg(test)]
        None,
    )
}

fn control(
    frame: Rc<GridFrame>,
    cell: CellSize,
    font: FontStack,
    #[cfg(test)] probe: Option<probe::DrawProbe>,
) -> impl IntoElement {
    let shaped = Rc::clone(&frame);
    #[cfg(test)]
    let prepared = probe.clone();
    canvas(
        move |bounds, window, _cx| {
            #[cfg(test)]
            if let Some(probe) = &prepared {
                probe.note(probe::Stage::Prepare);
            }
            shape(&shaped, cell, &font, bounds, window)
        },
        move |bounds, lines, window, cx| {
            #[cfg(test)]
            if let Some(probe) = &probe {
                probe.note(probe::Stage::Paint);
            }
            paint(&frame, cell, bounds, &lines, window, cx);
        },
    )
    .size_full()
}

/// Every run shaped, with where it starts.
fn shape(
    frame: &GridFrame,
    cell: CellSize,
    font: &FontStack,
    bounds: Bounds<Pixels>,
    window: &Window,
) -> Vec<(Point<Pixels>, ShapedLine)> {
    let text = window.text_system();
    let mut lines = Vec::new();
    for (row, cells) in (0_u16..).zip(frame.rows.iter()) {
        let y = at(bounds.origin.y, cell.height, row);
        for run in &cells.runs {
            let x = at(bounds.origin.x, cell.width, run.col);
            let style = TextRun {
                len: run.text.len(),
                font: face(font, run.ink.weight, run.ink.style),
                color: run.ink.color,
                background_color: None,
                underline: underline(run.ink),
                strikethrough: strike(run.ink),
            };
            let line = text.shape_line(
                run.text.clone(),
                frame.font_size,
                &[style],
                Some(cell.width),
            );
            lines.push((point(x, y), line));
        }
    }
    lines
}

fn underline(ink: Ink) -> Option<UnderlineStyle> {
    let wavy = match ink.underline {
        Underline::None => return None,
        Underline::Straight => false,
        Underline::Wavy => true,
    };
    Some(UnderlineStyle {
        thickness: px(1.0),
        color: Some(ink.color),
        wavy,
    })
}

fn strike(ink: Ink) -> Option<StrikethroughStyle> {
    match ink.strike {
        Strike::None => None,
        Strike::Through => Some(StrikethroughStyle {
            thickness: px(1.0),
            color: Some(ink.color),
        }),
    }
}

/// The grid's background, the rows' backgrounds, the text, the cursor.
fn paint(
    frame: &GridFrame,
    cell: CellSize,
    bounds: Bounds<Pixels>,
    lines: &[(Point<Pixels>, ShapedLine)],
    window: &mut Window,
    cx: &mut App,
) {
    window.paint_quad(fill(bounds, frame.background));
    for (row, cells) in (0_u16..).zip(frame.rows.iter()) {
        let y = at(bounds.origin.y, cell.height, row);
        for run in &cells.fills {
            let x = at(bounds.origin.x, cell.width, run.col);
            let width = at(px(0.0), cell.width, run.cells);
            window.paint_quad(fill(
                Bounds::new(point(x, y), size(width, cell.height)),
                run.color,
            ));
        }
    }
    for (origin, line) in lines {
        // A line that cannot paint (its glyphs failed to rasterize) is
        // left out; the rest of the grid still draws.
        let _painted = line.paint(*origin, cell.height, TextAlign::Left, None, window, cx);
    }
    paint_cursor(frame.cursor, cell, bounds, window);
}

fn paint_cursor(
    cursor: Option<GridCursor>,
    cell: CellSize,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    if let Some(cursor) = cursor {
        let x = at(bounds.origin.x, cell.width, cursor.col);
        let y = at(bounds.origin.y, cell.height, cursor.row);
        window.paint_quad(fill(
            Bounds::new(point(x, y), size(px(CURSOR_WIDTH), cell.height)),
            cursor.color,
        ));
    }
}
