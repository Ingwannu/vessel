//! SGR on the pen, and the pen on cells: erase in the background, DECSC
//! keeping it, scrolled-in lines, resizes and history.

use super::{history, model, play};
use crate::terminal::GridSize;
use crate::terminal::TerminalModel;
use crate::terminal::grid::{
    Cell, Colour, Concealment, Glyph, Pen, Polarity, Rgb, Scroll, Slant, Strike, Underline, Weight,
};

/// The pen after `bytes` on a fresh grid.
fn pen_after(bytes: &[u8]) -> Pen {
    let mut term = model(4, 2, 0);
    play(&mut term, bytes);
    play(&mut term, b"x");
    cell(&term, 0, 0).pen()
}

fn cell(term: &TerminalModel, row: usize, col: usize) -> Cell {
    let cols = usize::from(term.grid().size().cols());
    let at = row.saturating_mul(cols).saturating_add(col);
    term.snapshot(Scroll(0))
        .cells
        .get(at)
        .copied()
        .unwrap_or(Cell::BLANK)
}

const fn rgb(r: u8, g: u8, b: u8) -> Colour {
    Colour::Rgb(Rgb { r, g, b })
}

#[test]
fn each_attribute_is_set_and_reset_by_its_own_code() {
    let all = pen_after(b"\x1b[1;3;4;7;8;9m");
    assert_eq!(
        all,
        Pen {
            weight: Weight::Bold,
            slant: Slant::Italic,
            underline: Underline::Single,
            polarity: Polarity::Inverse,
            concealment: Concealment::Concealed,
            strike: Strike::Through,
            ..Pen::PLAIN
        }
    );
    assert_eq!(
        pen_after(b"\x1b[1;3;4;7;8;9;22;23;24;27;28;29m"),
        Pen::PLAIN
    );
    assert_eq!(pen_after(b"\x1b[1;31;44;0m"), Pen::PLAIN);
    assert_eq!(pen_after(b"\x1b[2m").weight, Weight::Faint);
    assert_eq!(pen_after(b"\x1b[21m").underline, Underline::Double);
    assert_eq!(pen_after(b"\x1b[4:3m").underline, Underline::Curly);
    assert_eq!(pen_after(b"\x1b[4:0m").underline, Underline::None);
    assert_eq!(pen_after(b"\x1b[4:9m").underline, Underline::Single);
}

#[test]
fn colours_in_every_form_and_malformed_ones_change_nothing() {
    assert_eq!(pen_after(b"\x1b[31;42m").fg, Colour::Indexed(1));
    assert_eq!(pen_after(b"\x1b[31;42m").bg, Colour::Indexed(2));
    assert_eq!(pen_after(b"\x1b[97;107m").fg, Colour::Indexed(15));
    assert_eq!(pen_after(b"\x1b[97;107m").bg, Colour::Indexed(15));
    assert_eq!(pen_after(b"\x1b[31;39m").fg, Colour::Default);
    assert_eq!(pen_after(b"\x1b[38;5;196m").fg, Colour::Indexed(196));
    assert_eq!(pen_after(b"\x1b[48;2;1;2;3m").bg, rgb(1, 2, 3));
    assert_eq!(pen_after(b"\x1b[38:2:1:2:3m").fg, rgb(1, 2, 3));
    assert_eq!(pen_after(b"\x1b[38:2::1:2:3m").fg, rgb(1, 2, 3));
    assert_eq!(pen_after(b"\x1b[38:5:300m").fg, Colour::Default);
    // A component past 255 drops the colour, and its values are not read
    // as codes of their own: the 1 after them is bold.
    let pen = pen_after(b"\x1b[38;2;300;7;9;1m");
    assert_eq!((pen.fg, pen.weight), (Colour::Default, Weight::Bold));
    // The underline's colour is read past and dropped.
    let pen = pen_after(b"\x1b[58;2;1;2;3;4m");
    assert_eq!(
        (pen.fg, pen.underline),
        (Colour::Default, Underline::Single)
    );
    for cut in [
        &b"\x1b[38m"[..],
        b"\x1b[38;5m",
        b"\x1b[38;2;1m",
        b"\x1b[48:2m",
        b"\x1b[38;9m",
    ] {
        assert_eq!(pen_after(cut), Pen::PLAIN);
    }
}

#[test]
fn an_erase_writes_the_background_and_nothing_else() {
    let mut term = model(4, 2, 0);
    play(&mut term, b"\x1b[41;1;4mX\x1b[K");
    let erased = cell(&term, 0, 2);
    assert_eq!(erased.glyph(), Glyph::Empty);
    assert_eq!(
        erased.pen(),
        Pen {
            bg: Colour::Indexed(1),
            ..Pen::PLAIN
        }
    );
    // A line scrolled in at the bottom takes the background too (xterm.js).
    play(&mut term, b"\x1b[44m\n\n");
    assert_eq!(cell(&term, 1, 3).pen().bg, Colour::Indexed(4));
}

#[test]
fn decsc_keeps_the_pen_and_decrc_with_nothing_kept_is_plain() {
    let mut term = model(4, 2, 0);
    play(&mut term, b"\x1b[31m\x1b7\x1b[0m\x1b8X");
    assert_eq!(cell(&term, 0, 0).pen().fg, Colour::Indexed(1));
    play(&mut term, b"\x1b[?47h\x1b[32m\x1b8Y");
    assert_eq!(cell(&term, 0, 0).pen(), Pen::PLAIN);
}

#[test]
fn a_resize_pads_with_plain_cells_whatever_the_pen() {
    let mut term = model(2, 1, 0);
    play(&mut term, b"\x1b[41m");
    if let Ok(size) = GridSize::new(4, 2) {
        term.resize(size);
    }
    assert_eq!(cell(&term, 0, 3), Cell::BLANK);
    assert_eq!(cell(&term, 1, 0), Cell::BLANK);
}

#[test]
fn history_drops_plain_trailing_blanks_but_keeps_coloured_ones() {
    let mut term = model(6, 1, 10);
    play(&mut term, b"ab\r\n\x1b[42mcd\x1b[K\r\n");
    let grid = term.grid();
    let widths: Vec<usize> = (0..grid.history_len())
        .filter_map(|index| grid.history_line(index))
        .map(|line| line.cells().len())
        .collect();
    assert_eq!(widths, [2, 6]);
    assert_eq!(history(&term), ["ab", "cd"]);
}
